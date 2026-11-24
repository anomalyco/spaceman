//! Scrolling Strip (a Lab) on the worker: the ribbon's rest layout, gliding the viewport
//! (`motion`), keeping columns out of view peeking or parked with the same durable parking that
//! hides inactive workspaces, following focus, and the strip's keys.
//!
//! Everything here is inert while `[labs] scrolling_strip` is off: [`WorkspaceRuntime::uses_strip`]
//! is false for every workspace, and the strip's actions fall through to the normal ones.

mod motion;

use motion::StripPlan;
pub(super) use motion::{MAX_PRESENT_LAG, StripCommand, StripViews, ViewportCamera};

use super::commands::{FOCUS_SETTLE, WarpContext};
use super::parking::RESTORE_CONFIRM_TOLERANCE;

/// How far a peeking column may sit from its sliver, in points, before it is put back.
const PEEK_TOLERANCE: i32 = 2;
use super::*;
use crate::{
    Placement,
    labs::LabFeature,
    motion::ScrollingDisplayBounds,
    strip::{
        ColumnRest, StripColumn, StripLayout, anchor_camera, clamp_camera, minimal_scroll,
        rest_layout, ribbon, ribbon_width,
    },
};

impl<B: WorkspaceBackend> WorkspaceRuntime<B> {
    /// Whether `workspace` is laid out as a strip: the Lab is on and so is its own switch.
    pub(super) fn uses_strip(&self, workspace: WorkspaceId) -> bool {
        self.config.labs().scrolling_strip && self.workspaces.uses_strip_layout(workspace)
    }

    /// Whether `window` is a column of a strip on a visible display. The strip owns where such
    /// a window is: in view, peeking, parked past a shared edge, or held at that edge mid-glide.
    /// None of that is the window's app un-parking it.
    pub(super) fn is_strip_column(&self, window: &crate::PhysicalWindowId) -> bool {
        self.workspaces
            .workspace_of(window)
            .is_some_and(|workspace| {
                self.uses_strip(workspace)
                    && self
                        .strip_outputs()
                        .is_some_and(|outputs| self.is_visible(workspace, outputs))
            })
    }

    /// Bound windows in strip order (the layout's left-to-right slot order).
    pub(super) fn strip_windows(&self, workspace: WorkspaceId) -> Vec<crate::PhysicalWindowId> {
        self.workspaces.engine(workspace).bound_windows()
    }

    /// The strip's rest layout in `output` with its camera aimed where it is going, or `None`
    /// outside strip mode.
    fn strip_plan(&self, workspace: WorkspaceId, output: crate::Rect) -> Option<StripPlan> {
        if !self.uses_strip(workspace) {
            return None;
        }
        let others: Vec<_> = self
            .strip_outputs()
            .map(|outputs| outputs.frames().filter(|frame| *frame != output).collect())
            .unwrap_or_default();
        let bounds = ScrollingDisplayBounds::new(output, others)
            .with_shared_edge(self.config.labs().scrolling_strip_shared_edge);
        let camera = self
            .strip_views
            .view(workspace)
            .camera
            .map(ViewportCamera::target);
        Some(self.plan_strip_in(workspace, output, bounds, camera))
    }

    fn plan_strip_in(
        &self,
        workspace: WorkspaceId,
        output: crate::Rect,
        bounds: ScrollingDisplayBounds,
        camera: Option<f64>,
    ) -> StripPlan {
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
        let ribbon = ribbon(&columns, output.width);
        let total = ribbon_width(&ribbon);
        let start = camera.map_or_else(
            || anchor_camera(&ribbon, state.anchor, output.width),
            |camera| clamp_camera(camera, output.width, total),
        );
        let active = self
            .workspaces
            .last_focused_slot(workspace)
            .and_then(|slot| ribbon.iter().find(|column| column.slot == slot))
            .copied();
        let target = active.map_or(start, |column| {
            minimal_scroll(start, output.width, column, total)
        });
        let layout = rest_layout(output, &bounds, &ribbon, target);
        StripPlan {
            output,
            bounds,
            ribbon,
            target,
            layout,
        }
    }

