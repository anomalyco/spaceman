//! Bounded deadline for `ctl restart` window restoration.
//!
//! `scripts/install-app.sh` gives `spaceman-agent` 60 s to restore parked
//! windows and exit before aborting the install. While the macOS screen is
//! locked, Accessibility returns `Gone` for every window and restoration keeps
//! retrying every second; without a deadline, the pending restart would stay
//! armed until unlock and quit right as the user returns. Cancelling after
//! [`RESTART_RESTORE_DEADLINE`] (45 s) returns Spaceman to normal operation
//! before the installer's wait expires.

use std::time::{Duration, Instant};

use super::*;

/// Maximum time a `ctl restart` restore may retry before cancelling cleanly.
/// Shorter than `scripts/install-app.sh`'s 60 s wait so a stalled restore
/// cancels and resumes normal operation instead of quitting later on unlock.
pub(super) const RESTART_RESTORE_DEADLINE: Duration = Duration::from_secs(45);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PendingRestart {
    deadline: Instant,
    resume_automatic: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CancelledRestart {
    pub(super) resume_automatic: bool,
}

impl PendingRestart {
    pub(super) fn new(now: Instant, resume_automatic: bool) -> Self {
        Self {
            deadline: now + RESTART_RESTORE_DEADLINE,
            resume_automatic,
        }
    }

    pub(super) const fn deadline(self) -> Instant {
        self.deadline
    }

    pub(super) const fn resume_automatic(self) -> bool {
        self.resume_automatic
    }
}

/// Cancels `pending` once `now` reaches its deadline, clearing `exit_intent`,
/// `next_restore_retry`, and `worker`'s restoration state, and arming
/// `pending_automatic_start` when tiling was automatic before the restart.
pub(super) fn expire_pending_restart(
    now: Instant,
    pending: &mut Option<PendingRestart>,
    exit_intent: &mut Option<ExitIntent>,
    next_restore_retry: &mut Option<Instant>,
    worker: &mut WorkerDispatch,
    pending_automatic_start: &mut bool,
) -> Option<CancelledRestart> {
    let active = (*pending)?;
    if *exit_intent != Some(ExitIntent::Restart) {
        *pending = None;
        return None;
    }
    if now < active.deadline() {
        return None;
    }
    *pending = None;
    *exit_intent = None;
    *next_restore_retry = None;
    worker.cancel_restore();
    if active.resume_automatic() {
        *pending_automatic_start = true;
    }
    Some(CancelledRestart {
        resume_automatic: active.resume_automatic(),
    })
}

impl MainThreadAgent {
    pub(super) fn check_restart_deadline(&mut self, now: Instant) {
        let Some(cancelled) = expire_pending_restart(
            now,
            &mut self.pending_restart,
            &mut self.exit_intent,
            &mut self.next_restore_retry,
            &mut self.worker,
            &mut self.pending_automatic_start,
        ) else {
            return;
        };
        let target = if cancelled.resume_automatic {
            "automatic tiling"
        } else {
            "paused"
        };
        log_event!(
            "restart cancelled: parked windows did not finish restoring within {}s; resuming {target}",
            RESTART_RESTORE_DEADLINE.as_secs()
        );
        self.ui_dirty = true;
        if !cancelled.resume_automatic && !self.worker.is_restoring_or_pending() {
            status_item::set_status(self.paused_status());
        }
    }

    pub(super) fn finish_cancelled_restore(&mut self) {
        self.worker.finish_restore();
        self.ui_dirty = true;
        if !self.pending_automatic_start {
            status_item::set_status(self.paused_status());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    #[test]
    fn restart_deadline_cancels_retry_and_resumes_automatic_after_45_seconds() {
        let (tx, _rx) = mpsc::channel();
        let mut worker = WorkerDispatch::new(tx);
        let start = Instant::now();
        let mut pending = Some(PendingRestart::new(start, true));
        let mut exit_intent = Some(ExitIntent::Restart);
        let mut next_retry = Some(start + Duration::from_secs(1));
        let mut pending_automatic_start = false;

        // Simulate a failed restore attempt waiting on the 1 s retry timer.
        worker.request_restore(RestoreKind::Restore);
        worker.defer_restore_retry();
        assert_eq!(
            worker.lifecycle(),
            WorkerLifecycle::RestoreRetryDue(RestoreKind::Restore)
        );

        // Before the 45 s deadline, the restart stays armed and retrying.
        assert_eq!(
            expire_pending_restart(
                start + RESTART_RESTORE_DEADLINE - Duration::from_millis(1),
                &mut pending,
                &mut exit_intent,
                &mut next_retry,
                &mut worker,
                &mut pending_automatic_start,
            ),
            None
        );
        assert_eq!(exit_intent, Some(ExitIntent::Restart));
        assert!(next_retry.is_some());
        assert!(!pending_automatic_start);
        assert_eq!(
            worker.lifecycle(),
            WorkerLifecycle::RestoreRetryDue(RestoreKind::Restore)
        );

        // At the 45 s deadline, the restart cancels cleanly and resumes automatic tiling.
        assert_eq!(
            expire_pending_restart(
                start + RESTART_RESTORE_DEADLINE,
                &mut pending,
                &mut exit_intent,
                &mut next_retry,
                &mut worker,
                &mut pending_automatic_start,
            ),
            Some(CancelledRestart {
                resume_automatic: true,
            })
        );
        assert_eq!(pending, None);
        assert_eq!(exit_intent, None);
        assert_eq!(next_retry, None);
        assert!(pending_automatic_start);
        assert_eq!(worker.lifecycle(), WorkerLifecycle::Idle);
        assert!(!worker.is_restoring_or_pending());
    }

    #[test]
    fn restart_deadline_waits_for_in_flight_restore_then_resumes_without_quitting() {
        let (tx, _rx) = mpsc::channel();
        let mut worker = WorkerDispatch::new(tx);
        let start = Instant::now();
        let mut pending = Some(PendingRestart::new(start, true));
        let mut exit_intent = Some(ExitIntent::Restart);
        let mut next_retry = None;
        let mut pending_automatic_start = false;

        // A RestoreAll attempt is still executing on the worker thread at 45 s.
        worker.request_restore(RestoreKind::Restore);
        assert_eq!(
            worker.lifecycle(),
            WorkerLifecycle::Restoring {
                kind: RestoreKind::Restore,
                again: None,
            }
        );

        assert_eq!(
            expire_pending_restart(
                start + RESTART_RESTORE_DEADLINE,
                &mut pending,
                &mut exit_intent,
                &mut next_retry,
                &mut worker,
                &mut pending_automatic_start,
            ),
            Some(CancelledRestart {
                resume_automatic: true,
            })
        );
        assert_eq!(exit_intent, None);
        assert!(pending_automatic_start);
        assert_eq!(worker.lifecycle(), WorkerLifecycle::CancellingRestore);
        assert!(worker.is_busy());
        assert!(worker.is_cancelling_restore());
        assert!(!worker.is_restoring());
        assert!(worker.is_restoring_or_pending());

        // When the in-flight restore finally returns (whether failed or succeeded),
        // finish_restore transitions cleanly back to Idle so automatic mode can start.
        worker.finish_restore();
        assert_eq!(worker.lifecycle(), WorkerLifecycle::Idle);
        assert!(!worker.is_busy());
        assert!(!worker.is_restoring_or_pending());
        assert!(pending_automatic_start);
    }

    #[test]
    fn restart_deadline_keeps_paused_mode_when_started_paused() {
        let (tx, _rx) = mpsc::channel();
        let mut worker = WorkerDispatch::new(tx);
        let start = Instant::now();
        let mut pending = Some(PendingRestart::new(start, false));
        let mut exit_intent = Some(ExitIntent::Restart);
        let mut next_retry = Some(start + Duration::from_secs(1));
        let mut pending_automatic_start = false;

        worker.request_restore(RestoreKind::Restore);
        worker.defer_restore_retry();

        assert_eq!(
            expire_pending_restart(
                start + RESTART_RESTORE_DEADLINE,
                &mut pending,
                &mut exit_intent,
                &mut next_retry,
                &mut worker,
                &mut pending_automatic_start,
            ),
            Some(CancelledRestart {
                resume_automatic: false,
            })
        );
        assert_eq!(exit_intent, None);
        assert_eq!(next_retry, None);
        assert!(!pending_automatic_start);
        assert_eq!(worker.lifecycle(), WorkerLifecycle::Idle);
    }

    #[test]
    fn restart_deadline_never_cancels_quit_terminate_or_update() {
        for intent in [ExitIntent::Quit, ExitIntent::Terminate, ExitIntent::Update] {
            let (tx, _rx) = mpsc::channel();
            let mut worker = WorkerDispatch::new(tx);
            let start = Instant::now();
            let mut pending = Some(PendingRestart::new(start, true));
            let mut exit_intent = Some(intent);
            let mut next_retry = Some(start + Duration::from_secs(1));
            let mut pending_automatic_start = false;

            worker.request_restore(RestoreKind::Restore);
            worker.defer_restore_retry();

            assert_eq!(
                expire_pending_restart(
                    start + RESTART_RESTORE_DEADLINE + Duration::from_secs(60),
                    &mut pending,
                    &mut exit_intent,
                    &mut next_retry,
                    &mut worker,
                    &mut pending_automatic_start,
                ),
                None
            );
            assert_eq!(exit_intent, Some(intent));
            assert!(next_retry.is_some());
            assert!(!pending_automatic_start);
            assert_eq!(
                worker.lifecycle(),
                WorkerLifecycle::RestoreRetryDue(RestoreKind::Restore)
            );
        }
    }
}
