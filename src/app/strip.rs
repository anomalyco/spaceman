//! Scrolling Strip (a Lab) on the worker: which columns show, parking the rest with the same
//! durable parking that hides inactive workspaces, following focus, and the strip's keys.
//!
//! Everything here is inert while `[labs] scrolling_strip` is off: [`WorkspaceRuntime::uses_strip`]
//! is false for every workspace, and the strip's actions fall through to the normal ones.

use super::commands::FOCUS_SETTLE;
use super::*;
use crate::{
    Placement,
    labs::LabFeature,
    strip::{StripColumn, StripLayout, compute_strip_layout},
};

impl<B: WorkspaceBackend> WorkspaceRuntime<B> {
    /// Whether `workspace` is laid out as a strip: the Lab is on and so is its own switch.
    pub(super) fn uses_strip(&self, workspace: WorkspaceId) -> bool {
        self.config.labs().scrolling_strip && self.workspaces.uses_strip_layout(workspace)
    }

    /// Bound windows in strip order (the layout's left-to-right slot order).
    pub(super) fn strip_windows(&self, workspace: WorkspaceId) -> Vec<crate::PhysicalWindowId> {
        self.workspaces.engine(workspace).bound_windows()
    }

    /// The strip's viewport in `output`, or `None` outside strip mode.
    pub(super) fn strip_layout(
        &self,
        workspace: WorkspaceId,
        output: crate::Rect,
    ) -> Option<StripLayout> {
        if !self.uses_strip(workspace) {
            return None;
        }
        let engine = self.workspaces.engine(workspace);
        let state = self.workspaces.strip(workspace);
        let columns = engine
            .layout()
            .slots()
            .into_iter()
            .filter(|slot| {
                engine
                    .model()
                    .slot(*slot)
                    .is_some_and(|slot| slot.bound_window().is_some())
            })
            .map(|slot| StripColumn {
                slot,
                width: state.width(slot),
            })
            .collect::<Vec<_>>();
        Some(compute_strip_layout(
            output,
            &columns,
            self.workspaces.last_focused_slot(workspace),
            state.anchor,
        ))
    }

    /// The placements a visible workspace shows, as navigation sees them: Fullscreen Layout
    /// fills the display, and a strip shows only its viewport's columns.
    pub(super) fn shown_placements(&self, resolved: &ResolvedWorkspace) -> Vec<Placement> {
        let (workspace, output) = (resolved.workspace, resolved.output);
        let mut placements = self
            .workspaces
            .engine(workspace)
            .placements(&Settings::for_output(output));
        if self.workspaces.uses_fullscreen_layout(workspace) {
            for placement in &mut placements {
                placement.frame = output;
            }
        }
        if let Some(strip) = self.strip_layout(workspace, output) {
            placements.retain_mut(|placement| {
                strip
                    .visible
                    .iter()
                    .find(|(slot, _)| *slot == placement.slot)
                    .map(|(_, frame)| placement.frame = *frame)
                    .is_some()
            });
        }
        placements
    }

    /// A window that just joined a strip becomes its active column, so the viewport opens
    /// onto it.
    pub(super) fn focus_inserted_strip_column(
        &mut self,
        workspace: WorkspaceId,
        decisions: &[Decision],
    ) {
        if !self.uses_strip(workspace) {
            return;
        }
        let inserted = decisions.iter().rev().find_map(|decision| match decision {
            Decision::WindowInserted { window, .. }
                if self.workspaces.workspace_of(window) == Some(workspace) =>
            {
                Some(window.clone())
            }
            _ => None,
        });
        if let Some(window) = inserted {
            self.workspaces.remember_focus(workspace, &window);
        }
    }

    /// Windows the strip keeps off-screen in `output`; empty outside strip mode.
    pub(super) fn strip_hidden_windows(
        &self,
        workspace: WorkspaceId,
        output: crate::Rect,
    ) -> HashSet<crate::PhysicalWindowId> {
        let Some(layout) = self.strip_layout(workspace, output) else {
            return HashSet::new();
        };
        let model = self.workspaces.engine(workspace).model();
        layout
            .hidden
            .iter()
            .filter_map(|slot| model.slot(*slot)?.bound_window().cloned())
            .collect()
    }

    /// Gives visible columns their viewport frames and drops the rest, which stay parked. A
    /// column whose restore the window server has not confirmed yet waits for it too, so a
    /// layout write never races the restore. Records the viewport's anchor.
    pub(super) fn apply_strip_layout(
        &mut self,
        workspace: WorkspaceId,
        placements: &mut Vec<Placement>,
        output: crate::Rect,
    ) {
        let Some(layout) = self.strip_layout(workspace, output) else {
            return;
        };
        self.workspaces.strip_mut(workspace).anchor = layout.anchor;
        let parked = &self.parked;
        placements.retain_mut(|placement| {
            let Some((_, frame)) = layout
                .visible
                .iter()
                .find(|(slot, _)| *slot == placement.slot)
            else {
                return false;
            };
            placement.frame = *frame;
            placement
                .window
                .as_ref()
                .is_some_and(|window| !parked.contains_key(window))
        });
        let engine = self.workspaces.engine_mut(workspace);
        for (slot, frame) in layout.visible {
            engine.record_placement_target(slot, frame);
        }
    }

