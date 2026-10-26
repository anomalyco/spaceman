use super::*;

/// How long a bound window may be missing from its app's Accessibility snapshot
/// while the window server still shows it on screen before the omission is
/// believed. Electron apps rebuild their accessibility tree under load (Slack
/// starting a huddle) and briefly report a partial window list; one such
/// snapshot must not retire a slot.
pub(super) const OMISSION_SETTLE: Duration = Duration::from_secs(2);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FrozenWindowAffinity {
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

struct FrozenAffinityPolicy {
    windows: HashMap<crate::PhysicalWindowId, FrozenWindowAffinity>,
    visible_workspaces: HashSet<WorkspaceId>,
}

struct ReconcileDisplayContext<'a> {
    outputs: &'a macos::DisplayOutputs,
    observation: &'a mut crate::NativeObservation,
    affinity_policy: &'a FrozenAffinityPolicy,
    server_frames: Option<&'a HashMap<u32, crate::Rect>>,
    onscreen_frames: Option<&'a HashMap<u32, crate::Rect>>,
    cancelled: &'a AtomicBool,
    failed: &'a mut usize,
}

impl<B: WorkspaceBackend> WorkspaceRuntime<B> {
    fn restore_visible(
        &mut self,
        visible: &[ResolvedWorkspace],
        cancelled: &AtomicBool,
        failed: &mut usize,
    ) {
        if self.parked.is_empty() {
            return;
        }
        let windows: Vec<_> = visible
            .iter()
            .flat_map(|resolved| self.workspaces.windows(resolved.workspace))
            .collect();
        for window in windows {
            if cancelled.load(Ordering::Acquire) {
                return;
            }
            if !self.parked.contains_key(&window) {
                continue;
            }
            // Throttle retries of restores awaiting window-server confirmation
            // so a slow app is not written to and logged about every cycle.
            if self
                .recently_parked
                .get(&window)
                .is_some_and(|attempted_at| attempted_at.elapsed() < RESTORE_RETRY_INTERVAL)
            {
                continue;
            }
            if !matches!(
                self.restore_window(&window),
                macos::ApplyOutcome::Applied | macos::ApplyOutcome::Gone
            ) {
                *failed += 1;
            }
        }
    }

