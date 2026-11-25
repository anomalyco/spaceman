use super::*;

/// One prompt retry while AX and the window server settle, without raising
/// the steady-state observation rate for a minimized or unresponsive app.
const CLOSE_RECHECK: Duration = Duration::from_millis(75);

pub(in crate::app) struct Omission {
    since: Instant,
    probe: Probe,
}

enum Probe {
    First,
    RetryAt(Instant),
    Done,
}

impl Omission {
    pub(in crate::app) fn new(since: Instant) -> Self {
        Self {
            since,
            probe: Probe::First,
        }
    }

    pub(super) fn is_settling(&self, now: Instant) -> bool {
        now.duration_since(self.since) < OMISSION_SETTLE
    }

    /// Next prompt recheck while an omitted window is still settling, bounded
    /// by the end of `OMISSION_SETTLE` so reflow happens as soon as macOS drops
    /// a closing window from the window server instead of waiting for the 1s
    /// fallback refresh.
    pub(super) fn next_recheck(&self, now: Instant) -> Instant {
        (now + CLOSE_RECHECK).min(self.since + OMISSION_SETTLE)
    }

    fn should_probe(&mut self, now: Instant) -> bool {
        match self.probe {
            Probe::First => self.probe = Probe::RetryAt(now + CLOSE_RECHECK),
            Probe::RetryAt(at) if now >= at => self.probe = Probe::Done,
            _ => return false,
        }
        true
    }
}

impl<B: WorkspaceBackend> WorkspaceRuntime<B> {
    /// An owned float that a Complete Accessibility snapshot omitted while the
    /// window server still shows it on screen keeps its workspace until the
    /// omission outlasts [`OMISSION_SETTLE`], like a bound tile does (#179).
    /// `onscreen` caches the window-server read for the rest of the cycle.
    pub(super) fn omitted_float_is_settling(
        &mut self,
        window: &crate::PhysicalWindowId,
        onscreen: &mut Option<Option<HashMap<u32, crate::Rect>>>,
    ) -> bool {
        let frames = onscreen.get_or_insert_with(|| self.backend.onscreen_window_frames());
        if !frames
            .as_ref()
            .is_some_and(|frames| frames.contains_key(&window.window_id))
        {
            return false;
        }
        let now = self.backend.now();
        self.omitted_windows
            .entry(window.clone())
            .or_insert_with(|| {
                log_event!(
                    "observation omitted floating {:?}: window server shows it on screen; keeping its workspace",
                    window
                );
                Omission::new(now)
            })
            .is_settling(now)
    }

    pub(super) fn ordered_out_window_is_settling(
        &mut self,
        workspace: WorkspaceId,
        window: &crate::PhysicalWindowId,
        sibling_visible_without_drag: bool,
        now: Instant,
        close_recheck: &mut Option<Instant>,
    ) -> bool {
        // A synthesized empty app snapshot retires every slot for that
        // app on this workspace. Limit early confirmation to one bound
        // tile, and leave native tab replacement to its settling policy.
        let slots = &self.workspaces.engine(workspace).model().workspace.slots;
        let mut owned = slots.iter().filter(|slot| slot.belongs_to(&window.app));
        let probe = sibling_visible_without_drag
            && !self.parked.contains_key(window)
            && owned.next().and_then(crate::Slot::bound_window) == Some(window)
            && owned.next().is_none()
            && self
                .bundle_id(&window.app)
                .is_some_and(|bundle| bundle != "com.mitchellh.ghostty");
        let omission = self
            .omitted_windows
            .entry(window.clone())
            .or_insert_with(|| {
                log_event!(
                    "observation omitted {:?}: window server lists it off screen; releasing its slot unless it returns",
                    window
                );
                Omission::new(now)
            });
        if !omission.is_settling(now) {
            return false;
        }
        // ⌘H or ⌘M is not a flaky snapshot; reflow now instead of after the settle.
        if !self.parked.contains_key(window) && self.backend.confirms_set_aside(window) {
            return false;
        }
        if probe && omission.should_probe(now) && self.backend.confirms_no_windows(&window.app) {
            return false;
        }
        if probe && let Probe::RetryAt(deadline) = omission.probe {
            *close_recheck = Some(close_recheck.map_or(deadline, |other| other.min(deadline)));
        }
        true
    }