    /// Parked columns are off-screen, so the snapshot leaves them out. Report them at their
    /// window-server frames so reconciliation keeps their slots; a window the window server no
    /// longer lists is really gone and is left out. Without a window-server snapshot nothing
    /// proves absence, so every parked column is kept.
    pub(super) fn observe_parked_strip_columns(
        &self,
        workspace: WorkspaceId,
        observation: &mut crate::NativeObservation,
        server_frames: Option<&HashMap<u32, crate::Rect>>,
    ) {
        if !self.uses_strip(workspace) {
            return;
        }
        for window in self.strip_windows(workspace) {
            let Some(recovery) = self.parked.get(&window).copied() else {
                continue;
            };
            let frame = match server_frames {
                Some(frames) => match frames.get(&window.window_id) {
                    Some(frame) => *frame,
                    None => continue,
                },
                None => recovery,
            };
            for application in &mut observation.applications {
                if let crate::AppObservation::Complete { app, windows, .. } = application
                    && app.id == window.app
                    && !windows.iter().any(|observed| observed.id == window)
                {
                    windows.push(crate::ObservedWindow {
                        id: window.clone(),
                        frame,
                        resizable: true,
                        title: None,
                    });
                }
            }
        }
    }

    /// Focus moved to a strip column by some other route (a click, the Dock, ⌘Tab onto a parked
    /// column): make it the active column so the viewport slides to it. A focus command still
    /// settling wins over the stale window Accessibility may report meanwhile.
    pub(super) fn follow_strip_focus(
        &mut self,
        visible: &[ResolvedWorkspace],
        focused: Option<&crate::PhysicalWindowId>,
    ) {
        let Some(focused) = focused else {
            return;
        };
        if self.pending_focus.as_ref().is_some_and(|pending| {
            pending.at.elapsed() < FOCUS_SETTLE && &pending.target != focused
        }) {
            return;
        }
        let Some(workspace) = self.workspaces.workspace_of(focused) else {
            return;
        };
        if self.uses_strip(workspace)
            && visible
                .iter()
                .any(|resolved| resolved.workspace == workspace)
        {
            self.workspaces.remember_focus(workspace, focused);
        }
    }

    /// Parked columns outside the viewports of `visible` strips. Parking treats them like the
    /// windows of inactive workspaces: re-parked when their app drags them back on screen or a
    /// display change uncovers them.
    pub(super) fn parked_strip_columns(
        &self,
        visible: &[ResolvedWorkspace],
    ) -> Vec<(WorkspaceId, crate::PhysicalWindowId)> {
        visible
            .iter()
            .flat_map(|resolved| {
                self.strip_hidden_windows(resolved.workspace, resolved.output)
                    .into_iter()
                    .filter(|window| self.parked.contains_key(window))
                    .map(move |window| (resolved.workspace, window))
            })
            .collect()
    }

    /// Restores parked columns now inside the viewport. Outside strip mode every parked window
    /// of the workspace comes back, which is how turning the strip off reveals its columns.
    fn reveal_strip_columns(
        &mut self,
        workspace: WorkspaceId,
        hidden: &HashSet<crate::PhysicalWindowId>,
    ) -> usize {
        let mut failed = 0;
        for window in self.strip_windows(workspace) {
            if hidden.contains(&window) || !self.parked.contains_key(&window) {
                continue;
            }
            match self.restore_window(&window) {
                macos::ApplyOutcome::Applied => {}
                macos::ApplyOutcome::Gone => self.forget_window(workspace, &window),
                _ => failed += 1,
            }
        }
        failed
    }

    /// Parks columns outside the viewport, recording each original frame durably first.
    pub(super) fn park_strip_columns(
        &mut self,
        workspace: WorkspaceId,
        outputs: &macos::DisplayOutputs,
    ) -> usize {
        let output = output_for(&self.workspaces, workspace, outputs);
        let mut hidden: Vec<_> = self
            .strip_hidden_windows(workspace, output)
            .into_iter()
            .filter(|window| !self.parked.contains_key(window))
            .collect();
        self.order_back_to_front(&mut hidden, |window| window.window_id);
        let mut failed = 0;
        for window in hidden {
            match self.park_window(&window, output, outputs) {
                macos::ApplyOutcome::Applied => {}
                // Only the window server or the process exiting proves a window gone.
                macos::ApplyOutcome::Gone if self.confirm_gone(&window) => {
                    self.forget_window(workspace, &window);
                }
                _ => failed += 1,
            }
        }
        failed
    }

