//! `[labs] window_overlay`: while the chord is held, one transparent, non-activating panel over
//! each display dims one window, the one under the pointer or else the one with keyboard
//! focus, and draws that window's card on it. Every other window stays as it is. Only the card
//! takes the pointer, so a click on one of its buttons never activates Spaceman or moves focus,
//! and everything else passes through to the windows below. Never shows a window title.
//!
//! `dim.rs` draws the dim and hears the pointer, `card.rs` and `chips.rs` draw the card,
//! `place.rs` decides where it sits, and `look.rs` holds the palette and the motion.

mod card;
mod chips;
mod dim;
mod glyphs;
mod look;
mod place;
mod stage;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use rune::{
    Cursor, Cx, DisplayTarget, El, KeyPolicy, OverlayHandle, OverlayInput, OverlayOptions,
    OverlayPlacement, OverlaySpec, el,
};
use spaceman::{
    Direction, PhysicalWindowId, Rect, WorkspaceId,
    app::{
        UiWindowOverlayDisplay, UiWindowOverlayNews, UiWindowOverlayWindow, WindowOverlayCommand,
    },
};

pub(crate) use stage::{OverlayPreview, SCENES, stage};

use crate::{
    apps,
    hud::{self, Icons},
};

/// One panel per display, up to this many displays.
const PANEL_NAMES: [&str; 4] = [
    "Spaceman Window Overlay",
    "Spaceman Window Overlay 2",
    "Spaceman Window Overlay 3",
    "Spaceman Window Overlay 4",
];
/// A brief fade (>= 33 ms) so AppKit marks the panel visible before Rune's occlusion check;
/// the card's and the dim's own entrance carries the motion.
const REVEAL_FADE: Duration = Duration::from_millis(33);
/// Long enough for the card's exit to play before the panel goes.
const LEAVE_FADE: Duration = Duration::from_millis(130);

/// Sends a card's button press to the agent.
pub type CommandSink = Arc<dyn Fn(WindowOverlayCommand) + Send + Sync>;

/// A control on a card.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Control {
    Float,
    Pin,
    Fullscreen,
    Swap(Direction),
    Send(WorkspaceId),
}

/// Where the pointer is on a display's overlay.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Hover {
    /// The windows whose frame holds the pointer.
    frames: Vec<PhysicalWindowId>,
    /// The window whose card holds it. A press that began on the card keeps it held until the
    /// button comes up, wherever the pointer is dragged meanwhile.
    card: Option<PhysicalWindowId>,
    /// The control it is on, and that control's window.
    control: Option<(PhysicalWindowId, Control)>,
    /// The window the pointer was on as the overlay showed, until the pointer is first heard
    /// from: the first frame is already on the right window, not on the focused one.
    seed: Option<PhysicalWindowId>,
}

impl Hover {
    /// The pointer on `control` of `window`'s card.
    pub fn on_control(window: &PhysicalWindowId, control: Control) -> Self {
        Self {
            card: Some(window.clone()),
            control: Some((window.clone(), control)),
            ..Self::default()
        }
    }

    /// The pointer on `window`, off its card.
    pub fn on_window(window: &PhysicalWindowId) -> Self {
        Self {
            frames: vec![window.clone()],
            ..Self::default()
        }
    }

    /// The frontmost of `windows` whose frame holds `pointer` (global points), as the overlay
    /// shows.
    fn seeded(windows: &[UiWindowOverlayWindow], pointer: Option<(i32, i32)>) -> Self {
        let (x, y) = pointer.unwrap_or((i32::MIN, i32::MIN));
        let under = windows
            .iter()
            .rev()
            .find(|window| window.frame.contains(x, y));
        Self {
            seed: under.map(|window| window.window.clone()),
            ..Self::default()
        }
    }

    fn frame(&mut self, window: &PhysicalWindowId, inside: bool) {
        self.seed = None;
        self.frames.retain(|held| held != window);
        if inside {
            self.frames.push(window.clone());
        }
    }

