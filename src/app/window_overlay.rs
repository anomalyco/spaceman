//! `[labs] window_overlay`, the worker's half: what the overlay shows over each visible window,
//! and its buttons, which act on that window whether or not it has focus. Title-free: bundle
//! IDs, frames, workspaces and window counts only.

use super::{commands::CommandOrigin, *};
use crate::navigation;

/// The most windows one display's overlay carries.
const MAX_WINDOWS: usize = 32;
/// The most apps a workspace names for a card's "send to" preview.
const MAX_WORKSPACE_APPS: usize = 4;
/// The sides a tile can swap toward, in the order a card shows them: ← ↑ ↓ →.
const SWAP_SIDES: [Direction; 4] = [
    Direction::Left,
    Direction::Up,
    Direction::Down,
    Direction::Right,
];

/// What the window overlay asks the worker for. Every one answers with a fresh
/// [`UiWindowOverlay`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WindowOverlayCommand {
    /// Only read what the overlay shows.
    Snapshot,
    /// Send the window to a workspace, focused or not.
    MoveTo {
        window: crate::PhysicalWindowId,
        workspace: WorkspaceId,
    },
    ToggleFloat(crate::PhysicalWindowId),
    TogglePin(crate::PhysicalWindowId),
    /// Fullscreen Layout for the window's workspace, with the window in front.
    ToggleFullscreen(crate::PhysicalWindowId),
    /// Swap the tile with its neighbour on that side, as `Option+Shift+H/J/K/L` does.
    Swap(crate::PhysicalWindowId, Direction),
}