    /// Lays out a visible workspace, revealing incoming strip columns before the layout write
    /// and parking outgoing ones after it, so the display is never left bare. With the Lab off
    /// this is exactly [`Self::layout_workspace`].
    pub(super) fn layout_strip_workspace(
        &mut self,
        workspace: WorkspaceId,
        outputs: &macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> LayoutStatus {
        let output = output_for(&self.workspaces, workspace, outputs);
        if !self.config.labs().scrolling_strip {
            return self.layout_workspace(workspace, output, cancelled);
        }
        let hidden = self.strip_hidden_windows(workspace, output);
        let revealed = self.reveal_strip_columns(workspace, &hidden);
        let mut status = self.layout_workspace(workspace, output, cancelled);
        status.failed += revealed + self.park_strip_columns(workspace, outputs);
        status
    }

    /// `⌥⇧S` / `spaceman ctl strip`: the focused workspace becomes a strip, or stops being one.
    pub(super) fn toggle_strip(
        &mut self,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        if !self.config.labs().scrolling_strip {
            return Err(LabFeature::ScrollingStrip.off_message());
        }
        let (window, workspace) = self.focused_active_workspace(&outputs, cancelled, true)?;
        self.workspaces.remember_focus(workspace, &window);
        let enabled = !self.workspaces.uses_strip_layout(workspace);
        self.workspaces.set_strip_layout(workspace, enabled);
        if enabled {
            self.workspaces.strip_mut(workspace).anchor = None;
        }
        self.pending_fullscreen_focus_restore.remove(&workspace);
        log_event!(
            "workspace {workspace}: Scrolling Strip {}",
            if enabled { "on" } else { "off" }
        );
        let layout = self.layout_strip_workspace(workspace, &outputs, cancelled);
        self.persist_workspace_state_if_changed()?;
        Ok(self.status(layout.failed, &outputs))
    }

    /// The strip's meaning for the focus, move, resize and cycle keys, when the focused window
    /// is on a visible strip. `None` hands the action to the normal layout.
    pub(super) fn strip_action(
        &mut self,
        action: &WorkerAction,
        outputs: &macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Option<Result<CycleStatus, String>> {
        if !self.config.labs().scrolling_strip
            || !matches!(
                action,
                WorkerAction::Focus(_)
                    | WorkerAction::Reorder(_)
                    | WorkerAction::Resize(_)
                    | WorkerAction::CycleFocus
                    | WorkerAction::Balance
            )
        {
            return None;
        }
        let focused = self.current_focus()?;
        let workspace = self.workspaces.workspace_of(&focused)?;
        if !self.uses_strip(workspace) || !self.is_visible(workspace, outputs) {
            return None;
        }
        let windows = self.strip_windows(workspace);
        let index = windows.iter().position(|window| window == &focused)?;
        let step = |direction| match direction {
            Direction::Left => index.checked_sub(1),
            Direction::Right => Some(index + 1).filter(|next| *next < windows.len()),
            Direction::Up | Direction::Down => None,
        };
        Some(match action {
            WorkerAction::Focus(Direction::Up | Direction::Down) => return None,
            WorkerAction::Focus(direction) => {
                // Past either end of the strip, the next display's tiles.
                let target = windows[step(*direction)?].clone();
                self.focus_strip_column(workspace, focused, target, outputs, cancelled)
            }
            WorkerAction::CycleFocus if windows.len() > 1 => {
                let target = windows[(index + 1) % windows.len()].clone();
                self.focus_strip_column(workspace, focused, target, outputs, cancelled)
            }
            WorkerAction::Reorder(direction) => match step(*direction) {
                Some(target) => {
                    self.workspaces
                        .engine_mut(workspace)
                        .swap_windows(&focused, &windows[target]);
                    self.relayout_strip(workspace, &focused, outputs, cancelled)
                }
                None => Ok(self.status(0, outputs)),
            },
            WorkerAction::Resize(direction) => {
                self.resize_strip_column(workspace, &focused, *direction, outputs, cancelled)
            }
            _ => Ok(self.status(0, outputs)),
        })
    }

    fn focus_strip_column(
        &mut self,
        workspace: WorkspaceId,
        origin: crate::PhysicalWindowId,
        target: crate::PhysicalWindowId,
        outputs: &macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        let status = self.relayout_strip(workspace, &target, outputs, cancelled)?;
        self.focus_tile(Some(origin), &target)?;
        Ok(status)
    }

    fn resize_strip_column(
        &mut self,
        workspace: WorkspaceId,
        focused: &crate::PhysicalWindowId,
        direction: ResizeDirection,
        outputs: &macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        let Some(slot) = self.workspaces.engine(workspace).slot_for_window(focused) else {
            return Ok(self.status(0, outputs));
        };
        let strip = self.workspaces.strip_mut(workspace);
        let width = strip.width(slot);
        let next = match direction {
            ResizeDirection::Grow => width.wider(),
            ResizeDirection::Shrink => width.narrower(),
        };
        strip.set_width(slot, next);
        log_event!(
            "workspace {workspace}: strip column {} -> {}%",
            slot.0,
            next.percent()
        );
        self.relayout_strip(workspace, focused, outputs, cancelled)
    }

    /// Makes `active` the strip's active column and lays the strip out around it.
    fn relayout_strip(
        &mut self,
        workspace: WorkspaceId,
        active: &crate::PhysicalWindowId,
        outputs: &macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        self.workspaces.remember_focus(workspace, active);
        let layout = self.layout_strip_workspace(workspace, outputs, cancelled);
        self.persist_workspace_state_if_changed()?;
        Ok(self.status(layout.failed, outputs))
    }
}
