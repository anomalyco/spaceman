use super::*;

mod drag;
mod float;
mod sizing;
pub(super) use drag::DragSession;
#[cfg(test)]
pub(super) use drag::{DISPLAY_EDGE_DROP_BAND, display_edge_bands};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum WarpContext {
    /// Direct method call outside `execute_action` (unit tests).
    #[default]
    Direct,
    /// Explicit keyboard or `ctl` focus-moving action.
    Explicit,
    /// Passive cycle, gesture, swipe, or non-focus action: never warp.
    Suppressed,
}

pub(super) enum WorkspaceFocusOutcome {
    Focused(crate::PhysicalWindowId),
    Empty,
    Failed,
}

pub(super) const fn rect_center(rect: crate::Rect) -> (i32, i32) {
    (rect.x + rect.width / 2, rect.y + rect.height / 2)
}

impl<B: WorkspaceBackend> WorkspaceRuntime<B> {
    pub(super) fn with_warp_context<T>(
        &mut self,
        context: WarpContext,
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let previous = std::mem::replace(&mut self.warp_context, context);
        let result = f(self);
        self.warp_context = previous;
        result
    }

    pub(super) fn focus_window_frame(
        &mut self,
        window: &crate::PhysicalWindowId,
        workspace: WorkspaceId,
        output: crate::Rect,
    ) -> Option<crate::Rect> {
        // A tile that kept a different frame than it was given (a minimum
        // size, an aspect ratio) is where the window is, not its tile (#418).
        if let Some((_, accepted)) = self.constrained[workspace.index()].get(window) {
            return Some(*accepted);
        }
        let settings = self.layout_settings(output);
        let tile_frame = if self.workspaces.uses_fullscreen_layout(workspace)
            && self.workspaces.workspace_of(window) == Some(workspace)
        {
            Some(settings.single())
        } else {
            self.workspaces
                .engine(workspace)
                .placements(&settings)
                .into_iter()
                .find(|placement| placement.window.as_ref() == Some(window))
                .map(|placement| placement.frame)
        };
        tile_frame.or_else(|| {
            self.backend
                .window_server_frame(window)
                .or_else(|| self.backend.frame(window).ok())
        })
    }

    pub(super) fn follow_pointer_to(
        &mut self,
        target_display_id: &str,
        output: crate::Rect,
        focused_window: Option<crate::Rect>,
        outputs: &macos::DisplayOutputs,
    ) {
        let mode = self.config.mouse_follows_focus();
        if mode == crate::config::MouseFollowsFocus::Off
            || self.warp_context == WarpContext::Suppressed
            || self.backend.any_mouse_button_down()
        {
            return;
        }
        let target_rect = focused_window.unwrap_or(output);
        let target_display_id = focused_window
            .and_then(|frame| outputs.display_at(rect_center(frame)))
            .map_or(target_display_id, |display| display.id.as_str());
        let already_there = self
            .backend
            .pointer_location()
            .is_some_and(|(x, y)| match mode {
                crate::config::MouseFollowsFocus::Off => true,
                crate::config::MouseFollowsFocus::Display => outputs
                    .display_at((x, y))
                    .is_some_and(|display| display.id == target_display_id),
                crate::config::MouseFollowsFocus::Window => target_rect.contains(x, y),
            });
        if already_there {
            return;
        }
        let _: bool = self.backend.warp_pointer(rect_center(target_rect));
    }

