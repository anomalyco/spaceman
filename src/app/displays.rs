//! Display changes. Parked positions are chosen against the connected displays;
//! when a display is added or removed they may land on screen, and recovery
//! frames recorded on a removed display would restore off every screen.

use super::*;

/// The displays the runtime last saw, and the hidden windows still to re-park
/// since they changed.
#[derive(Default)]
pub(super) struct DisplayWatch {
    known: Option<macos::DisplayOutputs>,
    repark_pending: HashSet<crate::PhysicalWindowId>,
}

impl<B: WorkspaceBackend> WorkspaceRuntime<B> {
    /// How tiles fill `output`: its frame with the configured gaps, padding and width cap.
    pub(super) fn layout_settings(&self, output: crate::Rect) -> Settings {
        Settings::for_output(output).spaced(self.config.spacing())
    }

    /// Record the current displays. When they differ from the last ones, every
    /// parked window is due a re-park against them.
    pub(super) fn note_outputs(&mut self, outputs: &macos::DisplayOutputs) {
        if self
            .displays
            .known
            .as_ref()
            .is_some_and(|known| !known.frames().eq(outputs.frames()))
        {
            log_event!("displays changed: re-parking hidden windows");
            self.displays.repark_pending = self.parked.keys().cloned().collect();
        }
        self.displays.known = Some(outputs.clone());
    }

    /// Note the displays, then re-park each hidden window still due one. Runs
    /// before focus follow and heal, which would otherwise take a window the
    /// new display uncovered for one the user brought back. A write that fails
    /// (the app is briefly inaccessible) stays due and is retried next time.
    pub(super) fn follow_displays(&mut self, outputs: &macos::DisplayOutputs) {
        self.note_outputs(outputs);
        if self.displays.repark_pending.is_empty() {
            return;
        }
        let visible: HashSet<_> = self
            .visible_workspaces(outputs)
            .into_iter()
            .map(|resolved| resolved.workspace)
            .collect();
        let displays: Vec<_> = outputs.frames().collect();
        for window in std::mem::take(&mut self.displays.repark_pending) {
            if !self.parked.contains_key(&window) || self.parking_exempt.contains(&window) {
                continue;
            }
            // Hidden scratchpad windows intentionally have no workspace owner, but still need to
            // stay parked off-screen when a newly connected display covers their parking edge.
            let output = if let Some(owner) = self.workspaces.workspace_of_any(&window) {
                if visible.contains(&owner) {
                    continue;
                }
                output_for(&self.workspaces, owner, outputs)
            } else if self
                .scratchpad
                .entry(&window)
                .is_some_and(|entry| entry.shown_on.is_none())
            {
                outputs.main().frame
            } else {
                continue;
            };
            if !self.repark_for_displays(&window, output, &displays) {
                self.displays.repark_pending.insert(window);
            }
        }
    }

    /// Whether `window` is parked for `displays` now.
    fn repark_for_displays(
        &mut self,
        window: &crate::PhysicalWindowId,
        output: crate::Rect,
        displays: &[crate::Rect],
    ) -> bool {
        let Some(current) = self.backend.window_server_frame(window) else {
            return false;
        };
        let target = crate::geometry::parked_frame(output, displays, current);
        if (target.x, target.y) == (current.x, current.y) {
            return true;
        }
        let outcome = self.backend.set_position(window, target);
        log_event!("displays changed: re-park {window:?} at {target:?} ({outcome:?})");
        if outcome != macos::ApplyOutcome::Applied {
            return false;
        }
        self.recently_parked.insert(window.clone(), Instant::now());
        true
    }

    /// Whether `window` still awaits its re-park after a display change. Until
    /// then a new display may show it, which is no reappearance to honor.
    pub(super) fn awaiting_display_repark(&self, window: &crate::PhysicalWindowId) -> bool {
        self.displays.repark_pending.contains(window)
    }

    /// Where a recovery frame is restored: itself while it shows on a connected
    /// display, otherwise moved onto the main display (its own is gone).
    pub(super) fn restore_target(&self, frame: crate::Rect) -> crate::Rect {
        let Some(outputs) = &self.displays.known else {
            return frame;
        };
        let displays: Vec<_> = outputs.frames().collect();
        crate::geometry::onto_displays(frame, &displays, outputs.main().frame)
    }
}
