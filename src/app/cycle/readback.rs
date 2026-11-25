use super::*;

/// How long a freshly applied target stays armed for bounce readback.
/// Electron apps accept `AXSize` writes synchronously and emit `AXResized`
/// before their async resize clamp (~40–100 ms later) snaps the window back;
/// wiping `applied` on an early matching tick would miss that snap-back and
/// restart an infinite resize-flicker loop.
pub(in crate::app) const ASYNC_BOUNCE_SETTLE: Duration = Duration::from_millis(500);

/// Consecutive bounced or rejected applications for the same target frame
/// before Spaceman stops retrying that target (even when rejected frames vary
/// each cycle), preventing infinite resize-fight flicker loops.
pub(in crate::app) const APPLY_ATTEMPT_LIMIT: u8 = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) struct AppliedFrame {
    pub(in crate::app) target: crate::Rect,
    pub(in crate::app) at: Instant,
}

impl AppliedFrame {
    pub(in crate::app) fn now(target: crate::Rect) -> Self {
        Self {
            target,
            at: Instant::now(),
        }
    }

    pub(super) fn is_settling(&self, target: crate::Rect) -> bool {
        self.target == target && self.at.elapsed() < ASYNC_BOUNCE_SETTLE
    }
}

impl<B: WorkspaceBackend> WorkspaceRuntime<B> {
    pub(in crate::app) fn record_results(
        &mut self,
        workspace: WorkspaceId,
        results: &[macos::ApplyResult],
    ) {
        let constrained = &mut self.constrained[workspace.index()];
        let applied = &mut self.applied[workspace.index()];
        let attempts = &mut self.apply_attempts[workspace.index()];
        for result in results {
            // A write right after a move to another display can come back
            // cut to the display the window left (a window sent to a taller
            // portrait display stops at the old bottom edge). Leave it
            // unaccepted so the next cycle writes it again from where it now
            // is, until the attempt limit accepts it as it is (#428).
            let short_after_move = self.relocated.contains(&result.window)
                && matches!(
                    (result.outcome, result.actual),
                    (macos::ApplyOutcome::Constrained, Some(actual))
                        if falls_short(actual, result.target)
                );
            // A drag defers the write; the window keeps its pending retry.
            if !short_after_move && result.outcome != macos::ApplyOutcome::Deferred {
                self.relocated.remove(&result.window);
            }
            match (result.outcome, result.actual) {
                (macos::ApplyOutcome::Constrained, Some(actual)) if short_after_move => {
                    applied.remove(&result.window);
                    constrained.remove(&result.window);
                    record_apply_attempt(attempts, workspace, &result.window, result.target);
                    if attempts
                        .get(&result.window)
                        .is_some_and(|(_, count)| *count >= APPLY_ATTEMPT_LIMIT)
                    {
                        constrained.insert(result.window.clone(), (result.target, actual));
                        self.relocated.remove(&result.window);
                    }
                }
                (macos::ApplyOutcome::Constrained, Some(actual)) => {
                    applied.remove(&result.window);
                    constrained.insert(result.window.clone(), (result.target, actual));
                    record_apply_attempt(attempts, workspace, &result.window, result.target);
                }
                (macos::ApplyOutcome::Applied, _) => {
                    constrained.remove(&result.window);
                    if attempts
                        .get(&result.window)
                        .is_some_and(|(target, _)| *target != result.target)
                    {
                        attempts.remove(&result.window);
                    }
                    applied.insert(result.window.clone(), AppliedFrame::now(result.target));
                }
                (macos::ApplyOutcome::Gone, _) => {
                    applied.remove(&result.window);
                    constrained.remove(&result.window);
                    attempts.remove(&result.window);
                }
                (macos::ApplyOutcome::Deferred, _) => {
                    applied.remove(&result.window);
                }
                _ => {
                    applied.remove(&result.window);
                    record_apply_attempt(attempts, workspace, &result.window, result.target);
                }
            }
        }
    }
}

/// Whether a readback is smaller than its target on either axis. A larger
/// readback is a minimum size, which refitting handles instead.
fn falls_short(actual: crate::Rect, target: crate::Rect) -> bool {
    actual.width < target.width || actual.height < target.height
}

pub(in crate::app) fn record_apply_attempt(
    attempts: &mut HashMap<crate::PhysicalWindowId, (crate::Rect, u8)>,
    workspace: WorkspaceId,
    window: &crate::PhysicalWindowId,
    target: crate::Rect,
) {
    let entry = attempts.entry(window.clone()).or_insert((target, 0));
    if entry.0 != target {
        *entry = (target, 0);
    }
    let previous = entry.1;
    entry.1 = entry.1.saturating_add(1);
    if previous < APPLY_ATTEMPT_LIMIT && entry.1 >= APPLY_ATTEMPT_LIMIT {
        log_event!(
            "workspace {workspace}: freezing {window:?} after {} bounced/rejected applications for {target:?}",
            APPLY_ATTEMPT_LIMIT
        );
    }
}