    pub(super) fn follow_pointer_for_workspace(
        &mut self,
        workspace: WorkspaceId,
        output: crate::Rect,
        outcome: WorkspaceFocusOutcome,
        outputs: &macos::DisplayOutputs,
    ) {
        let target_display_id = outputs
            .resolve(self.workspaces.assignment(workspace))
            .id
            .as_str();
        match outcome {
            WorkspaceFocusOutcome::Focused(window) => {
                let frame = self.focus_window_frame(&window, workspace, output);
                self.follow_pointer_to(target_display_id, output, frame, outputs);
            }
            WorkspaceFocusOutcome::Empty => {
                self.follow_pointer_to(target_display_id, output, None, outputs);
            }
            WorkspaceFocusOutcome::Failed => {}
        }
    }
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
        let mut candidates = self.empty_display_targets(&outputs, &tiles);
        candidates.extend(tiles);
        let Some(target) = crate::navigation::target(&candidates, &window, direction) else {
            return Ok(self.status(0, &outputs));
        };
        self.with_warp_context(WarpContext::Suppressed, |rt| {
            rt.move_window(window, target.workspace, outputs, cancelled)
        })
    }

    /// A display whose workspace has no tiles offers no window to move toward,
    /// so its single-tile frame stands in for one. The stand-in's window id
    /// is never a real window; only its workspace is used.
    fn empty_display_targets(
        &self,
        outputs: &macos::DisplayOutputs,
        tiles: &[NavigationTile],
    ) -> Vec<NavigationTile> {
        self.visible_workspaces(outputs)
            .into_iter()
            .filter(|resolved| {
                self.scope.manages_selector(&resolved.selector)
                    && !tiles
                        .iter()
                        .any(|tile| tile.workspace == resolved.workspace)
            })
            .map(|resolved| NavigationTile {
                workspace: resolved.workspace,
                window: crate::PhysicalWindowId {
                    app: crate::AppInstanceId {
                        pid: -1,
                        launched_at: 0,
                    },
                    window_id: u32::try_from(resolved.workspace.index()).unwrap_or(u32::MAX),
                },
                frame: self.layout_settings(resolved.output).single(),
            })
            .collect()
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
        if !self.is_visible(workspace, &outputs) || self.parked.contains_key(&window) {
            return Ok(self.status(0, &outputs));
        }
        if self.workspaces.uses_fullscreen_layout(workspace) {
            return Ok(self.status(0, &outputs));
        }
        self.constrained[workspace.index()].remove(&window);
        self.applied[workspace.index()].remove(&window);
        let output = output_for(&self.workspaces, workspace, &outputs);
        let settings = self.layout_settings(output);
        let failed = if self
            .workspaces
            .engine_mut(workspace)
            .resize_window(&window, frame, &settings)
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

    pub(super) fn cycle_focus(
        &mut self,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        self.cycle_focus_by(outputs, cancelled, false)
    }

    pub(super) fn cycle_focus_previous(
        &mut self,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        self.cycle_focus_by(outputs, cancelled, true)
    }

    /// Steps through the workspace's tiles in stable order, then its floats,
    /// wrapping either way, so a workspace of floats (or one tile plus floats)
    /// can still be cycled. Focus outside that ring enters at the first window
    /// going forward and the last going backward.
    fn cycle_focus_by(
        &mut self,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
        backward: bool,
    ) -> Result<CycleStatus, String> {
        let (focused, workspace) = self.focused_owner_workspace(&outputs, cancelled)?;
        let windows = self.workspaces.windows(workspace);
        let len = windows.len();
        let target = match windows.iter().position(|window| window == &focused) {
            Some(_) if len < 2 => return Ok(self.status(0, &outputs)),
            Some(index) if backward => windows[(index + len - 1) % len].clone(),
            Some(index) => windows[(index + 1) % len].clone(),
            None => match if backward {
                windows.last()
            } else {
                windows.first()
            } {
                Some(target) => target.clone(),
                None => return Ok(self.status(0, &outputs)),
            },
        };
        self.focus_tile(Some(focused), &target, &outputs)?;
        Ok(self.status(0, &outputs))
    }

    fn focused_active_workspace(
        &mut self,
        outputs: &macos::DisplayOutputs,
        cancelled: &AtomicBool,
        refresh_if_unmanaged: bool,
    ) -> Result<(crate::PhysicalWindowId, WorkspaceId), String> {
        let window = self
            .current_focus()
            .ok_or("Focused window is unavailable")?;
        let workspace =
            self.active_workspace_of(&window, outputs, cancelled, refresh_if_unmanaged)?;
        Ok((window, workspace))
    }

    /// The visible workspace that owns the focused window, as a tile or a
    /// float, for commands that act on the whole workspace.
    fn focused_owner_workspace(
        &mut self,
        outputs: &macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<(crate::PhysicalWindowId, WorkspaceId), String> {
        let window = self
            .current_focus()
            .ok_or("Focused window is unavailable")?;
        if self.workspaces.workspace_of_any(&window).is_none() {
            self.with_warp_context(WarpContext::Suppressed, |rt| {
                rt.cycle(outputs.clone(), cancelled)
            })?;
        }
        let workspace = self
            .workspaces
            .workspace_of_any(&window)
            .ok_or("Focused window is not managed by Spaceman")?;
        if !self.is_visible(workspace, outputs) {
            return Err("Focused window is not on an active workspace".into());
        }
        Ok((window, workspace))
    }

    fn active_workspace_of(
        &mut self,
        window: &crate::PhysicalWindowId,
        outputs: &macos::DisplayOutputs,
        cancelled: &AtomicBool,
        refresh_if_unmanaged: bool,
    ) -> Result<WorkspaceId, String> {
        let workspace = match self.workspaces.workspace_of(window) {
            Some(workspace) => workspace,
            None if refresh_if_unmanaged => {
                self.with_warp_context(WarpContext::Suppressed, |rt| {
                    rt.cycle(outputs.clone(), cancelled)
                })?;
                self.workspaces
                    .workspace_of(window)
                    .ok_or("Focused window is not managed by Spaceman")?
            }
            None => return Err("Focused window is not managed by Spaceman".into()),
        };
        if !self.is_visible(workspace, outputs) {
            return Err("Focused window is not on an active workspace".into());
        }
        Ok(workspace)
    }

    pub(super) fn remember_fullscreen_focus(
        &mut self,
        workspace: WorkspaceId,
        window: &crate::PhysicalWindowId,
    ) -> Result<(), String> {
        if !self.workspaces.uses_fullscreen_layout(workspace) {
            return Ok(());
        }
        self.focus.settle_fullscreen(workspace);
        self.workspaces.remember_focus(workspace, window);
        self.persist_workspace_state_if_changed()
    }

    pub(super) fn toggle_layout(
        &mut self,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        let (_, workspace) = self.focused_owner_workspace(&outputs, cancelled)?;
        if self.workspaces.uses_fullscreen_layout(workspace) {
            self.workspaces.set_fullscreen_layout(workspace, false);
            self.focus.settle_fullscreen(workspace);
        }
        self.workspaces.engine_mut(workspace).toggle_axis();
        let axis = self.workspaces.engine(workspace).root_axis();
        let mut status = self.relayout_persist(workspace, &outputs, cancelled)?;
        status.hud = Some(self.hud_for(workspace, UiHudKind::Layout { axis }, None, &outputs));
        Ok(status)
    }

    pub(super) fn toggle_fullscreen_layout(
        &mut self,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        let (window, workspace) = self.focused_owner_workspace(&outputs, cancelled)?;
        self.workspaces.remember_focus(workspace, &window);
        let enabled = !self.workspaces.uses_fullscreen_layout(workspace);
        self.workspaces.set_fullscreen_layout(workspace, enabled);
        self.focus.settle_fullscreen(workspace);
        let mut status = self.relayout_persist(workspace, &outputs, cancelled)?;
        status.hud =
            Some(self.hud_for(workspace, UiHudKind::Fullscreen { enabled }, None, &outputs));
        Ok(status)
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
