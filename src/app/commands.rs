use super::*;

const KEYBOARD_RESIZE_INCREMENT: i32 = 50;

impl<B: WorkspaceBackend> WorkspaceRuntime<B> {
    pub(super) fn reorder_focused(
        &mut self,
        direction: Direction,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        let (window, workspace) = self.focused_active_workspace(&outputs, cancelled, true)?;
        if self.workspaces.uses_fullscreen_layout(workspace) {
            return Err("Exit Fullscreen Layout before reordering the split layout".into());
        }
        let windows = self.workspaces.engine(workspace).bound_windows();
        let axis = self.workspaces.engine(workspace).root_axis();
        let perpendicular = matches!(
            (axis, direction),
            (crate::Axis::Horizontal, Direction::Down | Direction::Up)
                | (crate::Axis::Vertical, Direction::Left | Direction::Right)
        );
        let tiles = self.navigation_tiles(&outputs);
        let local_tiles = tiles
            .iter()
            .filter(|tile| tile.workspace == workspace)
            .cloned()
            .collect::<Vec<_>>();
        if let Some(target) = crate::navigation::target(&local_tiles, &window, direction) {
            if !self
                .workspaces
                .engine_mut(workspace)
                .swap_windows(&window, &target.window)
            {
                return Ok(self.status(0, &outputs));
            }
            return self.relayout_persist(workspace, &outputs, cancelled);
        }
        if windows.len() >= 2
            && perpendicular
            && self
                .workspaces
                .engine_mut(workspace)
                .create_implicit_split(&window, direction)
        {
            return self.relayout_persist(workspace, &outputs, cancelled);
        }
        let Some(target) = crate::navigation::target(&tiles, &window, direction) else {
            return Ok(self.status(0, &outputs));
        };
        self.move_window(window, target.workspace, outputs, cancelled)
    }

    pub(super) fn drop_window(
        &mut self,
        window: crate::PhysicalWindowId,
        point: (i32, i32),
        preview: bool,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        let Some(source) = self.workspaces.workspace_of(&window) else {
            return self.cycle(outputs, cancelled);
        };
        if self.workspaces.uses_fullscreen_layout(source) {
            return Ok(self.status(0, &outputs));
        }
        self.constrained[source.index()].remove(&window);
        self.applied[source.index()].remove(&window);
        let target = self
            .navigation_tiles(&outputs)
            .into_iter()
            .find(|tile| tile.window != window && tile.frame.contains(point.0, point.1));
        if let Some(target) = target {
            if target.workspace != source {
                if preview {
                    return Ok(self.status(0, &outputs));
                }
                return self.move_window(window, target.workspace, outputs, cancelled);
            }
            let direction = drop_direction(point, target.frame);
            let changed = match direction {
                Some(direction) => self.workspaces.engine_mut(source).move_window_relative(
                    &window,
                    &target.window,
                    direction,
                ),
                None => self
                    .workspaces
                    .engine_mut(source)
                    .swap_windows(&window, &target.window),
            };
            if changed {
                let layout = self.layout_workspace_during_drag(
                    source,
                    output_for(&self.workspaces, source, &outputs),
                    preview.then_some(&window),
                    cancelled,
                );
                if !preview {
                    self.persist_workspace_state_if_changed()?;
                }
                return Ok(self.status(layout.failed, &outputs));
            }
        }
        if let Some(target) = self
            .visible_workspaces(&outputs)
            .into_iter()
            .find(|resolved| resolved.output.contains(point.0, point.1))
            .map(|resolved| resolved.workspace)
            && target != source
        {
            if preview {
                return Ok(self.status(0, &outputs));
            }
            return self.move_window(window, target, outputs, cancelled);
        }
        let layout = self.layout_workspace(
            source,
            output_for(&self.workspaces, source, &outputs),
            cancelled,
        );
        if !preview {
            self.persist_workspace_state_if_changed()?;
        }
        Ok(self.status(layout.failed, &outputs))
    }

