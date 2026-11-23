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
        if probe && omission.should_probe(now) && self.backend.confirms_no_windows(&window.app) {
            return false;
        }
        if probe && let Probe::RetryAt(deadline) = omission.probe {
            *close_recheck = Some(close_recheck.map_or(deadline, |other| other.min(deadline)));
        }
        true
    }
}
