//! The miniature desktop's windows: a dark macOS window in each app's own shape, with real,
//! whisper-small contents (`window/`), wearing the app's icon in its corner. A window's contents
//! are laid out by its frame, so when the frame springs to a new place they travel with it.
//!
//! Read closely, the four windows tell one afternoon: somebody has mounted a 90 lb speaker over
//! his chair on 15 lb anchors, and scheduled it to play at a quarter past two.

mod mail;
mod notes;
mod safari;
mod terminal;

use rune::{
    Color, El, Shadow, Spring, TextStyle, Transition, el, oklch, row, text,
    tok::motion::pair::SETTLE,
};

use super::{WINDOW, app_badge};
use crate::{
    Ui,
    controls::Paint,
    onboarding::demo::{Frame, Mode, Pose},
};

pub const RADIUS: f32 = 8.0;
const LIGHT: f32 = 5.0;
const TOOLBAR_H: f32 = 20.0;
const ICON: f32 = 26.0;
const ICON_INSET: f32 = 8.0;
/// How much an unfocused window gives up, so the focused one reads first: a little beside
/// it, more under a window floating over it.
const CALM: f32 = 0.2;
const COVERED: f32 = 0.34;
/// The window tucked behind one that fills the screen is a paler sheet's edge above it.
const TUCKED: f32 = 0.09;
const TUCK: Spring = Spring::new(0.1, 0.0);
/// Where a window's contents run past its bottom edge, they fade out over this much.
const FADE_H: f32 = 18.0;

/// The windows' type, as small as a real desktop's seen from across the room: a caption, the
/// running text, its strong cut, headings and the terminal's monospace.
const TINY: TextStyle = TextStyle::new(4.5, 500.0).lh(6.0);
const SMALL: TextStyle = TextStyle::new(5.5, 450.0).lh(7.5);
const STRONG: TextStyle = TextStyle::new(5.5, 620.0).lh(7.5);
const MONO: TextStyle = TextStyle::new(5.5, 450.0).lh(8.0).mono();

fn ink(alpha: f32) -> Color {
    Color::WHITE.alpha(alpha)
}

/// One line of words, faded rather than cut where it runs out of room.
fn words(key: &str, label: &str, style: TextStyle, color: Color) -> El {
    text(label.to_owned())
        .key(key)
        .font(style)
        .color(color)
        .truncate_fade()
}

/// Words that wrap to the width they are given.
fn prose(key: &str, label: &str, style: TextStyle, color: Color) -> El {
    text(label.to_owned())
        .key(key)
        .w_full()
        .font(style)
        .color(color)
        .wrap()
}

/// What a window shows and how it sits.
#[derive(Clone, Copy, Debug)]
pub struct Look {
    pub bundle: &'static str,
    pub frame: Frame,
    pub mode: Mode,
    /// The workspace's focused window.
    pub focused: bool,
    /// The left tile, which wears its icon in its left corner, away from its neighbour and
    /// clear of the Workspace HUD between them.
    pub first: bool,
    pub still: bool,
}