    fn card(&mut self, window: &PhysicalWindowId, inside: bool) {
        self.seed = None;
        if inside {
            self.card = Some(window.clone());
        } else if self.card.as_ref() == Some(window) {
            self.card = None;
        }
    }

    fn control(&mut self, window: &PhysicalWindowId, control: Control, inside: bool) {
        let this = (window.clone(), control);
        if inside {
            self.control = Some(this);
        } else if self.control.as_ref() == Some(&this) {
            self.control = None;
        }
    }

    /// The window the pointer is on: the one whose card it is over, else the frontmost of
    /// `windows` (tiles, then floats) whose frame holds it.
    #[must_use]
    pub fn window<'a>(
        &'a self,
        windows: &'a [UiWindowOverlayWindow],
    ) -> Option<&'a PhysicalWindowId> {
        let framed = || {
            windows
                .iter()
                .rev()
                .map(|window| &window.window)
                .find(|window| self.frames.contains(window))
        };
        self.card.as_ref().or_else(framed).or(self.seed.as_ref())
    }

    /// The control of `window`'s card the pointer is on.
    #[must_use]
    pub fn control_of(&self, window: &PhysicalWindowId) -> Option<Control> {
        self.control
            .as_ref()
            .filter(|(held, _)| held == window)
            .map(|(_, control)| *control)
    }

    /// Forgets windows that left the overlay.
    fn keep(&mut self, windows: &[UiWindowOverlayWindow]) {
        let shown = |window: &PhysicalWindowId| windows.iter().any(|entry| &entry.window == window);
        self.frames.retain(|window| shown(window));
        self.card = self.card.take().filter(|window| shown(window));
        self.control = self.control.take().filter(|(window, _)| shown(window));
        self.seed = self.seed.take().filter(|window| shown(window));
    }
}

/// The one window of `windows` the overlay dims and draws a card on, as its place in the list:
/// the window the pointer is on, else the one with keyboard focus. With the pointer on a
/// window of another display (`elsewhere`), none: that display's overlay has the one.
#[must_use]
pub fn active(hover: &Hover, windows: &[UiWindowOverlayWindow], elsewhere: bool) -> Option<usize> {
    let at = |wanted: &PhysicalWindowId| windows.iter().position(|window| &window.window == wanted);
    match hover.window(windows) {
        Some(pointed) => at(pointed),
        None if elsewhere => None,
        None => windows.iter().position(|window| window.focused),
    }
}

/// How a built element answers the pointer: the live overlay's card presses and reports
/// hover, a preview's is drawn only.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wiring {
    /// One device pixel, in points.
    pub hair: f32,
    pub live: bool,
}

impl Wiring {
    /// A button: lights under the pointer, sinks while pressed, and sends `command`.
    fn control(
        self,
        element: El,
        window: &PhysicalWindowId,
        control: Control,
        command: WindowOverlayCommand,
    ) -> El {
        if !self.live {
            return element;
        }
        let window = window.clone();
        element
            .cursor(Cursor::Pointer)
            .press(look::white(0.08))
            .press_scale(look::PRESS_SCALE)
            .on_press_tx(move |state: &mut OverlayState, _| state.send(command.clone()))
            .on_hover_change(move |state: &mut OverlayState, inside, tx| {
                state.hover.control(&window, control, inside);
                tx.color_motion(look::HOVER);
            })
    }

    /// The card: the one thing on the overlay that takes the pointer. A press on its own
    /// surface does nothing but hold the card, so dragging off it keeps its window the one.
    fn card(self, element: El, window: &PhysicalWindowId) -> El {
        if !self.live {
            return element;
        }
        let window = window.clone();
        element
            .hit()
            .on_background_press(|_: &mut OverlayState| {})
            .on_hover_change(move |state: &mut OverlayState, inside, tx| {
                state.hover.card(&window, inside);
                state.share_pointer();
                tx.color_motion(look::HOVER);
            })
    }