    pub(super) fn resize_window(
        &mut self,
        window: crate::PhysicalWindowId,
        frame: crate::Rect,
        preview: bool,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        let Some(workspace) = self.workspaces.workspace_of(&window) else {
            return self.cycle(outputs, cancelled);
        };
        if self.workspaces.uses_fullscreen_layout(workspace) {
            return Ok(self.status(0, &outputs));
        }
        self.constrained[workspace.index()].remove(&window);
        self.applied[workspace.index()].remove(&window);
        let output = output_for(&self.workspaces, workspace, &outputs);
        let failed = if self
            .workspaces
            .engine_mut(workspace)
            .resize_window(&window, frame, output)
        {
            self.layout_workspace_during_drag(workspace, output, Some(&window), cancelled)
                .failed
        } else {
            0
        };
        if !preview {
            self.persist_workspace_state_if_changed()?;
        }
        Ok(self.status(failed, &outputs))
    }

    pub(super) fn focus_relative(
        &mut self,
        direction: Direction,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        let (focused, workspace) = self.focused_active_workspace(&outputs, cancelled, true)?;
        let Some(target) = self
            .stacked_target(workspace, &focused, direction)
            .or_else(|| {
                self.spatial_target(&focused, direction, &outputs)
                    .map(|(_, target)| target)
            })
        else {
            return Ok(self.status(0, &outputs));
        };
        if !self.backend.focus_window(&target) {
            return Err("Target window could not be focused".into());
        }
        self.remember_fullscreen_focus(workspace, &target)?;
        Ok(self.status(0, &outputs))
    }

    /// Fullscreen Layout tiles share one frame, so spatial navigation cannot
    /// tell them apart. Left/up and right/down instead step backward and
    /// forward through the stable tile order, without wrapping, so the ends of
    /// the stack still lead to the adjacent display.
    fn stacked_target(
        &self,
        workspace: WorkspaceId,
        focused: &crate::PhysicalWindowId,
        direction: Direction,
    ) -> Option<crate::PhysicalWindowId> {
        if !self.workspaces.uses_fullscreen_layout(workspace) {
            return None;
        }
        let windows = self.workspaces.engine(workspace).bound_windows();
        let index = windows.iter().position(|window| window == focused)?;
        let target = match direction {
            Direction::Left | Direction::Up => index.checked_sub(1)?,
            Direction::Right | Direction::Down => index + 1,
        };
        windows.get(target).cloned()
    }

    pub(super) fn cycle_focus(
        &mut self,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        let (focused, workspace) = self.focused_active_workspace(&outputs, cancelled, true)?;
        let windows = self.workspaces.engine(workspace).bound_windows();
        let Some(index) = windows.iter().position(|window| window == &focused) else {
            return Ok(self.status(0, &outputs));
        };
        if windows.len() < 2 {
            return Ok(self.status(0, &outputs));
        }
        let target = &windows[(index + 1) % windows.len()];
        if !self.backend.focus_window(target) {
            return Err("Target window could not be focused".into());
        }
        self.remember_fullscreen_focus(workspace, target)?;
        Ok(self.status(0, &outputs))
    }

    fn focused_active_workspace(
        &mut self,
        outputs: &macos::DisplayOutputs,
        cancelled: &AtomicBool,
        refresh_if_unmanaged: bool,
    ) -> Result<(crate::PhysicalWindowId, WorkspaceId), String> {
        let window = self
            .backend
            .focused_window()
            .ok_or("Focused window is unavailable")?;
        let workspace = match self.workspaces.workspace_of(&window) {
            Some(workspace) => workspace,
            None if refresh_if_unmanaged => {
                self.cycle(outputs.clone(), cancelled)?;
                self.workspaces
                    .workspace_of(&window)
                    .ok_or("Focused window is not managed by Spaceman")?
            }
            None => return Err("Focused window is not managed by Spaceman".into()),
        };
        if !self.is_visible(workspace, outputs) {
            return Err("Focused window is not on an active workspace".into());
        }
        Ok((window, workspace))
    }