    pub(super) fn cycle(
        &mut self,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        let mut failed = 0;
        if cancelled.load(Ordering::Acquire) {
            return Err("workspace operation cancelled".into());
        }
        // Parking is durable before workspace ownership. A crash between those
        // writes must not strand an unowned window behind an otherwise valid state.
        let unowned = self
            .parked
            .keys()
            .filter(|window| self.workspaces.workspace_of_any(window).is_none())
            .cloned()
            .collect::<Vec<_>>();
        let pending = self.restore_windows(unowned);
        if pending > 0 {
            return Err(format!(
                "Recovery pending for {pending} unowned parked window(s)"
            ));
        }
        self.synchronize_foreground(&outputs);
        let visible = self.visible_workspaces(&outputs);
        let frontmost = self.backend.frontmost_application();
        let focused = self.backend.focused_window();
        let focused_is_frontmost = focused.as_ref().is_some_and(|focused| {
            frontmost
                .as_ref()
                .is_none_or(|frontmost| &focused.app == frontmost)
        });
        let focus_workspace = if focused_is_frontmost {
            focused
                .as_ref()
                .and_then(|focused| self.workspaces.workspace_of_any(focused))
        } else {
            frontmost
                .as_ref()
                .and_then(|app| self.workspaces.workspace_for_app(app))
        };
        let suppressed = focused_is_frontmost
            && focused
                .as_ref()
                .is_some_and(|focused| self.suppressed_focus.as_ref() == Some(focused));
        if !suppressed {
            self.suppressed_focus = None;
            if let Some(workspace) = focus_workspace
                && !visible
                    .iter()
                    .any(|resolved| resolved.workspace == workspace)
            {
                let input = self
                    .backend
                    .recent_input()
                    .map_or_else(|| "input unknown".into(), |input| input.to_string());
                log_event!(
                    "focus follow: {:?} (frontmost {:?}) -> workspace {} ({})",
                    focused,
                    frontmost,
                    workspace,
                    input
                );
                return self.switch(workspace, outputs, cancelled);
            }
        }
        for resolved in &visible {
            self.foreground_lanes
                .insert(resolved.display_id.clone(), resolved.selector.clone());
        }
        // One window-server snapshot serves every authoritative-frame check in
        // this cycle; per-window queries would each copy the full window list.
        let server_snapshot = self.backend.window_server_frames();
        let empty_server_frames = HashMap::new();
        let server_frames = server_snapshot.as_ref().unwrap_or(&empty_server_frames);
        self.recover_frontmost_stale_parked_window(
            &outputs,
            frontmost.as_ref(),
            focused.as_ref(),
            server_frames,
        );
        self.restore_visible(&visible, cancelled, &mut failed);
        self.park_inactive(&outputs, &visible, server_frames, cancelled, &mut failed);
        if cancelled.load(Ordering::Acquire) {
            return Err("workspace operation cancelled".into());
        }
        let mut observation = self
            .backend
            .snapshot(0, &outputs)
            .map_err(|error| format!("Observation error: {error}"))?;
        let affinity_policy =
            self.freeze_window_affinities(&observation, &visible, server_frames)?;
        // Taken after the Accessibility snapshot so a window listed here was
        // on screen no earlier than the snapshot that may have omitted it.
        let onscreen_snapshot = self.backend.onscreen_window_frames();
        for resolved in &visible {
            if !self.scope.manages_selector(&resolved.selector) {
                continue;
            }
            self.reconcile_display(
                resolved,
                ReconcileDisplayContext {
                    outputs: &outputs,
                    observation: &mut observation,
                    affinity_policy: &affinity_policy,
                    server_frames: server_snapshot.as_ref(),
                    onscreen_frames: onscreen_snapshot.as_ref(),
                    cancelled,
                    failed: &mut failed,
                },
            )?;
        }
        let mut attempted_focus_restore = HashSet::new();
        for resolved in &visible {
            if !self
                .pending_fullscreen_focus_restore
                .contains(&resolved.workspace)
            {
                continue;
            }
            attempted_focus_restore.insert(resolved.workspace);
            let Some(window) = self.workspaces.preferred_focus(resolved.workspace) else {
                self.pending_fullscreen_focus_restore
                    .remove(&resolved.workspace);
                continue;
            };
            let restored = if focus_workspace == Some(resolved.workspace) {
                self.backend.focus_window(&window)
            } else {
                self.backend.raise_window(&window)
            };
            self.pending_fullscreen_focus_restore
                .remove(&resolved.workspace);
            if !restored {
                failed += 1;
            }
        }
        if focused_is_frontmost
            && let Some(focused) = focused.as_ref()
            && let Some(workspace) = self.workspaces.workspace_of(focused)
            && self.workspaces.uses_fullscreen_layout(workspace)
            && !attempted_focus_restore.contains(&workspace)
            && visible
                .iter()
                .any(|resolved| resolved.workspace == workspace)
        {
            self.workspaces.remember_focus(workspace, focused);
        }
        self.record_window_memory(server_frames);
        self.journal.observation = Some(observation);
        self.persist_workspace_state_if_changed()?;
        Ok(self.status_for_visible(failed, &visible))
    }

