//! Keyboard edits of the focused tile's split: `resize shrink|grow`, `resize width|height`,
//! `resize width|height grow|shrink` (the Relative resize Lab), and `balance`.

use super::*;

const KEYBOARD_RESIZE_INCREMENT: i32 = 50;
const RELATIVE_RESIZE_OFF: &str =
    "Relative resize is off (set [labs] relative_resize = true in config.toml)";

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
            |engine, window, settings, minimums| {
                engine.resize_focused_tile(
                    window,
                    direction,
                    KEYBOARD_RESIZE_INCREMENT,
                    settings,
                    minimums,
                )
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
            |engine, window, settings, minimums| {
                engine.resize_focused_tile_to(window, axis, points, settings, minimums)
            },
        )
    }

    pub(in crate::app) fn resize_focused_by(
        &mut self,
        axis: crate::Axis,
        delta: i32,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        if !self.config.labs().relative_resize {
            return Err(RELATIVE_RESIZE_OFF.into());
        }
        self.edit_focused_split(
            "resizing",
            outputs,
            cancelled,
            |engine, window, settings, minimums| {
                engine.resize_focused_tile_by(window, axis, delta, settings, minimums)
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
            |engine, window, settings, minimums| {
                engine.balance_focused_parent(window, settings, minimums)
            },
        )
    }

    /// Apply a keyboard edit of the focused tile's split layout, then relayout and persist when
    /// it changed anything. Fullscreen Layout has no split to edit.
    fn edit_focused_split(
        &mut self,
        verb: &str,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
        edit: impl FnOnce(
            &mut crate::Engine,
            &crate::PhysicalWindowId,
            &Settings,
            &crate::runtime::WindowMinimums,
        ) -> bool,
    ) -> Result<CycleStatus, String> {
        let (window, workspace) = self.focused_active_workspace(&outputs, cancelled, false)?;
        self.workspaces.deck(workspace).require_split(verb)?;
        let settings = self.layout_settings(output_for(&self.workspaces, workspace, &outputs));
        let engine = self.workspaces.engine_mut(workspace);
        if !edit(engine, &window, &settings, &self.minimum_sizes) {
            return Ok(self.status(0, &outputs));
        }
        // An explicit size is written exactly, even a point or two away (#412).
        self.exact_spacing_workspaces.insert(workspace);
        self.relayout_persist(workspace, &outputs, cancelled)
    }
}