    /// A window's frame: hears the pointer, never takes it.
    fn sense(self, element: El, window: &PhysicalWindowId) -> El {
        if !self.live {
            return element;
        }
        let window = window.clone();
        element.on_hover_change(move |state: &mut OverlayState, inside, _| {
            state.hover.frame(&window, inside);
            state.share_pointer();
        })
    }
}

/// One display's overlay, as the live panel and the Playground's stage both draw it.
#[derive(Clone, Copy)]
pub struct Scene<'a> {
    pub display: &'a UiWindowOverlayDisplay,
    pub icons: &'a Icons,
    pub names: &'a HashMap<String, String>,
    pub hover: &'a Hover,
    /// The pointer is on a window of another display.
    pub elsewhere: bool,
    /// Reduce Motion.
    pub still: bool,
    pub wiring: Wiring,
}

/// Where the pointer is heard for every window, then the one window's dim and its card over
/// it, in display-local points.
pub fn layers(scene: &Scene<'_>) -> Vec<El> {
    let display = scene.display;
    let mut layers: Vec<El> = display
        .windows
        .iter()
        .filter(|_| scene.wiring.live)
        .map(|window| dim::sense(window, display.screen, scene.wiring))
        .collect();
    let Some(index) = active(scene.hover, &display.windows, scene.elsewhere) else {
        return layers;
    };
    let window = &display.windows[index];
    // A float is in front of every tile and of the floats listed before it.
    let fronts: Vec<Rect> = display.windows[index + 1..]
        .iter()
        .filter(|other| other.floating)
        .map(|other| other.frame)
        .collect();
    let name = window
        .bundle_id
        .as_deref()
        .map_or_else(String::new, |bundle| {
            scene
                .names
                .get(bundle)
                .cloned()
                .unwrap_or_else(|| apps::fallback_name(bundle))
        });
    let plan = card::plan(window, display);
    let parts = card::Parts {
        display,
        name: &name,
        icons: scene.icons,
        hot: scene.hover.control_of(&window.window),
        origin: place::place(display.screen, window.frame, plan.size, &fronts),
        plan: &plan,
        still: scene.still,
        wiring: scene.wiring,
    };
    layers.push(dim::dim(window, display.screen, &fronts));
    layers.push(card::card(window, parts));
    layers
}

/// What every panel shares, so only one of them shows a card.
#[derive(Default)]
struct Shared {
    /// The panel, by its place in `peers`, whose display has the pointer on a window.
    pointing: Option<usize>,
    /// Every panel, and whether it is showing a display.
    peers: Vec<(OverlayHandle<OverlayState>, bool)>,
}

/// One panel's state: the display it covers, while shown.
pub struct OverlayState {
    display: Option<UiWindowOverlayDisplay>,
    icons: Icons,
    names: HashMap<String, String>,
    reduce_motion: bool,
    hover: Hover,
    /// This panel's place among the panels.
    index: usize,
    shared: Arc<Mutex<Shared>>,
    send: CommandSink,
}

impl OverlayState {
    fn send(&self, command: WindowOverlayCommand) {
        (self.send)(command);
    }

    /// Tells the other panels when the pointer reaches or leaves this display's windows: a
    /// panel falls back to its focused window only while the pointer is on no window anywhere.
    fn share_pointer(&mut self) {
        let here = self
            .display
            .as_ref()
            .is_some_and(|display| self.hover.window(&display.windows).is_some());
        let Ok(mut shared) = self.shared.lock() else {
            return;
        };
        let was = shared.pointing;
        if here {
            shared.pointing = Some(self.index);
        } else if was == Some(self.index) {
            shared.pointing = None;
        }
        if shared.pointing == was {
            return;
        }
        for (index, (peer, shown)) in shared.peers.iter().enumerate() {
            if index != self.index && *shown {
                // Nothing of the peer's changes but what its view reads from `shared`.
                peer.update(|_, _| {});
            }
        }
    }

    /// The pointer is on a window of another panel's display.
    fn pointer_elsewhere(&self) -> bool {
        self.shared
            .lock()
            .is_ok_and(|shared| shared.pointing.is_some_and(|panel| panel != self.index))
    }
}