    fn freeze_window_affinities(
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
                let floating = !window.resizable || disposition == WindowDisposition::Floating;
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
                    .bound_windows()
                    .into_iter()
                    .filter(|window| window.app == app)
                    .collect::<Vec<_>>();
                for window in &windows {
                    self.forget_window(workspace, window);
                }
                self.workspaces.forget_app(workspace, &app);
                self.known_apps[workspace.index()].remove(&app);
                log_event!(
                    "workspace {workspace}: forgot {app:?}: app exited while hidden ({} tile(s))",
                    windows.len()
                );
            }
        }
    }

    /// A bound window the window server still shows on screen inside this
    /// output, but which the app's Accessibility snapshot omitted, has not
    /// gone anywhere: the snapshot is incomplete for that app. Downgrade the
    /// app to `Unavailable` so reconciliation keeps its slots instead of
    /// retiring one and re-adopting the window as new a cycle later. The
    /// omission is believed once it outlasts [`OMISSION_SETTLE`].
    fn guard_omitted_windows(
        &mut self,
        workspace: WorkspaceId,
        output: crate::Rect,
        observation: &mut crate::NativeObservation,
        onscreen_frames: Option<&HashMap<u32, crate::Rect>>,
    ) {
        let Some(onscreen_frames) = onscreen_frames else {
            return;
        };
        let bound = self.workspaces.engine(workspace).bound_windows();
        if bound.is_empty() && self.omitted_windows.is_empty() {
            return;
        }
        let observed: HashSet<_> = live::observed_frames(observation).into_keys().collect();
        self.omitted_windows
            .retain(|window, _| match self.workspaces.workspace_of(window) {
                Some(owner) => owner != workspace || !observed.contains(window),
                None => false,
            });
        let now = Instant::now();
        for app_observation in &mut observation.applications {
            let crate::AppObservation::Complete { app, .. } = app_observation else {
                continue;
            };
            let settling = bound
                .iter()
                .filter(|window| window.app == app.id && !observed.contains(*window))
                .filter(|window| {
                    onscreen_frames
                        .get(&window.window_id)
                        .is_some_and(|frame| output.contains_center(*frame))
                })
                .filter(|window| {
                    let since = *self
                        .omitted_windows
                        .entry((*window).clone())
                        .or_insert_with(|| {
                            log_event!(
                                "observation omitted {:?}: window server shows it on screen; keeping its slot",
                                window
                            );
                            now
                        });
                    now.duration_since(since) < OMISSION_SETTLE
                })
                .count();
            if settling == 0 {
                continue;
            }
            *app_observation = crate::AppObservation::Unavailable {
                app: app.id.clone(),
                error: format!(
                    "accessibility snapshot omitted {settling} on-screen bound window(s)"
                ),
            };
        }
    }

    /// Apps the snapshot omitted entirely keep their slots while the window
    /// server still lists one of their bound windows. The window server also
    /// lists windows that are ordered out (a closed window its app keeps
    /// alive, a minimized window), and keeping those slots leaves an empty
    /// tile. So while another window of this workspace is observed, meaning
    /// the display is not showing another Space, an ordered-out window keeps
    /// its app's slots only until the omission outlasts [`OMISSION_SETTLE`].
    /// Unresolved replacement candidates keep the plain window-server rule.
    fn apps_with_window_server_evidence(
        &mut self,
        workspace: WorkspaceId,
        observation: &crate::NativeObservation,
        server_frames: Option<&HashMap<u32, crate::Rect>>,
        onscreen_frames: Option<&HashMap<u32, crate::Rect>>,
    ) -> HashSet<AppInstanceId> {
        let mut apps = HashSet::new();
        let Some(server_frames) = server_frames else {
            return apps;
        };
        let present: HashSet<_> = observation
            .applications
            .iter()
            .map(|app| app.app_id().clone())
            .collect();
        let workspace_observed = live::observed_frames(observation)
            .keys()
            .any(|window| self.workspaces.workspace_of(window) == Some(workspace));
        let onscreen_frames = onscreen_frames.filter(|_| workspace_observed);
        let windows: Vec<_> = self
            .workspaces
            .engine(workspace)
            .model()
            .workspace
            .slots
            .iter()
            .flat_map(|slot| match &slot.binding {
                BindingState::Bound(window) => vec![(window.clone(), true)],
                BindingState::Unresolved {
                    previous,
                    candidates,
                    ..
                } => std::iter::once((previous.clone(), true))
                    .chain(candidates.iter().map(|window| (window.clone(), false)))
                    .collect(),
            })
            .filter(|(window, _)| {
                !present.contains(&window.app) && server_frames.contains_key(&window.window_id)
            })
            .collect();
        let now = Instant::now();
        for (window, settles) in windows {
            let Some(onscreen_frames) = onscreen_frames.filter(|_| settles) else {
                // Off-screen time spent on another Space must not count
                // toward the settle once the display shows this workspace.
                self.omitted_windows.remove(&window);
                apps.insert(window.app);
                continue;
            };
            if onscreen_frames.contains_key(&window.window_id) {
                self.omitted_windows.remove(&window);
                apps.insert(window.app);
                continue;
            }
            let since = *self
                .omitted_windows
                .entry(window.clone())
                .or_insert_with(|| {
                    log_event!(
                        "observation omitted {:?}: window server lists it off screen; releasing its slot unless it returns",
                        window
                    );
                    now
                });
            if now.duration_since(since) < OMISSION_SETTLE {
                apps.insert(window.app);
            }
        }
        apps
    }

    /// Bookkeeping shared by every path that moves a window between workspaces.
    pub(super) fn clear_relocation_evidence(
        &mut self,
        source: WorkspaceId,
        target: WorkspaceId,
        window: &crate::PhysicalWindowId,
    ) {
        self.constrained[source.index()].remove(window);
        self.constrained[target.index()].remove(window);
        self.applied[source.index()].remove(window);
        self.applied[target.index()].remove(window);
    }

    fn reconcile_display(
        &mut self,
        resolved: &ResolvedWorkspace,
        context: ReconcileDisplayContext<'_>,
    ) -> Result<(), String> {
        let ReconcileDisplayContext {
            outputs,
            observation,
            affinity_policy,
            server_frames,
            onscreen_frames,
            cancelled,
            failed,
        } = context;
        let workspace = resolved.workspace;
        let output = resolved.output;
        let ticket = {
            let engine = self.workspaces.engine_mut(workspace);
            engine.invalidate();
            engine.begin_cycle()
        };
        let mut scoped_observation = observation.clone();
        scoped_observation.cycle = ticket.cycle;
        let settings = Settings::for_output(output);
        for app in &mut scoped_observation.applications {
            if let crate::AppObservation::Complete {
                app: _,
                focused,
                windows,
            } = app
            {
                windows.retain(|window| {
                    let affinity = affinity_policy
                        .windows
                        .get(&window.id)
                        .copied()
                        .unwrap_or(FrozenWindowAffinity::CurrentDisplay);
                    match affinity {
                        FrozenWindowAffinity::Owned(owner)
                        | FrozenWindowAffinity::PendingReplacement(owner) => owner == workspace,
                        FrozenWindowAffinity::AmbiguousReplacement
                        | FrozenWindowAffinity::Floating
                        | FrozenWindowAffinity::Ignored => false,
                        FrozenWindowAffinity::Inferred(target)
                            if affinity_policy.visible_workspaces.contains(&target) =>
                        {
                            target == workspace
                        }
                        FrozenWindowAffinity::ExplicitRoute(_)
                        | FrozenWindowAffinity::Inferred(_)
                        | FrozenWindowAffinity::CurrentDisplay => {
                            output.contains_center(window.frame)
                                && self
                                    .workspaces
                                    .workspace_of(&window.id)
                                    .is_none_or(|owner| owner == workspace)
                        }
                    }
                });
                if focused
                    .as_ref()
                    .is_some_and(|focused| !windows.iter().any(|window| &window.id == focused))
                {
                    *focused = None;
                }
            }
        }
        self.guard_omitted_windows(workspace, output, &mut scoped_observation, onscreen_frames);
        let apps_with_window_server_evidence = self.apps_with_window_server_evidence(
            workspace,
            &scoped_observation,
            server_frames,
            onscreen_frames,
        );
        live::add_absent_applications_with(
            &mut scoped_observation,
            &mut self.known_apps[workspace.index()],
            |app| self.backend.is_process_running(app),
            |app| server_frames.is_none() || apps_with_window_server_evidence.contains(app),
        );
        let reconciliation = self
            .workspaces
            .engine_mut(workspace)
            .accept(ticket, &scoped_observation, &settings)
            .map_err(|error| format!("stale workspace cycle: {error:?}"))?;
        self.log_binding_decisions(workspace, &reconciliation.decisions, affinity_policy);
        let routes: Vec<_> = reconciliation
            .decisions
            .iter()
            .filter_map(|decision| match decision {
                Decision::WindowInserted { window, .. } => affinity_policy
                    .windows
                    .get(window)
                    .copied()
                    .and_then(|affinity| match affinity {
                        FrozenWindowAffinity::ExplicitRoute(target)
                        | FrozenWindowAffinity::Inferred(target)
                            if target != workspace =>
                        {
                            Some((window.clone(), target))
                        }
                        _ => None,
                    }),
                _ => None,
            })
            .collect();
        let mut routed_targets = HashSet::new();
        for (window, target) in &routes {
            if !affinity_policy.visible_workspaces.contains(target) {
                let target_output = output_for(&self.workspaces, *target, outputs);
                match self.park_window(window, target_output, outputs) {
                    macos::ApplyOutcome::Applied => {}
                    macos::ApplyOutcome::Gone => {
                        self.forget_window(workspace, window);
                        continue;
                    }
                    _ => {
                        *failed += 1;
                        continue;
                    }
                }
            }
            if self.workspaces.move_window(window, *target).is_none() {
                continue;
            }
            routed_targets.insert(*target);
        }
        let inserted_here = reconciliation.decisions.iter().any(|decision| {
            matches!(
                decision,
                Decision::WindowInserted { window, .. }
                    if !routes.iter().any(|(routed, _)| routed == window)
            )
        });
        let bound: HashSet<_> = self
            .workspaces
            .engine(workspace)
            .bound_windows()
            .into_iter()
            .collect();
        if inserted_here {
            self.exit_fullscreen_if_split(workspace);
        }
        for target in &routed_targets {
            self.exit_fullscreen_if_split(*target);
        }
        self.workspaces.prune_intents(workspace);
        self.clear_fullscreen_restore_if_inactive(workspace);
        for target in &routed_targets {
            self.clear_fullscreen_restore_if_inactive(*target);
        }
        self.constrained[workspace.index()].retain(|window, _| bound.contains(window));
        self.applied[workspace.index()].retain(|window, _| bound.contains(window));
        let observed_frames = live::observed_frames(&scoped_observation);
        let filter_unchanged = routes.is_empty();
        let mut placements = if filter_unchanged {
            reconciliation.placements
        } else {
            self.workspaces.engine(workspace).placements(&settings)
        };
        self.apply_fullscreen_layout(workspace, &mut placements, output);
        if !scoped_observation.mouse_down {
            for placement in &placements {
                let Some(window) = placement.window.as_ref() else {
                    continue;
                };
                let Some(&observed) = observed_frames.get(window) else {
                    continue;
                };
                let target = placement.frame;
                if observed.approximately_equals(target, settings.frame_tolerance) {
                    self.applied[workspace.index()].remove(window);
                    self.constrained[workspace.index()].remove(window);
                    continue;
                }
                if self.constrained[workspace.index()]
                    .get(window)
                    .is_some_and(|(constrained_target, _)| *constrained_target != target)
                {
                    self.constrained[workspace.index()].remove(window);
                }
                if self.applied[workspace.index()].remove(window) == Some(target) {
                    self.constrained[workspace.index()].insert(window.clone(), (target, observed));
                }
            }
        }
        if filter_unchanged {
            placements.retain(|placement| {
                live::needs_application(placement, &observed_frames, settings.frame_tolerance)
            });
        }
        placements.retain(|placement| {
            let Some(window) = placement.window.as_ref() else {
                return false;
            };
            !self.constrained[workspace.index()]
                .get(window)
                .zip(observed_frames.get(window))
                .is_some_and(|((target, accepted), observed)| {
                    *target == placement.frame
                        && observed.approximately_equals(*accepted, settings.frame_tolerance)
                })
        });
        if !placements.is_empty() {
            let results = self
                .backend
                .apply_observed(&placements, &settings, cancelled);
            self.record_results(workspace, &results);
            *failed += failed_results(&results);
            if self.fit_minimum_sizes(workspace, output, &results) {
                *failed += self
                    .apply_layout(workspace, output, None, cancelled)
                    .0
                    .failed;
            }
        }
        for target in routed_targets {
            if affinity_policy.visible_workspaces.contains(&target) {
                let target_output = output_for(&self.workspaces, target, outputs);
                let layout = self.layout_workspace(target, target_output, cancelled);
                *failed += layout.failed;
                update_observed_frames(
                    observation,
                    &self
                        .workspaces
                        .engine(target)
                        .placements(&Settings::for_output(target_output)),
                );
            }
        }
        Ok(())
    }
    /// Slot-identity changes are the events that explain "my window moved"; the
    /// steady-state decisions (unavailable, deferred) would only add noise.
    fn log_binding_decisions(
        &self,
        workspace: WorkspaceId,
        decisions: &[Decision],
        affinity_policy: &FrozenAffinityPolicy,
    ) {
        for decision in decisions {
            match decision {
                Decision::WindowInserted { slot, window } => {
                    let bundle = self.bundle_id(&window.app).unwrap_or("unknown");
                    let affinity = affinity_policy.windows.get(window);
                    log_event!(
                        "workspace {workspace}: bound {window:?} ({bundle}, {affinity:?}) to slot {slot:?}"
                    );
                }
                Decision::WindowRemoved { slot, window } => {
                    let bundle = self.bundle_id(&window.app).unwrap_or("unknown");
                    log_event!(
                        "workspace {workspace}: retired slot {slot:?} ({window:?} / {bundle} absent)"
                    );
                }
                Decision::NativeTabReplaced {
                    slot,
                    previous,
                    current,
                } => {
                    log_event!("workspace {workspace}: slot {slot:?} {previous:?} -> {current:?}");
                }
                Decision::ApplicationTerminated { app } => {
                    log_event!("workspace {workspace}: {app:?} terminated");
                }
                Decision::NativeTabAmbiguous { .. }
                | Decision::ObservationUnavailable { .. }
                | Decision::MouseDownDeferred => {}
            }
        }
    }
}

fn update_observed_frames(
    observation: &mut crate::NativeObservation,
    placements: &[crate::Placement],
) {
    let frames: HashMap<_, _> = placements
        .iter()
        .filter_map(|placement| {
            placement
                .window
                .as_ref()
                .map(|window| (window, placement.frame))
        })
        .collect();
    for application in &mut observation.applications {
        if let crate::AppObservation::Complete { windows, .. } = application {
            for window in windows {
                if let Some(frame) = frames.get(&window.id) {
                    window.frame = *frame;
                }
            }
        }
    }
}