    /// The strip's rest layout in `output`, or `None` outside strip mode.
    pub(super) fn strip_layout(
        &self,
        workspace: WorkspaceId,
        output: crate::Rect,
    ) -> Option<StripLayout> {
        self.strip_plan(workspace, output).map(|plan| plan.layout)
    }

    /// A strip's columns in view, scaled into `area` for the HUD's preview.
    pub(super) fn strip_preview(
        &self,
        workspace: WorkspaceId,
        area: crate::Rect,
    ) -> Option<Vec<crate::Rect>> {
        if !self.uses_strip(workspace) {
            return None;
        }
        let bounds = ScrollingDisplayBounds::new(area, []);
        let plan = self.plan_strip_in(workspace, area, bounds, None);
        Some(
            plan.layout
                .visible()
                .map(|(_, frame)| {
                    let left = frame.x.max(area.x);
                    let right = (frame.x + frame.width).min(area.x + area.width);
                    crate::Rect {
                        x: left,
                        width: right - left,
                        ..frame
                    }
                })
                .collect(),
        )
    }

    /// The placements a visible workspace shows, as navigation sees them: Fullscreen Layout
    /// fills the display, and a strip shows only the columns in its viewport.
    pub(super) fn shown_placements(&self, resolved: &ResolvedWorkspace) -> Vec<Placement> {
        let (workspace, output) = (resolved.workspace, resolved.output);
        let settings = self.layout_settings(output);
        let mut placements = self.workspaces.engine(workspace).placements(&settings);
        if self.workspaces.uses_fullscreen_layout(workspace) {
            for placement in &mut placements {
                placement.frame = settings.single();
            }
        }
        if let Some(strip) = self.strip_layout(workspace, output) {
            placements.retain_mut(|placement| match strip.rest(placement.slot) {
                Some(ColumnRest::Visible(frame)) => {
                    placement.frame = frame;
                    true
                }
                _ => false,
            });
        }
        placements
    }

    /// A window that just joined a strip becomes its active column, so the viewport glides
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

    fn strip_window(
        &self,
        workspace: WorkspaceId,
        slot: crate::SlotId,
    ) -> Option<crate::PhysicalWindowId> {
        self.workspaces
            .engine(workspace)
            .model()
            .slot(slot)?
            .bound_window()
            .cloned()
    }

    /// Windows the strip keeps out of view in `output`, peeking or parked; empty outside strip
    /// mode.
    pub(super) fn strip_hidden_windows(
        &self,
        workspace: WorkspaceId,
        output: crate::Rect,
    ) -> HashSet<crate::PhysicalWindowId> {
        let Some(layout) = self.strip_layout(workspace, output) else {
            return HashSet::new();
        };
        layout
            .hidden()
            .filter_map(|slot| self.strip_window(workspace, slot))
            .collect()
    }

    /// Gives columns in view their frames and drops the rest, which peek or stay parked. While
    /// a glide moves the strip, the main thread owns its windows and nothing is written here.
    /// Records the viewport's anchor.
    pub(super) fn apply_strip_layout(
        &mut self,
        workspace: WorkspaceId,
        placements: &mut Vec<Placement>,
        output: crate::Rect,
    ) {
        let Some(plan) = self.strip_plan(workspace, output) else {
            return;
        };
        self.workspaces.strip_mut(workspace).anchor = plan.layout.anchor;
        if self.steer_strip(workspace, &plan) {
            placements.clear();
            return;
        }
        placements.retain_mut(|placement| match plan.layout.rest(placement.slot) {
            Some(ColumnRest::Visible(frame)) => {
                placement.frame = frame;
                true
            }
            _ => false,
        });
        let engine = self.workspaces.engine_mut(workspace);
        for (slot, frame) in plan.layout.visible() {
            engine.record_placement_target(slot, frame);
        }
    }

