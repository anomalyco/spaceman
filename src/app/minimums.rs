use super::*;

/// Size difference, in points, below which a constrained readback is noise.
const MINIMUM_TOLERANCE: i32 = 2;
/// Refitting can shrink a sibling below a minimum nobody knew yet, which the
/// next write reveals. A few rounds settle that; a bound keeps it finite.
const MAX_REFITS: usize = 3;

impl<B: WorkspaceBackend> WorkspaceRuntime<B> {
    /// Refit and re-apply while writes keep revealing refused sizes. Returns
    /// the status of the last re-apply, or `None` when nothing was refitted.
    pub(super) fn refit_minimum_sizes(
        &mut self,
        workspace: WorkspaceId,
        output: crate::Rect,
        results: Vec<macos::ApplyResult>,
        cancelled: &AtomicBool,
    ) -> Option<LayoutStatus> {
        let mut results = results;
        let mut status = None;
        for _ in 0..MAX_REFITS {
            if !self.fit_minimum_sizes(workspace, output, &results) {
                break;
            }
            let (applied, next) = self.apply_layout(workspace, output, None, cancelled);
            status = Some(applied);
            results = next;
        }
        status
    }

    /// Grow the tiles of windows that refused to shrink to their target, so
    /// they stop overlapping their neighbors. A refused size is the window's
    /// minimum; siblings give up the space in proportion to what they can
    /// spare above their own known minimums. Returns whether the layout changed.
    fn fit_minimum_sizes(
        &mut self,
        workspace: WorkspaceId,
        output: crate::Rect,
        results: &[macos::ApplyResult],
    ) -> bool {
        if self.workspaces.uses_fullscreen_layout(workspace) || self.uses_strip(workspace) {
            return false;
        }
        let mut refused = false;
        for result in results {
            let (macos::ApplyOutcome::Constrained, Some(actual)) = (result.outcome, result.actual)
            else {
                continue;
            };
            let target = result.target;
            let wider =
                actual.width > target.width + MINIMUM_TOLERANCE && actual.width <= output.width;
            let taller =
                actual.height > target.height + MINIMUM_TOLERANCE && actual.height <= output.height;
            if !wider && !taller {
                continue;
            }
            let minimum = self
                .minimum_sizes
                .entry(result.window.clone())
                .or_insert((0, 0));
            if wider {
                minimum.0 = minimum.0.max(actual.width);
            }
            if taller {
                minimum.1 = minimum.1.max(actual.height);
            }
            refused = true;
        }
        if !refused {
            return false;
        }
        let engine = self.workspaces.engine_mut(workspace);
        if !engine.fit_minimums(output, &self.minimum_sizes) {
            log_event!(
                "minimum size: workspace {} cannot fit its windows' minimum sizes",
                workspace
            );
            return false;
        }
        for placement in engine.placements(&Settings::for_output(output)) {
            engine.record_placement_target(placement.slot, placement.frame);
        }
        log_event!(
            "minimum size: refit workspace {} around windows that refused their tile size",
            workspace
        );
        true
    }
}
