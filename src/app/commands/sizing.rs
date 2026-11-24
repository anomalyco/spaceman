//! Keyboard edits of the focused tile's split: `resize shrink|grow`, `resize width|height`, and
//! `balance`.

use super::*;

const KEYBOARD_RESIZE_INCREMENT: i32 = 50;

impl<B: WorkspaceBackend> WorkspaceRuntime<B> {
    pub(in crate::app) fn resize_focused(
        &mut self,
        direction: ResizeDirection,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        self.edit_focused_split(
            "resizing",
            outputs,
            cancelled,
            |engine, window, settings| {
                engine.resize_focused_tile(window, direction, KEYBOARD_RESIZE_INCREMENT, settings)
            },
        )
    }

    pub(in crate::app) fn resize_focused_to(
        &mut self,
        axis: crate::Axis,
        points: i32,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        self.edit_focused_split(
            "resizing",
            outputs,
            cancelled,
            |engine, window, settings| {
                engine.resize_focused_tile_to(window, axis, points, settings)
            },
        )
    }

    pub(in crate::app) fn balance_focused(
        &mut self,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        self.edit_focused_split(
            "balancing",
            outputs,
            cancelled,
            |engine, window, settings| engine.balance_focused_parent(window, settings),
        )
    }

    /// Apply a keyboard edit of the focused tile's split layout, then relayout and persist when
    /// it changed anything. Fullscreen Layout has no split to edit.
    fn edit_focused_split(
        &mut self,
        verb: &str,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
        edit: impl FnOnce(&mut crate::Engine, &crate::PhysicalWindowId, &Settings) -> bool,
    ) -> Result<CycleStatus, String> {
        let (window, workspace) = self.focused_active_workspace(&outputs, cancelled, false)?;
        if self.workspaces.uses_fullscreen_layout(workspace) {
            return Err(format!(
                "Exit Fullscreen Layout before {verb} the split layout"
            ));
        }
        let settings = self.layout_settings(output_for(&self.workspaces, workspace, &outputs));
        if !edit(self.workspaces.engine_mut(workspace), &window, &settings) {
            return Ok(self.status(0, &outputs));
        }
        self.relayout_persist(workspace, &outputs, cancelled)
    }
}