    /// Columns out of view are off-screen, so the snapshot leaves them out (as it does a column
    /// mostly past a free edge). Report them at their window-server frames so reconciliation
    /// keeps their slots; a window the window server no longer lists is really gone and is left
    /// out. Without a window-server snapshot nothing proves absence, so every parked column is
    /// kept. While the strip glides, the snapshots taken a moment apart disagree about where a
    /// column crossing an edge is, so every column the window server lists is kept.
    pub(super) fn observe_parked_strip_columns(
        &self,
        workspace: WorkspaceId,
        observation: &mut crate::NativeObservation,
        server_frames: Option<&HashMap<u32, crate::Rect>>,
    ) {
        if !self.uses_strip(workspace) {
            return;
        }
        let union = self.strip_outputs().map(macos::DisplayOutputs::union);
        let moving = self.strip_in_motion(workspace);
        for window in self.strip_windows(workspace) {
            let recovery = self.parked.get(&window).copied();
            let frame = match (server_frames, recovery) {
                (Some(frames), _) => match frames.get(&window.window_id) {
                    // A column the strip moved off screen, not one closed or minimized.
                    Some(frame)
                        if moving
                            || recovery.is_some()
                            || union.is_some_and(|union| !union.contains_center(*frame)) =>
                    {
                        *frame
                    }
                    _ => continue,
                },
                (None, Some(recovery)) => recovery,
                (None, None) => continue,
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

    /// Focus moved to a strip column by some other route (a click, the Dock, ⌘Tab onto a
    /// column out of view): make it the active column so the viewport glides to it. A focus
    /// command still settling wins over the stale window Accessibility may report meanwhile.
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

    /// Parked columns past an edge shared with another display, in `visible` strips at rest.
    /// Parking treats them like the windows of inactive workspaces: re-parked when their app
    /// drags them back on screen or a display change uncovers them. Peeking columns are kept
    /// at their sliver by [`Self::settle_strip_columns`] instead.
    pub(super) fn parked_strip_columns(
        &self,
        visible: &[ResolvedWorkspace],
    ) -> Vec<(WorkspaceId, crate::PhysicalWindowId)> {
        let mut parked = Vec::new();
        for resolved in visible {
            if self.strip_in_motion(resolved.workspace) {
                continue;
            }
            let Some(layout) = self.strip_layout(resolved.workspace, resolved.output) else {
                continue;
            };
            for (slot, rest) in &layout.columns {
                if *rest == ColumnRest::Parked
                    && let Some(window) = self.strip_window(resolved.workspace, *slot)
                    && self.parked.contains_key(&window)
                {
                    parked.push((resolved.workspace, window));
                }
            }
        }
        parked
    }

    /// With the strip off, every parked window of the workspace comes back: how turning the
    /// strip off reveals its columns.
    fn reveal_strip_columns(&mut self, workspace: WorkspaceId) -> usize {
        let mut failed = 0;
        for window in self.strip_windows(workspace) {
            if !self.parked.contains_key(&window) {
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

    /// Brings a resting strip's out-of-view columns to their rest: peeking columns to their
    /// sliver and parked ones off screen, each frame recorded durably first; and retires the
    /// recovery frames of columns the window server confirms back in view.
    pub(super) fn settle_strip_columns(
        &mut self,
        workspace: WorkspaceId,
        outputs: &macos::DisplayOutputs,
    ) -> usize {
        if self.strip_in_motion(workspace) {
            return 0;
        }
        let output = output_for(&self.workspaces, workspace, outputs);
        let Some(layout) = self.strip_layout(workspace, output) else {
            return 0;
        };
        let columns: Vec<_> = layout
            .columns
            .iter()
            .filter_map(|(slot, rest)| Some((self.strip_window(workspace, *slot)?, *rest)))
            .collect();
        let mut failed = 0;
        let mut retired = Vec::new();
        let mut to_park = Vec::new();
        for (window, rest) in columns {
            match rest {
                ColumnRest::Visible(frame) => {
                    if let Some(recovery) = self.parked.get(&window).copied()
                        && self
                            .backend
                            .window_server_frame(&window)
                            .is_some_and(|actual| {
                                actual
                                    .position_approximately_equals(frame, RESTORE_CONFIRM_TOLERANCE)
                            })
                    {
                        self.commit_unparked(&window);
                        retired.push((window, recovery));
                    }
                }
                ColumnRest::Peeking(frame) => match self.peek_strip_column(&window, frame, outputs)
                {
                    macos::ApplyOutcome::Applied => {}
                    macos::ApplyOutcome::Gone if self.confirm_gone(&window) => {
                        self.forget_window(workspace, &window);
                    }
                    _ => failed += 1,
                },
                // Not parked yet, or recorded when a glide took it out of view and still on
                // screen: a column leaving past a shared edge waits at the edge under its
                // neighbor until the glide ends, and parks now.
                ColumnRest::Parked
                    if !self.parked.contains_key(&window)
                        || self
                            .backend
                            .window_server_frame(&window)
                            .is_some_and(|frame| {
                                crate::geometry::shows_on(
                                    frame,
                                    &outputs.frames().collect::<Vec<_>>(),
                                )
                            }) =>
                {
                    to_park.push(window);
                }
                ColumnRest::Parked => {}
            }
        }
        self.persist_removals_or_rollback(retired);
        self.order_back_to_front(&mut to_park, |window| window.window_id);
        for window in to_park {
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

    /// Moves a column out of view to its sliver past a free edge, recording its on-screen
    /// frame durably first, like parking.
    fn peek_strip_column(
        &mut self,
        window: &crate::PhysicalWindowId,
        frame: crate::Rect,
        outputs: &macos::DisplayOutputs,
    ) -> macos::ApplyOutcome {
        if self.parking_exempt.contains(window) {
            return macos::ApplyOutcome::Applied;
        }
        let current = match self.backend.frame(window) {
            Ok(current) => self.backend.window_server_frame(window).unwrap_or(current),
            Err(macos::WindowAccessError::Gone) => return macos::ApplyOutcome::Gone,
            Err(macos::WindowAccessError::Unavailable) => {
                return macos::ApplyOutcome::Unavailable;
            }
        };
        // In place by position: apps that snap their size (Terminal's character cells) never
        // match the column's width exactly, and rewriting them every cycle would fight them.
        if (current.x - frame.x).abs() <= PEEK_TOLERANCE
            && (current.y - frame.y).abs() <= PEEK_TOLERANCE
        {
            return macos::ApplyOutcome::Applied;
        }
        let displays: Vec<_> = outputs.frames().collect();
        if !self.parked.contains_key(window)
            && crate::geometry::shows_on(current, &displays)
            && !self.mark_parked(window.clone(), current)
        {
            return macos::ApplyOutcome::Unavailable;
        }
        let resized = (current.width - frame.width).abs() > PEEK_TOLERANCE
            || (current.height - frame.height).abs() > PEEK_TOLERANCE;
        let outcome = if !resized {
            self.backend.set_position(window, frame)
        } else {
            // Resized since it was laid out: a stale width would hide the sliver or widen it.
            let placement = Placement {
                slot: crate::SlotId(0),
                window: Some(window.clone()),
                frame,
            };
            let results = self.backend.apply_managed(
                std::slice::from_ref(&placement),
                &Settings::for_output(outputs.main().frame),
                &AtomicBool::new(false),
            );
            match results.first().map(|result| result.outcome) {
                Some(macos::ApplyOutcome::Constrained) => macos::ApplyOutcome::Applied,
                Some(outcome) => outcome,
                None => macos::ApplyOutcome::Unavailable,
            }
        };
        log_event!(
            "strip peek: {:?} from {:?} to {:?} ({:?})",
            window,
            current,
            frame,
            outcome
        );
        if outcome == macos::ApplyOutcome::Applied {
            self.recently_parked.insert(window.clone(), Instant::now());
        }
        outcome
    }

    /// Lays out a visible workspace. A strip at rest gets its columns in view written and the
    /// rest peeking or parked; a strip whose camera moves hands its windows to a glide instead.
    /// With the Lab off this is exactly [`Self::layout_workspace`].
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
        let revealed = if self.uses_strip(workspace) {
            0
        } else {
            self.reveal_strip_columns(workspace)
        };
        let mut status = self.layout_workspace(workspace, output, cancelled);
        status.failed += revealed + self.settle_strip_columns(workspace, outputs);
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
        let workspace = self.strip_toggle_target(&outputs, cancelled)?;
        let enabled = !self.workspaces.uses_strip_layout(workspace);
        self.workspaces.set_strip_layout(workspace, enabled);
        self.strip_views.reset(workspace);
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

    /// The workspace the strip toggle acts on: the focused tile's when it is on screen, otherwise
    /// the workspace showing on the pointer's display (or the main display). Settings, a float,
    /// an unmanaged app or an empty desktop having focus never makes the shortcut a no-op.
    fn strip_toggle_target(
        &mut self,
        outputs: &macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<WorkspaceId, String> {
        let focused = self.current_focus();
        if let Some(window) = &focused
            && self.workspaces.workspace_of_any(window).is_none()
        {
            // A window that just opened may not be adopted yet.
            self.with_warp_context(WarpContext::Suppressed, |rt| {
                rt.cycle(outputs.clone(), cancelled)
            })?;
        }
        if let Some(window) = &focused
            && let Some(workspace) = self.workspaces.workspace_of_any(window)
            && self.is_visible(workspace, outputs)
        {
            if self.workspaces.workspace_of(window) == Some(workspace) {
                self.workspaces.remember_focus(workspace, window);
            }
            return Ok(workspace);
        }
        let display_id = self
            .backend
            .pointer_location()
            .and_then(|point| outputs.display_at(point))
            .unwrap_or_else(|| outputs.main())
            .id
            .clone();
        self.visible_workspaces(outputs)
            .into_iter()
            .find(|resolved| resolved.display_id == display_id)
            .map(|resolved| resolved.workspace)
            .ok_or_else(|| "No workspace is showing on this display".into())
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
        // Focus lands at once, so keys reach the column while it glides in on top
        // (`StripMotion::curtain` never tucks it under a neighbor).
        self.strip_views.focus_on_settle(workspace, None);
        match self.focus_tile(Some(origin.clone()), &target, outputs) {
            Ok(()) => Ok(status),
            // An app busy with the glide's writes can miss a focus; try again once it ends.
            Err(error) if self.strip_in_motion(workspace) => {
                log_event!("workspace {workspace}: {error} mid-glide; focusing once it settles");
                self.focus_strip_column_on_settle(workspace, origin, target);
                Ok(status)
            }
            Err(error) => Err(error),
        }
    }

    /// Makes `target` the focus every later key and focus check sees, and focuses it for real
    /// once the glide settles.
    fn focus_strip_column_on_settle(
        &mut self,
        workspace: WorkspaceId,
        origin: crate::PhysicalWindowId,
        target: crate::PhysicalWindowId,
    ) {
        let mut stale = self
            .pending_focus
            .take()
            .map(|pending| pending.stale)
            .unwrap_or_default();
        stale.push(Some(origin));
        // Counted from when the focus lands, so Accessibility's report of the column still
        // focused meanwhile reads as stale for the whole glide.
        let at = self
            .strip_views
            .settle_deadline(workspace)
            .unwrap_or_else(Instant::now);
        self.pending_focus = Some(super::commands::PendingFocus {
            target: target.clone(),
            stale,
            at,
        });
        self.strip_views.focus_on_settle(workspace, Some(target));
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
