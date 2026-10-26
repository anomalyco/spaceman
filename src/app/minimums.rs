use super::*;

/// Size difference, in points, below which a constrained readback is noise.
const MINIMUM_TOLERANCE: i32 = 2;

impl<B: WorkspaceBackend> WorkspaceRuntime<B> {
    /// Grow the tiles of windows that refused to shrink to their target, so
    /// they stop overlapping their neighbors. A refused size is the window's
    /// minimum; neighbors give up the space unless that would push one of
    /// them below its own known minimum. Returns whether the layout changed.
    pub(super) fn fit_minimum_sizes(
        &mut self,
        workspace: WorkspaceId,
        output: crate::Rect,
        results: &[macos::ApplyResult],
    ) -> bool {
        if self.workspaces.uses_fullscreen_layout(workspace) {
            return false;
        }
        let mut changed = false;
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
            let grown = if wider {
                grow_width(target, actual.width, output)
            } else {
                grow_height(target, actual.height, output)
            };
            let engine = self.workspaces.engine_mut(workspace);
            let before = engine.layout().clone();
            if !engine.resize_window(&result.window, grown, output) {
                continue;
            }
            let settings = Settings::for_output(output);
            let squeezed = engine
                .placements(&settings)
                .into_iter()
                .filter_map(|placement| Some((placement.window?, placement.frame)))
                .find(|(window, frame)| {
                    window != &result.window
                        && self
                            .minimum_sizes
                            .get(window)
                            .is_some_and(|&(width, height)| {
                                frame.width + MINIMUM_TOLERANCE < width
                                    || frame.height + MINIMUM_TOLERANCE < height
                            })
                });
            if let Some((neighbor, _)) = squeezed {
                engine.restore_layout(before);
                log_event!(
                    "minimum size: {:?} needs {}x{} but growing it would squeeze {:?} below its minimum",
                    result.window,
                    actual.width,
                    actual.height,
                    neighbor
                );
                continue;
            }
            log_event!(
                "minimum size: grew {:?} on workspace {} to fit {}x{} (target was {}x{})",
                result.window,
                workspace,
                actual.width,
                actual.height,
                target.width,
                target.height
            );
            changed = true;
        }
        changed
    }
}

/// The target widened to `width`, moving its trailing edge unless that edge
/// is already at the output's, in which case the leading edge moves instead.
fn grow_width(target: crate::Rect, width: i32, output: crate::Rect) -> crate::Rect {
    let x = if target.x + target.width < output.x + output.width {
        target.x
    } else {
        target.x + target.width - width
    };
    crate::Rect { x, width, ..target }
}

fn grow_height(target: crate::Rect, height: i32, output: crate::Rect) -> crate::Rect {
    let y = if target.y + target.height < output.y + output.height {
        target.y
    } else {
        target.y + target.height - height
    };
    crate::Rect {
        y,
        height,
        ..target
    }
}
