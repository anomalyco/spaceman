//! `[labs] window_overlay`: while the chord is held, one transparent, non-activating panel over
//! each display outlines every visible window and draws a card on it. Only the cards take the
//! pointer, so a click on a card's button never activates Spaceman or moves focus, and
//! everything else passes through to the windows below. Never shows a window title.
//!
//! `frame.rs` draws the outlines, `card.rs` and `chips.rs` the cards, `place.rs` decides where
//! each card sits, and `look.rs` holds the palette and the motion.

mod card;
mod chips;
mod frame;
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
/// the cards' and outlines' own entrance carries the motion.
const REVEAL_FADE: Duration = Duration::from_millis(33);
/// Long enough for the cards' exit to play before the panel goes.
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
    /// The windows whose outline holds the pointer.
    frames: Vec<PhysicalWindowId>,
    /// The window whose card holds it.
    card: Option<PhysicalWindowId>,
    /// The control it is on, and that control's window.
    control: Option<(PhysicalWindowId, Control)>,
}

impl Hover {
    /// The pointer on `control` of `window`'s card.
    pub fn on_control(window: &PhysicalWindowId, control: Control) -> Self {
        Self {
            frames: Vec::new(),
            card: Some(window.clone()),
            control: Some((window.clone(), control)),
        }
    }

    /// The pointer on `window`, off its card.
    pub fn on_window(window: &PhysicalWindowId) -> Self {
        Self {
            frames: vec![window.clone()],
            ..Self::default()
        }
    }

    fn frame(&mut self, window: &PhysicalWindowId, inside: bool) {
        self.frames.retain(|held| held != window);
        if inside {
            self.frames.push(window.clone());
        }
    }

    fn card(&mut self, window: &PhysicalWindowId, inside: bool) {
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
    /// `windows` (tiles, then floats) whose outline holds it.
    #[must_use]
    pub fn window<'a>(
        &'a self,
        windows: &'a [UiWindowOverlayWindow],
    ) -> Option<&'a PhysicalWindowId> {
        self.card.as_ref().or_else(|| {
            windows
                .iter()
                .rev()
                .map(|window| &window.window)
                .find(|window| self.frames.contains(window))
        })
    }

    /// The control of `window`'s card the pointer is on.
    #[must_use]
    pub fn control_of(&self, window: &PhysicalWindowId) -> Option<Control> {
        self.control
            .as_ref()
            .filter(|(held, _)| held == window)
            .map(|(_, control)| *control)
    }

    /// Forgets windows that left the overlay: an element that goes never reports the pointer
    /// leaving it.
    fn keep(&mut self, windows: &[UiWindowOverlayWindow]) {
        let shown = |window: &PhysicalWindowId| windows.iter().any(|entry| &entry.window == window);
        self.frames.retain(|window| shown(window));
        self.card = self.card.take().filter(|window| shown(window));
        self.control = self.control.take().filter(|(window, _)| shown(window));
    }
}

