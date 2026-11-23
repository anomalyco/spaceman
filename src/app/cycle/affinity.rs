use super::*;
use crate::journal::PlacementSource;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FrozenWindowAffinity {
    Owned(WorkspaceId),
    PendingReplacement(WorkspaceId),
    AmbiguousReplacement,
    Floating,
    ExplicitRoute(WorkspaceId),
    /// The app's existing workspace, or a remembered placement a relaunched
    /// application's window claimed.
    Inferred(WorkspaceId),
    CurrentDisplay,
    /// Never tiled, owned, or parked (ignore rule or parking exemption).
    Ignored,
}

pub(super) struct FrozenAffinityPolicy {
    pub(super) windows: HashMap<crate::PhysicalWindowId, FrozenWindowAffinity>,
    pub(super) visible_workspaces: HashSet<WorkspaceId>,
}

impl<B: WorkspaceBackend> WorkspaceRuntime<B> {
    pub(super) fn freeze_window_affinities(
        &mut self,
        observation: &crate::NativeObservation,
        visible: &[ResolvedWorkspace],
        server_frames: &HashMap<u32, crate::Rect>,
    ) -> Result<FrozenAffinityPolicy, String> {
        let has_floating = self.workspaces.floating_entries().next().is_some();
        if has_floating {
            let floating_apps = self
                .workspaces
                .floating_entries()
                .map(|(window, _)| window.app.clone())
                .collect::<HashSet<_>>();
            for app in floating_apps {
                if !self.backend.is_process_running(&app) {
                    for window in self.workspaces.floating_windows_for_app(&app) {
                        log_event!("forget floating {:?}: app exited", window);
                        self.workspaces.forget_floating(&window);
                    }
                }
            }
        }
        self.forget_exited_apps_on_hidden_workspaces(visible);

        self.close_recalls();
        // Before any new instance below is considered: a relaunch's new
        // process can appear in the same observation that reports the old
        // one terminated.
        self.note_absent_departures(observation);
        let mut affinities = HashMap::new();
        for application in &observation.applications {
            let crate::AppObservation::Complete { app, windows, .. } = application else {
                continue;
            };
            if has_floating {
                let floating_windows = self.workspaces.floating_windows_for_app(&app.id);
                for window in floating_windows {
                    if !windows.iter().any(|observed| observed.id == window)
                        && !self.parked.contains_key(&window)
                    {
                        log_event!("forget floating {:?}: not observed and not parked", window);
                        self.workspaces.forget_floating(&window);
                    }
                }
            }
            self.prepare_recall(app, windows);
            let mut app_owns_window = self.workspaces.app_owns_windows(&app.id);
            for window in windows {
                self.heal_reappeared_window(window, visible, server_frames);
                let disposition = self.config.disposition(&app.bundle_id, window);
                if disposition == WindowDisposition::Ignored {
                    if let Some(owner) = self.workspaces.workspace_of_any(&window.id) {
                        log_event!("release {:?} from workspace {}: ignored", window.id, owner);
                        self.forget_window(owner, &window.id);
                    }
                    affinities.insert(window.id.clone(), FrozenWindowAffinity::Ignored);
                    continue;
                }
                // A window the user floated by hand stays floating even though no rule
                // names it; Toggle Float is the rule, for this window and this session.
                let floating = !window.resizable
                    || disposition == WindowDisposition::Floating
                    || self.manual_floating.contains(&window.id);
                if floating {
                    if let Some(owner) = self.workspaces.workspace_of(&window.id) {
                        self.forget_window(owner, &window.id);
                        self.workspaces.own_floating(window.id.clone(), owner);
                    }
                } else if self.workspaces.floating_workspace_of(&window.id).is_some() {
                    self.workspaces.forget_floating(&window.id);
                }
                let affinity = if let Some(owner) = self.workspaces.workspace_of(&window.id) {
                    FrozenWindowAffinity::Owned(owner)
                } else if !floating
                    && let Some(affinity) = {
                        let mut reserved = self.workspaces.replacement_workspaces(&window.id);
                        reserved.next().map(|workspace| {
                            if reserved.next().is_some() {
                                FrozenWindowAffinity::AmbiguousReplacement
                            } else {
                                FrozenWindowAffinity::PendingReplacement(workspace)
                            }
                        })
                    }
                {
                    affinity
                } else if floating && self.workspaces.floating_workspace_of(&window.id).is_some() {
                    FrozenWindowAffinity::Floating
                } else if floating && let Some(placement) = self.memory.recalled(&window.id) {
                    app_owns_window = true;
                    self.workspaces
                        .own_floating(window.id.clone(), placement.workspace);
                    self.note_placement(&window.id, PlacementSource::Recall, placement.workspace);
                    FrozenWindowAffinity::Floating
                } else if floating {
                    visible
                        .iter()
                        .find(|resolved| resolved.output.contains_center(window.frame))
                        .map_or(FrozenWindowAffinity::CurrentDisplay, |resolved| {
                            app_owns_window = true;
                            log_event!(
                                "adopt floating {:?} -> workspace {}",
                                window.id,
                                resolved.workspace
                            );
                            self.workspaces
                                .own_floating(window.id.clone(), resolved.workspace);
                            self.note_placement(
                                &window.id,
                                PlacementSource::Active,
                                resolved.workspace,
                            );
                            FrozenWindowAffinity::Floating
                        })
                } else {
                    match disposition {
                        WindowDisposition::Route(target) => {
                            if !self
                                .scope
                                .manages_selector(self.workspaces.assignment(target))
                            {
                                return Err(format!(
                                    "window rule routes to unmanaged workspace {}",
                                    target
                                ));
                            }
                            if !app_owns_window
                                || self.config.route_matches_title(&app.bundle_id, window)
                            {
                                app_owns_window = true;
                                FrozenWindowAffinity::ExplicitRoute(target)
                            } else {
                                FrozenWindowAffinity::CurrentDisplay
                            }
                        }
                        WindowDisposition::Managed => {
                            app_owns_window = true;
                            self.memory
                                .recalled(&window.id)
                                .map(|placement| {
                                    FrozenWindowAffinity::Inferred(placement.workspace)
                                })
                                .unwrap_or(FrozenWindowAffinity::CurrentDisplay)
                        }
                        WindowDisposition::Floating | WindowDisposition::Ignored => unreachable!(),
                    }
                };
                affinities.insert(window.id.clone(), affinity);
            }
        }
        Ok(FrozenAffinityPolicy {
            windows: affinities,
            visible_workspaces: visible.iter().map(|resolved| resolved.workspace).collect(),
        })
    }

