//! macOS Space slides (Mission Control desktops, native fullscreen). While one
//! runs, windows from the outgoing and incoming Spaces are on screen together
//! and Accessibility writes to the incoming Space's windows can fail, so
//! background cycles wait for the slide to settle instead of laying out a mix
//! of both Spaces (#334) or rolling back a focus follow and re-homing the
//! window it revealed (#329).

use super::*;

/// How long after a failed focus follow a Space change may still explain it.
const FAILED_FOLLOW_RETRY_WINDOW: Duration = Duration::from_secs(3);

impl<B: WorkspaceBackend> WorkspaceRuntime<B> {
    /// When the macOS Space slide under way settles, while one is.
    pub(super) fn space_transition_until(&mut self) -> Option<Instant> {
        let until = self.backend.active_space_changed_at()? + SPACE_TRANSITION_SETTLE;
        (Instant::now() < until).then_some(until)
    }

    /// The status of a background cycle held back by a Space slide. Nothing
    /// was observed or written; the app loop retiles once the slide settles.
    pub(super) fn defer_for_space_transition(
        &self,
        until: Instant,
        outputs: &macos::DisplayOutputs,
    ) -> CycleStatus {
        log_event!(
            "space transition: holding layout for {}ms",
            until.saturating_duration_since(Instant::now()).as_millis()
        );
        let mut status = self.status(0, outputs);
        status.close_recheck = Some(until);
        status
    }

    /// A focus follow that failed while macOS was sliding between Spaces
    /// (Cmd+Tab out of a native fullscreen app) failed because of the slide,
    /// not the workspace. Lift the focus suppression its rollback set so the
    /// next cycle follows again, instead of leaving the revealed window for
    /// heal to re-home onto the workspace on screen. Returns whether it did.
    pub(super) fn retry_follow_after_space_change(&mut self) -> bool {
        let Some(started) = self.failed_follow_at else {
            return false;
        };
        if started.elapsed() >= FAILED_FOLLOW_RETRY_WINDOW {
            self.failed_follow_at = None;
            return false;
        }
        let Some(changed) = self.backend.active_space_changed_at() else {
            return false;
        };
        if changed + SPACE_TRANSITION_SETTLE <= started {
            // That slide had settled before the follow began.
            return false;
        }
        self.failed_follow_at = None;
        log_event!("focus follow: retrying after a macOS Space change interrupted it");
        self.clear_suppressed_focus();
        true
    }
}