    /// A bound window the window server still shows on screen, but which the
    /// app's Accessibility snapshot omitted (including a visible window left
    /// with its center off-display after a display disconnect or shrink), has
    /// not gone anywhere: the snapshot is incomplete for that app. Downgrade
    /// the app to `Unavailable` so reconciliation keeps its slots instead of
    /// retiring one and re-adopting the window as new a cycle later. The
    /// omission is believed once it outlasts [`OMISSION_SETTLE`].
    pub(super) fn guard_omitted_windows(
        &mut self,
        workspace: WorkspaceId,
        observation: &mut crate::NativeObservation,
        onscreen_frames: Option<&HashMap<u32, crate::Rect>>,
        close_recheck: &mut Option<Instant>,
    ) {
        if observation.mouse_down {
            return;
        }
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
                // An omitted float's settle is tracked by affinity freezing.
                None => self.workspaces.floating_workspace_of(window).is_some(),
            });
        let now = self.backend.now();
        for app_observation in &mut observation.applications {
            let crate::AppObservation::Complete { app, .. } = app_observation else {
                continue;
            };
            let settling = bound
                .iter()
                .filter(|window| window.app == app.id && !observed.contains(*window))
                .filter(|window| onscreen_frames.contains_key(&window.window_id))
                .filter(|window| {
                    let omission = self
                        .omitted_windows
                        .entry((*window).clone())
                        .or_insert_with(|| {
                            log_event!(
                                "observation omitted {:?}: window server shows it on screen; keeping its slot",
                                window
                            );
                            Omission::new(now)
                        });
                    if omission.is_settling(now) {
                        let deadline = omission.next_recheck(now);
                        *close_recheck =
                            Some(close_recheck.map_or(deadline, |other| other.min(deadline)));
                        true
                    } else {
                        false
                    }
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
    /// tile. So while another window of this workspace, or one it is about to
    /// adopt, is observed on screen, meaning the display is not showing
    /// another Space, an ordered-out window keeps its app's slots only until
    /// the omission outlasts [`OMISSION_SETTLE`]. A fresh empty AXWindows list
    /// can confirm a last-window close earlier.
    /// Unresolved replacement candidates keep the plain window-server rule.
    ///
    /// Right after a macOS Space slide, though, an ordered-out window is on the
    /// desktop the slide left. When the snapshot is clean and windows of this
    /// display arrived on screen with the slide, keeping its slot would tile
    /// both desktops' windows together, each at half width, until the
    /// omission settled (#431); it is released at once instead.
    pub(super) fn apps_with_window_server_evidence(
        &mut self,
        workspace: WorkspaceId,
        observation: &crate::NativeObservation,
        server_frames: Option<&HashMap<u32, crate::Rect>>,
        onscreen_frames: Option<&HashMap<u32, crate::Rect>>,
        close_recheck: &mut Option<Instant>,
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
        let observed = live::observed_frames(observation);
        let left_by_slide = onscreen_frames
            .filter(|onscreen| {
                observation
                    .applications
                    .iter()
                    .all(|app| matches!(app, crate::AppObservation::Complete { .. }))
                    && observed
                        .keys()
                        .any(|window| onscreen.contains_key(&window.window_id))
            })
            .filter(|_| self.space_slide_just_settled());
        // A window this cycle is about to adopt counts too: after a desktop
        // swipe the arriving desktop's window is often not a tile yet, and
        // keeping the outgoing desktop's slot would tile the two together
        // until the next cycle (#431).
        let mut observed_tiles = observed
            .keys()
            .filter(|window| {
                self.workspaces
                    .workspace_of(window)
                    .is_none_or(|owner| owner == workspace)
            })
            .peekable();
        let onscreen_frames = onscreen_frames.filter(|_| observed_tiles.peek().is_some());
        // The newer CG snapshot must still show a sibling. A Space switch
        // between the AX and CG reads can order out the entire workspace.
        let sibling_on_screen = onscreen_frames
            .is_some_and(|frames| observed_tiles.any(|tile| frames.contains_key(&tile.window_id)));
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
        let now = self.backend.now();
        for (window, settles) in windows {
            if settles
                && !self.parked.contains_key(&window)
                && left_by_slide.is_some_and(|onscreen| !onscreen.contains_key(&window.window_id))
            {
                log_event!(
                    "space slide: releasing {:?}, left on the desktop the slide left",
                    window
                );
                self.omitted_windows.remove(&window);
                continue;
            }
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
            if self.ordered_out_window_is_settling(
                workspace,
                &window,
                sibling_on_screen && !observation.mouse_down,
                now,
                close_recheck,
            ) {
                apps.insert(window.app);
            }
        }
        apps
    }
}