    /// Hidden workspaces are not reconciled, so an app that quit while its
    /// windows were parked would otherwise keep their tiles until the
    /// workspace was shown again.
    fn forget_exited_apps_on_hidden_workspaces(&mut self, visible: &[ResolvedWorkspace]) {
        let mut running = HashMap::new();
        for workspace in WorkspaceId::all() {
            if visible
                .iter()
                .any(|resolved| resolved.workspace == workspace)
            {
                continue;
            }
            let apps = self
                .workspaces
                .app_instances(workspace)
                .cloned()
                .collect::<HashSet<_>>();
            for app in apps {
                let alive = *running
                    .entry(app.clone())
                    .or_insert_with(|| self.backend.is_process_running(&app));
                if alive {
                    continue;
                }
                let windows = self
                    .workspaces
                    .engine(workspace)
                    .model()
                    .workspace
                    .slots
                    .iter()
                    .filter(|slot| slot.belongs_to(&app))
                    .map(|slot| slot.identity_window().clone())
                    .collect::<Vec<_>>();
                self.note_departure(&app);
                self.workspaces.forget_app(workspace, &app);
                for window in &windows {
                    self.forget_window(workspace, window);
                }
                self.known_apps[workspace.index()].remove(&app);
                log_event!(
                    "workspace {workspace}: forgot {app:?}: app exited while hidden ({} tile(s))",
                    windows.len()
                );
                let parked = windows
                    .into_iter()
                    .filter(|window| self.parked.contains_key(window))
                    .collect();
                self.restore_windows(parked);
            }
        }
    }
}
