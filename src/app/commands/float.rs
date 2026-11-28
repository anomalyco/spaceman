//! Toggle Float: residency transitions between a workspace's tiles and its floats.
//!
//! Floating a tile takes its slot and floats it in place; tiling a float adopts it into its
//! workspace's tiles directly ([`crate::workspace::Workspaces::adopt_tile`]). Neither drops the
//! window and runs a nested observation cycle hoping reconciliation adopts it again, so a
//! window's route or recall can never pull it to another workspace on the way.

use super::*;
use crate::config::DispositionPatch;
use crate::workspace::FloatCause;

const STAYS_FLOATING: &str = concat!(
    "Focused window stays floating: a window rule or its ",
    "application keeps it out of the tiling"
);

impl<B: WorkspaceBackend> WorkspaceRuntime<B> {
    /// Float the focused window above its workspace's tiles, or tile it again.
    ///
    /// Floating is workspace ownership, not a rule: a first float lifts the window
    /// to a centered card over its display ([`crate::geometry::centered_float`]),
    /// a later one restores where it last floated
    /// ([`crate::geometry::clamp_to`]), and either way it follows its workspace
    /// across switches, parks and restores with it, and reconciliation leaves it
    /// alone until the user toggles it back.
    pub(in crate::app) fn toggle_float(
        &mut self,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        let window = self
            .current_focus()
            .ok_or("Focused window is unavailable")?;
        self.toggle_float_window(window, outputs, cancelled)
    }

    /// [`Self::toggle_float`] for `window`, focused or not (the Window overlay Lab's card).
    pub(in crate::app) fn toggle_float_window(
        &mut self,
        window: crate::PhysicalWindowId,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        if u32::try_from(window.app.pid).is_ok_and(|pid| pid == std::process::id()) {
            // Spaceman's own Settings or setup window is never managed: nothing to float.
            return Ok(self.status(0, &outputs));
        }
        if self.workspaces.workspace_of_any(&window).is_none() {
            // A window observed for the first time may not be bound yet.
            self.cycle(outputs.clone(), cancelled)?;
        }
        let workspace = self
            .workspaces
            .workspace_of_any(&window)
            .ok_or("Focused window is not managed by Spaceman")?;
        if !self.is_visible(workspace, &outputs) {
            return Err("Focused window is not on an active workspace".into());
        }
        if self.workspaces.workspace_of(&window).is_some() {
            self.float_window(window, workspace, outputs, cancelled)
        } else {
            self.tile_window(window, workspace, outputs, cancelled)
        }
    }

    pub(in crate::app) fn float_window(
        &mut self,
        window: crate::PhysicalWindowId,
        workspace: WorkspaceId,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        let settings = self.layout_settings(output_for(&self.workspaces, workspace, &outputs));
        let target = self.float_target(&window, settings.usable());
        // Detach from the tiles first so the remaining windows re-fill its
        // space on the very next layout.
        self.forget_window_on(workspace, &window, settings.canvas());
        self.workspaces
            .float(window.clone(), workspace, FloatCause::Manual);
        let status = self.relayout_persist(workspace, &outputs, cancelled)?;
        if let Some(target) = target {
            // An app may refuse the size (a minimum, an aspect ratio, a sheet): its
            // position still lands, so an unconfirmed frame is no reason to stop
            // floating. Remember the float we asked for; the next toggle keeps it.
            let _: macos::ApplyOutcome = self.backend.set_float_frame(&window, target);
            self.float_frames.insert(window, target);
        }
        Ok(status)
    }

    /// Drop remembered float frames no workspace, parked record or omission
    /// still knows: a float forgotten by reconciliation never passes through
    /// [`Self::forget_window_on`].
    pub(in crate::app) fn prune_float_frames(&mut self) {
        self.float_frames.retain(|window, _| {
            self.workspaces.workspace_of_any(window).is_some()
                || self.parked.contains_key(window)
                || self.omitted_windows.contains_key(window)
        });
    }