/// Every visible window, by display, for the overlay's cards.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UiWindowOverlay {
    pub displays: Vec<UiWindowOverlayDisplay>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiWindowOverlayDisplay {
    /// The display's ID, as `DisplayOutput::id` gives it.
    pub id: String,
    /// The display's full bounds in global points (top-left origin), menu bar and Dock included:
    /// what a full-display overlay covers.
    pub screen: crate::Rect,
    /// The workspace it shows.
    pub workspace: WorkspaceId,
    /// That workspace is in Fullscreen Layout.
    pub fullscreen: bool,
    /// Every workspace a window can be sent to, in listing order.
    pub workspaces: Vec<UiWindowOverlayWorkspace>,
    /// Its visible windows: tiles in layout order (only the front one in Fullscreen Layout),
    /// then floats.
    pub windows: Vec<UiWindowOverlayWindow>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiWindowOverlayWorkspace {
    pub workspace: WorkspaceId,
    /// How many windows it holds, tiles and floats.
    pub windows: usize,
    /// The apps on it by bundle ID, once each, tiles first, at most four: what a window sent
    /// there would join. Never a title.
    pub apps: Vec<String>,
    /// It is assigned to this display.
    pub on_this_display: bool,
    /// Its display's number, as Settings counts them: 1 the main display, 2 the secondary, then
    /// every further display left to right.
    pub display: usize,
    /// Its display shows it now.
    pub visible: bool,
}

impl UiWindowOverlayWorkspace {
    /// It holds windows.
    #[must_use]
    pub const fn occupied(&self) -> bool {
        self.windows > 0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiWindowOverlayWindow {
    pub window: crate::PhysicalWindowId,
    /// For the app's icon and name; never a title.
    pub bundle_id: Option<String>,
    /// Where it is on screen, in global points.
    pub frame: crate::Rect,
    /// The workspace that owns it (a pin's home).
    pub workspace: WorkspaceId,
    pub floating: bool,
    pub pinned: bool,
    /// It has keyboard focus.
    pub focused: bool,
    /// The sides with a neighbouring tile to swap with, in the order a card shows them:
    /// left, up, down, right.
    pub swaps: Vec<Direction>,
}

/// Each display's number by its ID: 1 the main display, 2 the secondary, then every further
/// display left to right, as Settings › Workspaces counts them.
fn display_numbers(outputs: &macos::DisplayOutputs) -> HashMap<&str, usize> {
    let main = outputs.main().id.as_str();
    let secondary = outputs.secondary().map(|display| display.id.as_str());
    let rest = outputs
        .ordered()
        .into_iter()
        .map(|display| display.id.as_str())
        .filter(|id| *id != main && Some(*id) != secondary);
    std::iter::once(main)
        .chain(secondary)
        .chain(rest)
        .zip(1..)
        .collect()
}

impl<B: WorkspaceBackend> WorkspaceRuntime<B> {
    pub(super) fn window_overlay_command(
        &mut self,
        command: WindowOverlayCommand,
        outputs: macos::DisplayOutputs,
        cancelled: &AtomicBool,
    ) -> Result<CycleStatus, String> {
        let mut status = match command {
            WindowOverlayCommand::Snapshot => Ok(self.status(0, &outputs)),
            WindowOverlayCommand::MoveTo { window, workspace } => {
                self.move_window(window, workspace, outputs.clone(), cancelled)
            }
            WindowOverlayCommand::ToggleFloat(window) => {
                self.toggle_float_window(window, outputs.clone(), cancelled)
            }
            WindowOverlayCommand::TogglePin(window) => {
                self.toggle_pin_window(window, outputs.clone(), cancelled)
            }
            WindowOverlayCommand::ToggleFullscreen(window) => {
                let workspace = self.visible_tile_workspace(&window, &outputs)?;
                self.toggle_fullscreen_with(window, workspace, outputs.clone(), cancelled)
            }
            WindowOverlayCommand::Swap(window, side) => {
                let workspace = self.visible_tile_workspace(&window, &outputs)?;
                let origin = CommandOrigin::Passive;
                self.reorder_window(window, workspace, side, outputs.clone(), cancelled, origin)
            }
        }?;
        status.window_overlay = Some(self.window_overlay(&outputs));
        Ok(status)
    }

    fn visible_tile_workspace(
        &self,
        window: &crate::PhysicalWindowId,
        outputs: &macos::DisplayOutputs,
    ) -> Result<WorkspaceId, String> {
        self.workspaces
            .workspace_of(window)
            .filter(|workspace| self.is_visible(*workspace, outputs))
            .ok_or_else(|| "Window is not a tile on an active workspace".into())
    }

    /// What the overlay shows: each managed display's visible windows and the workspaces they
    /// can go to.
    pub(super) fn window_overlay(&mut self, outputs: &macos::DisplayOutputs) -> UiWindowOverlay {
        let visible = self.visible_workspaces(outputs);
        let tiles = self.navigation_tiles_for(&visible);
        // The full CG list retains bounds for hidden apps and ordered-out panels whose
        // AXWindow still exists. Only fresh on-screen evidence can produce a card.
        let frames = self.backend.onscreen_window_frames().unwrap_or_default();
        let focused = self.backend.focused_window();
        let displays = visible
            .iter()
            .filter(|resolved| self.scope.manages_selector(&resolved.selector))
            .take(HUD_MAX_DISPLAYS)
            .map(|resolved| {
                let focused = focused.as_ref();
                self.window_overlay_display(resolved, &visible, &tiles, &frames, focused, outputs)
            })
            .collect();
        UiWindowOverlay { displays }
    }

    fn window_overlay_display(
        &self,
        resolved: &ResolvedWorkspace,
        visible: &[ResolvedWorkspace],
        tiles: &[NavigationTile],
        frames: &HashMap<u32, crate::Rect>,
        focused: Option<&crate::PhysicalWindowId>,
        outputs: &macos::DisplayOutputs,
    ) -> UiWindowOverlayDisplay {
        let workspace = resolved.workspace;
        let fullscreen = self.workspaces.deck(workspace).is_monocle();
        let visible_frame = |window: &crate::PhysicalWindowId| {
            frames
                .get(&window.window_id)
                .copied()
                .filter(|frame| crate::geometry::shows_on(*frame, &[resolved.screen]))
        };
        let local: Vec<NavigationTile> = tiles
            .iter()
            .filter(|tile| tile.workspace == workspace)
            .cloned()
            .collect();
        // Fullscreen Layout stacks every tile in one place: only the visible one in front shows.
        let front = self
            .workspaces
            .preferred_focus(workspace)
            .filter(|front| {
                local
                    .iter()
                    .any(|tile| &tile.window == front && visible_frame(&tile.window).is_some())
            })
            .or_else(|| {
                local
                    .iter()
                    .find(|tile| visible_frame(&tile.window).is_some())
                    .map(|tile| tile.window.clone())
            });
        let entry =
            |window: &crate::PhysicalWindowId, frame, owner, floating| UiWindowOverlayWindow {
                window: window.clone(),
                bundle_id: self.bundle_id(&window.app).map(str::to_owned),
                frame,
                workspace: owner,
                floating,
                pinned: self.workspaces.is_pinned(window),
                focused: focused == Some(window),
                swaps: Vec::new(),
            };
        let mut windows: Vec<UiWindowOverlayWindow> = local
            .iter()
            .filter(|tile| !fullscreen || front.as_ref() == Some(&tile.window))
            .filter_map(|tile| {
                let mut card = entry(&tile.window, visible_frame(&tile.window)?, workspace, false);
                if !fullscreen {
                    card.swaps = SWAP_SIDES
                        .into_iter()
                        .filter(|side| navigation::target(&local, &tile.window, *side).is_some())
                        .collect();
                }
                Some(card)
            })
            .collect();
        // Floats it owns and pins from anywhere, with on-screen evidence on this display.
        // A pinned window straddling two displays gets one card, on the display holding its center.
        let owned_by_other_display = |frame: crate::Rect| {
            !resolved.screen.contains_center(frame)
                && outputs.displays().iter().any(|display| {
                    display.id != resolved.display_id && display.frame.contains_center(frame)
                })
        };
        let mut floats: Vec<_> = self
            .workspaces
            .floating_entries()
            .filter(|(window, owner)| *owner == workspace || self.workspaces.is_pinned(window))
            .filter_map(|(window, owner)| {
                let frame = visible_frame(window)?;
                if self.workspaces.is_pinned(window) && owned_by_other_display(frame) {
                    return None;
                }
                Some(entry(window, frame, owner, true))
            })
            .collect();
        floats.sort_by(|a, b| a.window.cmp(&b.window));
        windows.extend(floats);
        windows.truncate(MAX_WINDOWS);
        let numbers = display_numbers(outputs);
        let workspaces = self
            .config
            .ordered_workspaces()
            .filter(|listed| {
                self.workspaces.is_enabled(*listed)
                    && self
                        .scope
                        .manages_selector(self.workspaces.assignment(*listed))
            })
            .map(|listed| {
                let held = self.workspaces.windows(listed);
                let mut apps: Vec<String> = Vec::new();
                for bundle in held.iter().filter_map(|window| self.bundle_id(&window.app)) {
                    if apps.len() < MAX_WORKSPACE_APPS && !apps.iter().any(|app| app == bundle) {
                        apps.push(bundle.to_owned());
                    }
                }
                let display = outputs.resolve(self.workspaces.assignment(listed));
                UiWindowOverlayWorkspace {
                    workspace: listed,
                    windows: held.len(),
                    apps,
                    on_this_display: display.id == resolved.display_id,
                    display: numbers.get(display.id.as_str()).copied().unwrap_or(1),
                    visible: visible.iter().any(|shown| shown.workspace == listed),
                }
            })
            .collect();
        UiWindowOverlayDisplay {
            id: resolved.display_id.clone(),
            screen: resolved.screen,
            workspace,
            fullscreen,
            workspaces,
            windows,
        }
    }
}