struct Panel {
    handle: OverlayHandle<OverlayState>,
    /// The display it covers, by `UiWindowOverlayDisplay::id`, while shown.
    display: Option<String>,
}

struct Panels {
    panels: Vec<Panel>,
    apps: apps::Catalog,
    shared: Arc<Mutex<Shared>>,
}

#[derive(Clone)]
pub struct WindowOverlayDriver {
    panels: Arc<Mutex<Panels>>,
}

pub fn install<S: 'static>(app: &mut rune::App<S>, send: &CommandSink) -> WindowOverlayDriver {
    let shared = Arc::new(Mutex::new(Shared::default()));
    let panels: Vec<Panel> = PANEL_NAMES
        .iter()
        .enumerate()
        .map(|(index, &name)| {
            let spec =
                OverlaySpec::new(name, (800.0, 600.0), overlay_view).options(OverlayOptions {
                    placement: Some(OverlayPlacement::FullScreen),
                    input: OverlayInput::Regions,
                    keys: KeyPolicy::None,
                    prewarm: false,
                    ..Default::default()
                });
            // A hidden display-sized panel would cost Mission Control its Space thumbnails, so
            // it holds nothing while hidden; the card enters from its own pose.
            let spec = spec.animate_reveal().release_gpu_when_hidden();
            let state = OverlayState {
                display: None,
                icons: Icons::new(),
                names: HashMap::new(),
                reduce_motion: false,
                hover: Hover::default(),
                index,
                shared: Arc::clone(&shared),
                send: Arc::clone(send),
            };
            Panel {
                handle: app.overlay(spec, state),
                display: None,
            }
        })
        .collect();
    if let Ok(mut shared) = shared.lock() {
        shared.peers = panels
            .iter()
            .map(|panel| (panel.handle.clone(), false))
            .collect();
    }
    WindowOverlayDriver {
        panels: Arc::new(Mutex::new(Panels {
            panels,
            apps: apps::Catalog::default(),
            shared,
        })),
    }
}

impl WindowOverlayDriver {
    /// Call on the main thread, where displays, app icons and Reduce Motion are read.
    pub fn apply(&self, news: UiWindowOverlayNews) {
        let Ok(mut panels) = self.panels.lock() else {
            return;
        };
        match news {
            UiWindowOverlayNews::Show(overlay) => panels.show(overlay.displays),
            UiWindowOverlayNews::Hide => panels.show(Vec::new()),
        }
    }
}

/// Every app a display's overlay draws: its windows', and the ones on its workspaces.
fn bundles(display: &UiWindowOverlayDisplay) -> impl Iterator<Item = &String> {
    let windows = display
        .windows
        .iter()
        .filter_map(|window| window.bundle_id.as_ref());
    windows.chain(display.workspaces.iter().flat_map(|entry| &entry.apps))
}

