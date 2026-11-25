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
        let mut written = None;
        let mut rounds = super::super::minimums::learning_rounds(0);
        let mut round = 0;
        while round < rounds {
            let known = self.minimum_sizes.clone();
            let Some((workspace, status)) = self.try_edit_focused_split(
                "resizing",
                &outputs,
                cancelled,
                |engine, window, settings, minimums| {
                    engine.resize_focused_tile_to(window, axis, points, settings, minimums)
                },
            )?
            else {
                break;
            };
            let tiles = self.workspaces.engine(workspace).bound_windows().len();
            rounds = rounds.max(super::super::minimums::learning_rounds(tiles));
            written = Some(status);
            // Writing the new split can reveal a sibling's minimum, and the refit around it
            // takes space from the resized tile too. Ask again with what was learned so the
            // request still lands when it fits (#492).
            if self.minimum_sizes == known || cancelled.load(Ordering::Acquire) {
                break;
            }
            round += 1;
        }
        Ok(written.unwrap_or_else(|| self.status(0, &outputs)))
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
        let status = self
            .try_edit_focused_split(verb, &outputs, cancelled, edit)?
            .map(|(_, status)| status);
        Ok(status.unwrap_or_else(|| self.status(0, &outputs)))
    }

    /// `edit_focused_split`, but `None` when the edit changed nothing and no layout was written.
    fn try_edit_focused_split(
        &mut self,
        verb: &str,
        outputs: &macos::DisplayOutputs,
        cancelled: &AtomicBool,
        edit: impl FnOnce(
            &mut crate::Engine,
            &crate::PhysicalWindowId,
            &Settings,
            &crate::runtime::WindowMinimums,
        ) -> bool,
    ) -> Result<Option<(WorkspaceId, CycleStatus)>, String> {
        let (window, workspace) = self.focused_active_workspace(outputs, cancelled, false)?;
        self.workspaces.deck(workspace).require_split(verb)?;
        let settings = self.layout_settings(output_for(&self.workspaces, workspace, outputs));
        let engine = self.workspaces.engine_mut(workspace);
        if !edit(engine, &window, &settings, &self.minimum_sizes) {
            return Ok(None);
        }
        // An explicit size is written exactly, even a point or two away (#412).
        self.exact_spacing_workspaces.insert(workspace);
        self.relayout_persist(workspace, outputs, cancelled)
            .map(|status| Some((workspace, status)))
    }
}
