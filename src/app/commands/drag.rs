use super::super::input::PointerBounds;
use super::super::*;

/// Width inside the tiled area (in addition to any outer gap) of the display's
/// left and right full-height column drop bands.
pub(in crate::app) const DISPLAY_EDGE_DROP_BAND: i32 = 64;

#[derive(Clone, Debug)]
pub(in crate::app) struct DragSession {
    pub(in crate::app) window: crate::PhysicalWindowId,
    pub(in crate::app) workspace: WorkspaceId,
    pub(in crate::app) layout: crate::layout_tree::LayoutTree,
    pub(in crate::app) pointer_bounds: Option<PointerBounds>,
}

enum DropTarget {
    /// Dock into a full-height half-width column on the display's left or right edge.
    DisplayEdge(Direction),
    /// Swap with or nest beside another tile on the same workspace.
    LocalTile {
        target: crate::PhysicalWindowId,
        direction: Option<Direction>,
    },
    /// Move to another display's active workspace.
    OtherWorkspace(WorkspaceId),
    /// Return to the pre-drag layout without editing it.
    None,
}

impl<B: WorkspaceBackend> WorkspaceRuntime<B> {
    #[cfg(test)]
    pub(in crate::app) fn drop_window(
        &mut self,
        window: crate::PhysicalWindowId,
        point: (i32, i32),
        preview: bool,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        self.drop_window_tracked(window, point, None, preview, outputs, cancelled)
    }

    pub(in crate::app) fn drop_window_tracked(
        &mut self,
        window: crate::PhysicalWindowId,
        point: (i32, i32),
        bounds: Option<PointerBounds>,
        preview: bool,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        let Some(source) = self.workspaces.workspace_of(&window) else {
            self.drag_session = None;
            return self.cycle(outputs, cancelled);
        };
        if !self.is_visible(source, &outputs) || self.parked.contains_key(&window) {
            self.drag_session = None;
            return Ok(self.status(0, &outputs));
        }
        if self.workspaces.uses_fullscreen_layout(source) || self.uses_strip(source) {
            self.drag_session = None;
            return Ok(self.status(0, &outputs));
        }
        self.constrained[source.index()].remove(&window);
        self.applied[source.index()].remove(&window);

        let (pre_drag_layout, effective_bounds) =
            self.ensure_drag_session(&window, source, bounds, preview);
        let source_output = output_for(&self.workspaces, source, &outputs);
        let source_settings = self.layout_settings(source_output);
        let target = self.resolve_drop_target(
            &window,
            source,
            point,
            source_output,
            source_settings.usable(),
            effective_bounds,
            &outputs,
        );

        match target {
            DropTarget::OtherWorkspace(target) => {
                if preview {
                    let failed = self.apply_drag_preview(
                        source,
                        &pre_drag_layout,
                        &source_settings,
                        &window,
                        cancelled,
                    );
                    return Ok(self.status(failed, &outputs));
                }
                self.with_warp_context(super::WarpContext::Suppressed, |rt| {
                    rt.move_window(window, target, outputs, cancelled)
                })
            }
            DropTarget::DisplayEdge(direction) => {
                let mut candidate = pre_drag_layout.clone();
                let changed = self
                    .workspaces
                    .engine(source)
                    .slot_for_window(&window)
                    .is_some_and(|slot| candidate.create_implicit_split(slot, direction));
                self.finish_local_drop(
                    source,
                    &window,
                    &pre_drag_layout,
                    candidate,
                    changed,
                    preview,
                    source_output,
                    &source_settings,
                    &outputs,
                    cancelled,
                )
            }
            DropTarget::LocalTile { target, direction } => {
                let mut candidate = pre_drag_layout.clone();
                let engine = self.workspaces.engine(source);
                let changed = match (
                    engine.slot_for_window(&window),
                    engine.slot_for_window(&target),
                ) {
                    (Some(moved), Some(target_slot)) => match direction {
                        Some(direction) => candidate.move_relative(moved, target_slot, direction),
                        None => candidate.swap(moved, target_slot),
                    },
                    _ => false,
                };
                self.finish_local_drop(
                    source,
                    &window,
                    &pre_drag_layout,
                    candidate,
                    changed,
                    preview,
                    source_output,
                    &source_settings,
                    &outputs,
                    cancelled,
                )
            }
            DropTarget::None => {
                if preview {
                    let failed = self.apply_drag_preview(
                        source,
                        &pre_drag_layout,
                        &source_settings,
                        &window,
                        cancelled,
                    );
                    return Ok(self.status(failed, &outputs));
                }
                let layout = self.layout_workspace(source, source_output, cancelled);
                self.persist_workspace_state_if_changed()?;
                Ok(self.status(layout.failed, &outputs))
            }
        }
    }