/// How a built element answers the pointer: the live overlay's cards press and report hover,
/// a preview's are drawn only.
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

    /// A card: the one thing on the overlay that takes the pointer.
    fn card(self, element: El, window: &PhysicalWindowId) -> El {
        if !self.live {
            return element;
        }
        let window = window.clone();
        element
            .hit()
            .on_hover_change(move |state: &mut OverlayState, inside, tx| {
                state.hover.card(&window, inside);
                tx.color_motion(look::HOVER);
            })
    }

    /// A window's outline: hears the pointer, never takes it.
    fn frame(self, element: El, window: &PhysicalWindowId) -> El {
        if !self.live {
            return element;
        }
        let window = window.clone();
        element.on_hover_change(move |state: &mut OverlayState, inside, tx| {
            state.hover.frame(&window, inside);
            tx.color_motion(look::HOVER);
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
    /// Reduce Motion.
    pub still: bool,
    pub wiring: Wiring,
}

/// Every window's outline, then every card over them, in display-local points.
pub fn layers(scene: &Scene<'_>) -> Vec<El> {
    let display = scene.display;
    let hovered = scene.hover.window(&display.windows);
    let plans: Vec<card::Plan> = display
        .windows
        .iter()
        .map(|window| card::plan(window, display))
        .collect();
    let slots: Vec<place::Slot> = display
        .windows
        .iter()
        .zip(&plans)
        .map(|(window, plan)| place::Slot {
            window: window.frame,
            size: plan.size,
            front: window.floating,
        })
        .collect();
    let origins = place::place(display.screen, &slots);
    let outlines = display.windows.iter().enumerate().map(|(index, window)| {
        // A float is in front of every tile and of the floats listed before it.
        let covers: Vec<Rect> = display.windows[index + 1..]
            .iter()
            .filter(|other| other.floating)
            .map(|other| other.frame)
            .collect();
        let hot = hovered == Some(&window.window);
        frame::outline(window, display.screen, &covers, hot, scene.wiring)
    });
    let cards =
        display
            .windows
            .iter()
            .zip(plans.iter().zip(&origins))
            .map(|(window, (plan, origin))| {
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
                let parts = card::Parts {
                    display,
                    name: &name,
                    icons: scene.icons,
                    hot: scene.hover.control_of(&window.window),
                    plan: *plan,
                    origin: *origin,
                    still: scene.still,
                    wiring: scene.wiring,
                };
                card::card(window, parts)
            });
    outlines.chain(cards).collect()
}

/// One panel's state: the display it covers, while shown.
pub struct OverlayState {
    display: Option<UiWindowOverlayDisplay>,
    icons: Icons,
    names: HashMap<String, String>,
    reduce_motion: bool,
    hover: Hover,
    send: CommandSink,
}

impl OverlayState {
    fn send(&self, command: WindowOverlayCommand) {
        (self.send)(command);
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
}

#[derive(Clone)]
pub struct WindowOverlayDriver {
    panels: Arc<Mutex<Panels>>,
}

pub fn install<S: 'static>(app: &mut rune::App<S>, send: &CommandSink) -> WindowOverlayDriver {
    let panels = PANEL_NAMES
        .iter()
        .map(|&name| {
            let spec =
                OverlaySpec::new(name, (800.0, 600.0), overlay_view).options(OverlayOptions {
                    placement: Some(OverlayPlacement::FullScreen),
                    input: OverlayInput::Regions,
                    keys: KeyPolicy::None,
                    prewarm: false,
                    ..Default::default()
                });
            // A hidden display-sized panel would cost Mission Control its Space thumbnails, so
            // it holds nothing while hidden; the cards enter from their own pose.
            let spec = spec.animate_reveal().release_gpu_when_hidden();
            let state = OverlayState {
                display: None,
                icons: Icons::new(),
                names: HashMap::new(),
                reduce_motion: false,
                hover: Hover::default(),
                send: Arc::clone(send),
            };
            Panel {
                handle: app.overlay(spec, state),
                display: None,
            }
        })
        .collect();
    WindowOverlayDriver {
        panels: Arc::new(Mutex::new(Panels {
            panels,
            apps: apps::Catalog::default(),
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
        for (panel, display) in self.panels.iter_mut().zip(assigned) {
            let row = rows
                .iter()
                .find(|(row, _)| Some(&row.id) == display.as_ref())
                .cloned();
            let was = std::mem::replace(&mut panel.display, display);
            let retarget = was != panel.display;
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
                    state.hover.keep(&row.windows);
                    state.display = Some(row);
                    state.icons.extend(icons);
                    state.names.extend(names);
                    state.reduce_motion = reduce_motion;
                }),
                None if was.is_some() => panel.handle.update(move |state, _| {
                    // The cards and outlines leave as the panel fades.
                    state.display = None;
                    state.hover = Hover::default();
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
    fn the_pointer_is_on_the_card_it_is_over_else_the_frontmost_outline_holding_it() {
        // A tile with a float over it, in the order the worker lists them.
        let windows = [window(1, false), window(2, true)];
        let mut hover = Hover::default();
        assert_eq!(hover.window(&windows), None);
        // Inside both outlines: the float is in front.
        hover.frame(&id(1), true);
        hover.frame(&id(2), true);
        assert_eq!(hover.window(&windows), Some(&id(2)));
        // Off the float, still on the tile.
        hover.frame(&id(2), false);
        assert_eq!(hover.window(&windows), Some(&id(1)));
        // On the float's card, which was moved over the tile: the card wins.
        hover.card(&id(2), true);
        assert_eq!(hover.window(&windows), Some(&id(2)));
        // Leaving one card after entering the next, in either order, keeps the newer one.
        hover.card(&id(1), true);
        hover.card(&id(2), false);
        assert_eq!(hover.window(&windows), Some(&id(1)));
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
        // The float was sent to another workspace: only the tile is left.
        hover.keep(&[window(1, false)]);
        assert_eq!(hover, Hover::on_window(&id(1)));
    }
}