impl Panels {
    /// Shows `displays`, one panel each, and hides every other panel.
    fn show(&mut self, displays: Vec<UiWindowOverlayDisplay>) {
        let connected = hud::display_ids();
        let reduce_motion = hud::reduce_motion();
        let rows: Vec<(UiWindowOverlayDisplay, u32)> = displays
            .into_iter()
            .filter_map(|display| {
                let id = *connected.get(&display.id)?;
                Some((display, id))
            })
            .take(PANEL_NAMES.len())
            .collect();
        let (icons, names) = self
            .apps
            .icons_and_names(rows.iter().flat_map(|(display, _)| bundles(display)));
        let current: Vec<Option<String>> = self
            .panels
            .iter()
            .map(|panel| panel.display.clone())
            .collect();
        let wanted: Vec<&str> = rows
            .iter()
            .map(|(display, _)| display.id.as_str())
            .collect();
        let assigned = hud::assign_panels(&current, &wanted);
        let pointer = spaceman::macos::mouse_position();
        for ((panel, display), index) in self.panels.iter_mut().zip(assigned).zip(0..) {
            let row = rows
                .iter()
                .find(|(row, _)| Some(&row.id) == display.as_ref())
                .cloned();
            let was = std::mem::replace(&mut panel.display, display);
            let retarget = was != panel.display;
            // As a panel shows, where the pointer already is: no hover has been heard yet.
            let seed = row
                .as_ref()
                .filter(|_| was.is_none())
                .map(|(row, _)| Hover::seeded(&row.windows, pointer));
            if let Ok(mut shared) = self.shared.lock() {
                if let Some((_, shown)) = shared.peers.get_mut(index) {
                    *shown = row.is_some();
                }
                if seed.as_ref().is_some_and(|seed| seed.seed.is_some()) {
                    shared.pointing = Some(index);
                }
            }
            let handle = panel.handle.clone();
            let (icons, names) = (icons.clone(), names.clone());
            match row {
                Some((row, native)) => panel.handle.update(move |state, _| {
                    if retarget {
                        let target = rune::NativeDisplayId::MacOS(native);
                        handle.set_display(DisplayTarget::Native(target));
                    }
                    if was.is_none() {
                        handle.fade_in(REVEAL_FADE);
                    }
                    if let Some(seed) = seed {
                        state.hover = seed;
                    }
                    state.hover.keep(&row.windows);
                    state.display = Some(row);
                    state.share_pointer();
                    state.icons.extend(icons);
                    state.names.extend(names);
                    state.reduce_motion = reduce_motion;
                }),
                None if was.is_some() => panel.handle.update(move |state, _| {
                    // The card and the dim leave as the panel fades.
                    state.display = None;
                    state.hover = Hover::default();
                    state.share_pointer();
                    handle.fade_out(LEAVE_FADE);
                }),
                None => {}
            }
        }
    }
}

fn overlay_view(state: &OverlayState, cx: &mut Cx<OverlayState>) -> El {
    if state.reduce_motion {
        cx.reduced_motion(true);
    }
    let wiring = Wiring {
        hair: 1.0 / cx.env().scale,
        live: true,
    };
    let still = cx.is_reduced_motion();
    let layers = state.display.as_ref().map(|display| {
        layers(&Scene {
            display,
            icons: &state.icons,
            names: &state.names,
            hover: &state.hover,
            elsewhere: state.pointer_elsewhere(),
            still,
            wiring,
        })
    });
    el().key("window-overlay")
        .w_full()
        .h_full()
        .children(layers.into_iter().flatten())
}

#[cfg(test)]
mod tests {
    use super::*;
    use spaceman::AppInstanceId;

    fn id(window_id: u32) -> PhysicalWindowId {
        PhysicalWindowId {
            app: AppInstanceId {
                pid: 7,
                launched_at: 1,
            },
            window_id,
        }
    }

    fn window(window_id: u32, floating: bool) -> UiWindowOverlayWindow {
        UiWindowOverlayWindow {
            window: id(window_id),
            bundle_id: None,
            frame: Rect {
                x: 0,
                y: 0,
                width: 800,
                height: 600,
            },
            workspace: WorkspaceId::ONE,
            floating,
            pinned: false,
            focused: false,
            swaps: Vec::new(),
        }
    }

    #[test]
    fn the_pointer_is_on_the_card_it_is_over_else_the_frontmost_frame_holding_it() {
        // A tile with a float over it, in the order the worker lists them.
        let windows = [window(1, false), window(2, true)];
        let mut hover = Hover::default();
        assert_eq!(hover.window(&windows), None);
        // Inside both frames: the float is in front.
        hover.frame(&id(1), true);
        hover.frame(&id(2), true);
        assert_eq!(hover.window(&windows), Some(&id(2)));
        // Off the float, still on the tile.
        hover.frame(&id(2), false);
        assert_eq!(hover.window(&windows), Some(&id(1)));
        // On the float's card, which sits over the tile: the card wins.
        hover.card(&id(2), true);
        assert_eq!(hover.window(&windows), Some(&id(2)));
        // Leaving one card after entering the next, in either order, keeps the newer one.
        hover.card(&id(1), true);
        hover.card(&id(2), false);
        assert_eq!(hover.window(&windows), Some(&id(1)));
    }