    fn ensure_drag_session(
        &mut self,
        window: &crate::PhysicalWindowId,
        source: WorkspaceId,
        bounds: Option<PointerBounds>,
        preview: bool,
    ) -> (crate::layout_tree::LayoutTree, Option<PointerBounds>) {
        let session = match self.drag_session.take() {
            Some(mut session) if session.window == *window && session.workspace == source => {
                if let Some(bounds) = bounds {
                    match &mut session.pointer_bounds {
                        Some(existing) => existing.merge(bounds),
                        None => session.pointer_bounds = Some(bounds),
                    }
                }
                session
            }
            _ => DragSession {
                window: window.clone(),
                workspace: source,
                layout: self.workspaces.engine(source).layout().clone(),
                pointer_bounds: bounds,
            },
        };
        let result = (session.layout.clone(), session.pointer_bounds);
        if preview {
            self.drag_session = Some(session);
        }
        result
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_drop_target(
        &self,
        window: &crate::PhysicalWindowId,
        source: WorkspaceId,
        point: (i32, i32),
        source_output: crate::Rect,
        usable: crate::Rect,
        bounds: Option<PointerBounds>,
        outputs: &macos::DisplayOutputs,
    ) -> DropTarget {
        if let Some(direction) = display_edge_drop(point, source_output, usable, bounds) {
            return DropTarget::DisplayEdge(direction);
        }
        if let Some(tile) = self
            .navigation_tiles(outputs)
            .into_iter()
            .find(|tile| tile.window != *window && tile.frame.contains(point.0, point.1))
        {
            return if tile.workspace != source {
                DropTarget::OtherWorkspace(tile.workspace)
            } else {
                DropTarget::LocalTile {
                    direction: drop_direction(point, tile.frame),
                    target: tile.window,
                }
            };
        }
        if let Some(target) = self
            .visible_workspaces(outputs)
            .into_iter()
            .find(|resolved| resolved.output.contains(point.0, point.1))
            .map(|resolved| resolved.workspace)
            && target != source
        {
            return DropTarget::OtherWorkspace(target);
        }
        DropTarget::None
    }

    #[allow(clippy::too_many_arguments)]
    fn finish_local_drop(
        &mut self,
        source: WorkspaceId,
        window: &crate::PhysicalWindowId,
        pre_drag_layout: &crate::layout_tree::LayoutTree,
        candidate: crate::layout_tree::LayoutTree,
        changed: bool,
        preview: bool,
        source_output: crate::Rect,
        source_settings: &Settings,
        outputs: &macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        if preview {
            let shown = if changed { &candidate } else { pre_drag_layout };
            let failed = self.apply_drag_preview(source, shown, source_settings, window, cancelled);
            return Ok(self.status(failed, outputs));
        }
        if changed {
            self.workspaces
                .engine_mut(source)
                .commit_layout_tree(candidate);
        }
        let layout = self.layout_workspace(source, source_output, cancelled);
        self.persist_workspace_state_if_changed()?;
        Ok(self.status(layout.failed, outputs))
    }

    /// Preview `layout` on sibling tiles without mutating the committed engine
    /// state or poisoning readback tracking with transient preview frames.
    fn apply_drag_preview(
        &mut self,
        workspace: WorkspaceId,
        layout: &crate::layout_tree::LayoutTree,
        settings: &Settings,
        dragged: &crate::PhysicalWindowId,
        cancelled: &AtomicBool,
    ) -> usize {
        let placements = self.workspaces.engine(workspace).placements_for_tree(
            layout,
            settings,
            &self.minimum_sizes,
        );
        let results =
            self.backend
                .apply_managed_during_drag(&placements, settings, dragged, cancelled);
        failed_results(&results)
    }
}

/// Left and right full-height edge bands on `output`. Each band runs the full
/// height of `output`, includes the outer gap on that edge, and lies strictly
/// inside `output`.
pub(in crate::app) fn display_edge_bands(
    output: crate::Rect,
    usable: crate::Rect,
) -> (crate::Rect, crate::Rect) {
    let inset = DISPLAY_EDGE_DROP_BAND.min(usable.width / 6).max(0);
    let left_end = (usable.x + inset).clamp(output.x, output.x + output.width);
    let right_start = (usable.x + usable.width - inset).clamp(output.x, output.x + output.width);
    let left = crate::Rect {
        x: output.x,
        y: output.y,
        width: left_end - output.x,
        height: output.height,
    };
    let right = crate::Rect {
        x: right_start,
        y: output.y,
        width: output.x + output.width - right_start,
        height: output.height,
    };
    (left, right)
}

fn display_edge_drop(
    point: (i32, i32),
    output: crate::Rect,
    usable: crate::Rect,
    bounds: Option<PointerBounds>,
) -> Option<Direction> {
    if !output.contains(point.0, point.1) {
        return None;
    }
    let (left, right) = display_edge_bands(output, usable);
    if left.contains(point.0, point.1) {
        let armed = bounds.is_none_or(|bounds| !bounds.entirely_inside(left));
        return armed.then_some(Direction::Left);
    }
    if right.contains(point.0, point.1) {
        let armed = bounds.is_none_or(|bounds| !bounds.entirely_inside(right));
        return armed.then_some(Direction::Right);
    }
    None
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