/// One window.
pub fn window(p: Paint, ui: &Ui, look: Look) -> El {
    let Look {
        bundle,
        frame,
        mode,
        focused,
        first,
        still,
    } = look;
    let (x, y, w, h) = super::rect(frame, mode);
    let lifted = frame.pose == Pose::Lifted;
    let behind = frame.pose == Pose::Behind;
    let lift = if lifted { 1.0 } else { 0.0 };
    let key = if still {
        // Under Reduce Motion a window fades into its new place.
        format!("w-{bundle}-{mode:?}-{:?}", frame.pose)
    } else {
        format!("w-{bundle}")
    };
    let veil = if behind {
        Color::WHITE.alpha(TUCKED)
    } else if focused {
        Color::BLACK.alpha(0.0)
    } else if mode == Mode::Floated {
        Color::BLACK.alpha(COVERED)
    } else {
        Color::BLACK.alpha(CALM)
    };
    let ring = if lifted {
        // Off the grid, a lifted window is edged by light instead of the tiles' focus ring.
        Color::WHITE.alpha(0.2)
    } else {
        oklch(0.82, 0.08, 250.0).alpha(if focused && !behind { 0.4 } else { 0.0 })
    };
    let cast = |y: f32, blur: f32, alpha: f32| Shadow {
        y,
        blur,
        color: Color::BLACK.alpha(alpha),
        ..Shadow::default()
    };
    let surface = surface(bundle);
    let mut window = el()
        .key(key)
        .absolute(x, y)
        .size(w, h)
        .radius(RADIUS)
        .clip_rounded(RADIUS)
        .fill(surface)
        // Lit from above: the rim is brightest along its top edge.
        .border_gradient(
            p.hair,
            ink(if focused && !behind {
                0.46
            } else if behind {
                0.36
            } else {
                0.2
            }),
            ink(if focused { 0.16 } else { 0.07 }),
        )
        .outline(1.0, 0.0, ring)
        // Shadow offsets never spring, so the resting and lifted casts trade strength.
        .box_shadow(cast(1.0, 2.0, 0.45))
        .box_shadow(cast(3.0, 8.0, 0.28 * (1.0 - lift)))
        .box_shadow(cast(4.0, 10.0, 0.38 * lift))
        .box_shadow(cast(14.0, 28.0, 0.46 * lift))
        .box_shadow(cast(28.0, 56.0, 0.36 * lift))
        .transition(Transition::opacity().springs(SETTLE))
        .a11y_hidden();
    if !still {
        window = window.motion(WINDOW);
    }
    let lit = focused && !behind;
    window
        // Tucked behind, a window is a plain sheet's edge: its contents fade, quickly, before
        // the sliver left of it closes over them, and come back as it opens.
        .child(
            chrome(bundle, lit, lifted)
                .opacity(if behind { 0.0 } else { 1.0 })
                .opacity_motion(if behind { TUCK } else { WINDOW }),
        )
        .child(
            el().key("fade")
                .pin(None, Some(0.0), Some(0.0), Some(0.0))
                .h(FADE_H)
                .fill(surface.alpha(0.0))
                .gradient(surface.alpha(0.0), surface),
        )
        .child(
            app_badge(ui, bundle, ICON)
                .key("icon")
                .pin(
                    first.then_some(ICON_INSET),
                    None,
                    (!first).then_some(ICON_INSET),
                    Some(ICON_INSET),
                )
                .opacity(if behind { 0.0 } else { 1.0 })
                .box_shadow(cast(2.0, 6.0, 0.4)),
        )
        .child(
            el().key("veil")
                .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
                .fill(veil),
        )
}

/// The window's own colour: a neutral charcoal, a shade apart for each app. The terminal is
/// the darkest and lets a little of the desktop through.
fn surface(bundle: &str) -> Color {
    match bundle {
        "com.apple.Terminal" => oklch(0.15, 0.008, 268.0).alpha(0.86),
        "com.apple.Notes" => oklch(0.225, 0.006, 80.0),
        _ => oklch(0.215, 0.007, 265.0),
    }
}

fn chrome(bundle: &str, focused: bool, lifted: bool) -> El {
    let body = match bundle {
        "com.apple.Safari" => safari::safari(focused, lifted),
        "com.apple.Terminal" => terminal::terminal(focused),
        "com.apple.Notes" => notes::notes(focused),
        _ => mail::mail(focused),
    };
    body.key("chrome").w_full().h_full()
}

fn lights(focused: bool) -> El {
    let colors = if focused {
        [
            oklch(0.70, 0.19, 25.0),
            oklch(0.83, 0.16, 85.0),
            oklch(0.74, 0.19, 145.0),
        ]
    } else {
        [ink(0.16); 3]
    };
    let mut lamps = row().key("lights").gap(3.5).center_y();
    for (index, color) in colors.into_iter().enumerate() {
        lamps = lamps.child(
            el().key(format!("light{index}"))
                .size(LIGHT, LIGHT)
                .radius(LIGHT / 2.0)
                .fill(color),
        );
    }
    lamps
}

/// A toolbar's row: the traffic lights, then `items`, over a hairline.
fn toolbar(focused: bool, items: Vec<El>) -> El {
    row()
        .key("toolbar")
        .w_full()
        .h(TOOLBAR_H)
        .px(8.0)
        .gap(6.0)
        .center_y()
        .fill(ink(0.045))
        .border_sides([0.0, 0.0, 1.0, 0.0], Color::BLACK.alpha(0.42))
        .child(lights(focused))
        .children(items)
}

fn glyph(key: &str) -> El {
    el().key(key).size(7.0, 6.0).radius(2.0).fill(ink(0.2))
}

fn space(key: &str) -> El {
    el().key(key).grow(1.0)
}
