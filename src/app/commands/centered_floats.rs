//! The Center Floats Lab (`[labs] center_floats`): Toggle Float centres the window it floats at
//! three quarters of its display (the Scratchpad's summon size), and `resize shrink|grow` and
//! `resize width|height` size a focused float around its centre instead of doing nothing. With the
//! switch off none of this runs and floats keep their exact frame.

use super::*;
use crate::scratchpad::summon_frame;

/// How far `resize shrink|grow` changes a float's width and height, matching a tile's step.
const FLOAT_RESIZE_STEP: i32 = 50;

impl<B: WorkspaceBackend> WorkspaceRuntime<B> {
    /// The focused window when the Lab is on and the window floats on a visible workspace;
    /// `None` sends the command down its usual path.
    pub(in crate::app) fn focused_centered_float(
        &mut self,
        outputs: &macos::DisplayOutputs,
    ) -> Option<crate::PhysicalWindowId> {
        if !self.config.labs().center_floats {
            return None;
        }
        let window = self.current_focus()?;
        if self.workspaces.workspace_of(&window).is_some() {
            return None;
        }
        let workspace = self.workspaces.workspace_of_any(&window)?;
        self.is_visible(workspace, outputs).then_some(window)
    }

    /// Centres a window that Toggle Float just floated, when the Lab is on.
    pub(in crate::app) fn center_new_float(
        &mut self,
        window: &crate::PhysicalWindowId,
        workspace: WorkspaceId,
        outputs: &macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> usize {
        if !self.config.labs().center_floats {
            return 0;
        }
        let area = output_for(&self.workspaces, workspace, outputs);
        self.write_float_frame(window, area, summon_frame(area, None), cancelled)
    }

    /// `resize shrink|grow` on a focused float: one step on both axes, around its centre.
    pub(in crate::app) fn resize_float(
        &mut self,
        window: &crate::PhysicalWindowId,
        direction: ResizeDirection,
        outputs: &macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        let step = match direction {
            ResizeDirection::Grow => FLOAT_RESIZE_STEP,
            ResizeDirection::Shrink => -FLOAT_RESIZE_STEP,
        };
        let stepped = |width: i32, height: i32| (width + step, height + step);
        self.resize_float_with(window, outputs, cancelled, stepped)
    }

    /// `resize width|height N` on a focused float: that axis set to `points`, around its centre.
    pub(in crate::app) fn resize_float_to(
        &mut self,
        window: &crate::PhysicalWindowId,
        axis: crate::Axis,
        points: i32,
        outputs: &macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        self.resize_float_with(window, outputs, cancelled, |width, height| match axis {
            crate::Axis::Horizontal => (points, height),
            crate::Axis::Vertical => (width, points),
        })
    }

    fn resize_float_with(
        &mut self,
        window: &crate::PhysicalWindowId,
        outputs: &macos::DisplayOutputs,
        cancelled: &AtomicBool,
        size: impl FnOnce(i32, i32) -> (i32, i32),
    ) -> Result<CycleStatus, String> {
        let current = self
            .backend
            .frame(window)
            .map_err(|_| "Focused window is unavailable")?;
        // The display the float is on, which a dragged float need not share with its workspace.
        let center = (current.x + current.width / 2, current.y + current.height / 2);
        let area = outputs.display_for_point(center).frame;
        let frame = resized_around_center(current, area, size(current.width, current.height));
        let failed = if frame == current {
            0
        } else {
            self.write_float_frame(window, area, frame, cancelled)
        };
        Ok(self.status(failed, outputs))
    }

    fn write_float_frame(
        &mut self,
        window: &crate::PhysicalWindowId,
        area: crate::Rect,
        frame: crate::Rect,
        cancelled: &AtomicBool,
    ) -> usize {
        let placement = crate::Placement {
            slot: crate::SlotId(0),
            window: Some(window.clone()),
            frame,
        };
        let settings = self.layout_settings(area);
        failed_results(&self.backend.apply_managed(&[placement], &settings, cancelled))
    }
}

/// `current` resized to `size` (no smaller than the summon minimum, no larger than `area`),
/// keeping its centre where it was but kept inside `area`.
fn resized_around_center(current: crate::Rect, area: crate::Rect, size: (i32, i32)) -> crate::Rect {
    let sized = summon_frame(area, Some(size));
    let center_x = current.x + current.width / 2;
    let center_y = current.y + current.height / 2;
    let x = (center_x - sized.width / 2).clamp(area.x, area.x + area.width - sized.width);
    let y = (center_y - sized.height / 2).clamp(area.y, area.y + area.height - sized.height);
    crate::Rect { x, y, ..sized }
}

#[cfg(test)]
mod tests {
    use super::resized_around_center;
    use crate::Rect;

    const AREA: Rect = Rect {
        x: 0,
        y: 25,
        width: 1600,
        height: 1000,
    };

    #[test]
    fn a_resized_float_keeps_its_centre() {
        let current = Rect {
            x: 400,
            y: 300,
            width: 800,
            height: 600,
        };
        assert_eq!(
            resized_around_center(current, AREA, (900, 700)),
            Rect {
                x: 350,
                y: 250,
                width: 900,
                height: 700,
            }
        );
    }

    #[test]
    fn a_resized_float_stays_on_its_display() {
        let near_edge = Rect {
            x: 1100,
            y: 25,
            width: 500,
            height: 400,
        };
        let grown = resized_around_center(near_edge, AREA, (700, 600));
        assert_eq!((grown.x, grown.y), (900, 25));
        assert_eq!(resized_around_center(near_edge, AREA, (5000, 5000)), AREA);
        let tiny = resized_around_center(near_edge, AREA, (10, 10));
        assert_eq!((tiny.width, tiny.height), (320, 240));
    }
}