    fn remember_fullscreen_focus(
        &mut self,
        workspace: WorkspaceId,
        window: &crate::PhysicalWindowId,
    ) -> Result<(), String> {
        if !self.workspaces.uses_fullscreen_layout(workspace) {
            return Ok(());
        }
        self.pending_fullscreen_focus_restore.remove(&workspace);
        self.workspaces.remember_focus(workspace, window);
        self.persist_workspace_state_if_changed()
    }

    pub(super) fn toggle_layout(
        &mut self,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        let (_, workspace) = self.focused_active_workspace(&outputs, cancelled, false)?;
        if self.workspaces.uses_fullscreen_layout(workspace) {
            self.workspaces.set_fullscreen_layout(workspace, false);
            self.pending_fullscreen_focus_restore.remove(&workspace);
        }
        self.workspaces.engine_mut(workspace).toggle_axis();
        self.relayout_persist(workspace, &outputs, cancelled)
    }

    pub(super) fn resize_focused(
        &mut self,
        direction: ResizeDirection,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        let (window, workspace) = self.focused_active_workspace(&outputs, cancelled, false)?;
        if self.workspaces.uses_fullscreen_layout(workspace) {
            return Err("Exit Fullscreen Layout before resizing the split layout".into());
        }
        let output = output_for(&self.workspaces, workspace, &outputs);
        if !self.workspaces.engine_mut(workspace).resize_focused_tile(
            &window,
            direction,
            KEYBOARD_RESIZE_INCREMENT,
            output,
        ) {
            return Ok(self.status(0, &outputs));
        }
        self.relayout_persist(workspace, &outputs, cancelled)
    }

    pub(super) fn balance_focused(
        &mut self,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        let (window, workspace) = self.focused_active_workspace(&outputs, cancelled, false)?;
        if self.workspaces.uses_fullscreen_layout(workspace) {
            return Err("Exit Fullscreen Layout before balancing the split layout".into());
        }
        let output = output_for(&self.workspaces, workspace, &outputs);
        if !self
            .workspaces
            .engine_mut(workspace)
            .balance_focused_parent(&window, output)
        {
            return Ok(self.status(0, &outputs));
        }
        self.relayout_persist(workspace, &outputs, cancelled)
    }

    pub(super) fn toggle_fullscreen_layout(
        &mut self,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        let (window, workspace) = self.focused_active_workspace(&outputs, cancelled, true)?;
        self.workspaces.remember_focus(workspace, &window);
        let enabled = !self.workspaces.uses_fullscreen_layout(workspace);
        self.workspaces.set_fullscreen_layout(workspace, enabled);
        self.pending_fullscreen_focus_restore.remove(&workspace);
        self.relayout_persist(workspace, &outputs, cancelled)
    }

    fn relayout_persist(
        &mut self,
        workspace: WorkspaceId,
        outputs: &macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        let layout = self.layout_workspace(
            workspace,
            output_for(&self.workspaces, workspace, outputs),
            cancelled,
        );
        self.persist_workspace_state_if_changed()?;
        Ok(self.status(layout.failed, outputs))
    }
}

fn drop_direction(point: (i32, i32), frame: crate::Rect) -> Option<Direction> {
    let distances = [
        (point.0 - frame.x, Direction::Left, frame.width),
        (
            frame.x + frame.width - point.0,
            Direction::Right,
            frame.width,
        ),
        (point.1 - frame.y, Direction::Up, frame.height),
        (
            frame.y + frame.height - point.1,
            Direction::Down,
            frame.height,
        ),
    ];
    distances
        .into_iter()
        .filter(|(distance, _, extent)| *distance <= *extent / 4)
        .min_by_key(|(distance, _, _)| *distance)
        .map(|(_, direction, _)| direction)
}