    #[test]
    fn one_window_is_active_the_pointers_else_the_focused_one() {
        let mut windows = [window(1, false), window(2, false), window(3, true)];
        windows[1].focused = true;
        let mut hover = Hover::default();
        // The pointer on the wallpaper or the menu bar: the window with keyboard focus.
        assert_eq!(active(&hover, &windows, false), Some(1));
        // On a window: that one, focused or not, and only that one.
        hover.frame(&id(1), true);
        assert_eq!(active(&hover, &windows, false), Some(0));
        // Sweeping onto the float over it moves the card there.
        hover.frame(&id(3), true);
        assert_eq!(active(&hover, &windows, false), Some(2));
        // On the float's card, off every frame (the card may sit beside its window), and while
        // a press that began on the card is dragged away: still the float.
        hover.card(&id(3), true);
        hover.frame(&id(3), false);
        hover.frame(&id(1), false);
        hover.frame(&id(2), true);
        assert_eq!(active(&hover, &windows, false), Some(2));
        // Off everything again: back to the focused window.
        hover.card(&id(3), false);
        hover.frame(&id(2), false);
        assert_eq!(active(&hover, &windows, false), Some(1));
        // With the pointer on a window of another display, this display shows no card at all,
        // and with no window focused here there is none to fall back to.
        assert_eq!(active(&hover, &windows, true), None);
        windows[1].focused = false;
        assert_eq!(active(&hover, &windows, false), None);
        // The pointer's own window wins whatever the other displays say.
        hover.frame(&id(1), true);
        assert_eq!(active(&hover, &windows, true), Some(0));
    }

    #[test]
    fn the_overlay_shows_on_the_window_already_under_the_pointer() {
        let mut windows = [window(1, false), window(2, true)];
        windows[0].focused = true;
        windows[1].frame.x = 400;
        // The pointer rests on the float as the overlay shows: the float, not the focused tile.
        let mut hover = Hover::seeded(&windows, Some((500, 300)));
        assert_eq!(active(&hover, &windows, false), Some(1));
        // Left of the float, on the tile alone.
        assert_eq!(
            active(&Hover::seeded(&windows, Some((100, 300))), &windows, false),
            Some(0)
        );
        // Once the pointer is heard from, what it says holds: here, that it is on no window.
        hover.frame(&id(2), false);
        assert_eq!(hover, Hover::default());
        assert_eq!(active(&hover, &windows, false), Some(0));
        // On the wallpaper, or nowhere known: the focused window.
        for pointer in [Some((5000, 300)), None] {
            let hover = Hover::seeded(&windows, pointer);
            assert_eq!(hover, Hover::default());
            assert_eq!(active(&hover, &windows, false), Some(0));
        }
    }

    #[test]
    fn a_control_is_hot_only_on_its_own_card_and_until_the_pointer_leaves_it() {
        let mut hover = Hover::default();
        let send = Control::Send(WorkspaceId::ONE);
        hover.control(&id(1), send, true);
        assert_eq!(hover.control_of(&id(1)), Some(send));
        assert_eq!(hover.control_of(&id(2)), None);
        // Sliding to the next chip: its enter may come before this one's leave.
        hover.control(&id(1), Control::Pin, true);
        hover.control(&id(1), send, false);
        assert_eq!(hover.control_of(&id(1)), Some(Control::Pin));
        hover.control(&id(1), Control::Pin, false);
        assert_eq!(hover.control_of(&id(1)), None);
    }

    #[test]
    fn a_window_that_leaves_the_overlay_takes_its_hover_with_it() {
        let mut hover = Hover::on_control(&id(2), Control::Float);
        hover.frame(&id(2), true);
        hover.frame(&id(1), true);
        // The float was sent to another workspace: only the tile is left, and it is the one.
        let left = [window(1, false)];
        hover.keep(&left);
        assert_eq!(hover, Hover::on_window(&id(1)));
        assert_eq!(active(&hover, &left, false), Some(0));
    }
}