    /// Where `window` floats on `area`: where it last floated (clamped onto the
    /// display it is shown on now), or a first float's centered lift over the
    /// frame it holds as a tile.
    fn float_target(
        &mut self,
        window: &crate::PhysicalWindowId,
        area: crate::Rect,
    ) -> Option<crate::Rect> {
        if let Some(remembered) = self.float_frames.get(window).copied() {
            return Some(crate::geometry::clamp_to(remembered, area));
        }
        let tiled = self
            .backend
            .frame(window)
            .ok()
            .or_else(|| self.backend.window_server_frame(window))?;
        Some(crate::geometry::centered_float(area, tiled))
    }

    fn tile_window(
        &mut self,
        window: crate::PhysicalWindowId,
        workspace: WorkspaceId,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        if let Some(reason) = self.keeps_floating(&window, &outputs) {
            return Err(reason.into());
        }
        self.tile_in_place(window, workspace, &outputs, cancelled)
    }

    /// Why a float cannot join the tiles even by hand: a float or pin rule, a built-in float
    /// default, or a fixed size. Reads the window afresh, since a sheet or title may have
    /// changed since the last cycle.
    pub(in crate::app) fn keeps_floating(
        &mut self,
        window: &crate::PhysicalWindowId,
        outputs: &macos::DisplayOutputs,
    ) -> Option<&'static str> {
        let Ok(observation) = self.backend.snapshot(0, outputs) else {
            return Some("Focused window is unavailable");
        };
        let observed = observation
            .applications
            .iter()
            .find_map(|application| match application {
                crate::AppObservation::Complete { app, windows, .. } => windows
                    .iter()
                    .find(|observed| &observed.id == window)
                    .map(|observed| (app.bundle_id.as_str(), observed)),
                _ => None,
            });
        let Some((bundle_id, observed)) = observed else {
            return Some("Focused window is unavailable");
        };
        let patch = self
            .config
            .patch(bundle_id, observed)
            .over(DispositionPatch::traits(observed.resizable, false));
        (patch.floats() || patch.is_ignored()).then_some(STAYS_FLOATING)
    }

    /// Adopts a window that holds no tile into `workspace`'s tiles, where its workspace would
    /// insert a new window, and lays them out.
    pub(in crate::app) fn tile_in_place(
        &mut self,
        window: crate::PhysicalWindowId,
        workspace: WorkspaceId,
        outputs: &macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        // Remember where the float sat, so the next Toggle Float restores it
        // instead of lifting it centered again.
        if self.workspaces.floating_workspace_of(&window).is_some()
            && let Some(frame) = self.backend.frame(&window).ok()
        {
            self.float_frames.insert(window.clone(), frame);
        }
        let output = output_for(&self.workspaces, workspace, outputs);
        let canvas = self.layout_settings(output).canvas();
        let anchor = self.workspaces.engine(workspace).insertion_focus();
        self.workspaces.adopt_tile(
            window.clone(),
            workspace,
            self.config.layout(),
            canvas,
            anchor,
        );
        if self.workspaces.deck(workspace).is_monocle() {
            self.workspaces.remember_focus(workspace, &window);
        }
        let mut status = self.relayout_persist(workspace, outputs, cancelled)?;
        if self.is_visible(workspace, outputs) && self.is_stacked_fullscreen(workspace) {
            status.hud = Some(self.hud_for(
                workspace,
                UiHudKind::Fullscreen { enabled: true },
                None,
                outputs,
            ));
        }
        // A restore still awaiting confirmation (a summon that has not landed) keeps its
        // recovery frame: make that the tile, not the frame it floated at.
        if self.parked.contains_key(&window)
            && let Some(tile_frame) = self.applied[workspace.index()]
                .get(&window)
                .map(|applied| applied.target)
                .or_else(|| self.backend.frame(&window).ok())
        {
            let _: bool = self.record_recovery(&window, tile_frame);
        }
        Ok(status)
    }
}
