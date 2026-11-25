//! The Try it step's miniature desktop, drawn: a small display holding a dark macOS desktop, with
//! a menu bar, two workspaces of two windows each (`window.rs`) and the Workspace HUD, which
//! shows for a moment after a switch as the real one does. It draws `Demo` and nothing else, so
//! every state is one build of the same elements and each change springs from where it is drawn.
//!
//! | What changes      | What moves                                                          | Spring |
//! | ----------------- | ------------------------------------------------------------------- | ------ |
//! | Another workspace | both workspaces slide; the one arriving grows to full size          | .38 / .12 |
//! |                   | the wallpaper's light drifts the same way, a little and later       | .60 / 0 |
//! |                   | the HUD rises in; its marker's leading edge, then its trailing edge | .16 / 0, .20 / 0, .30 / 0 |
//! | Float             | the focused window lifts to the middle, the other goes wide         | .36 / .14 |
//! | Fullscreen Layout | the focused window fills the screen, the other tucks behind         | .36 / .14 |
//! | Its number again  | the window behind comes forward over the one in front, which tucks | .36 / .14 |
//!
//! Under Reduce Motion nothing travels: the workspace and its windows cross-fade.

mod window;

use rune::{
    Color, El, Shadow, Spring, TextStyle, Transition, el, oklch, row, text,
    tok::motion::pair::SETTLE,
};

use super::demo::{self, APPS, Demo, Frame, Mode, Pose};
use crate::{Ui, apps, controls::Paint};
use window::Look;

/// The display's key: it takes the keyboard while the step is up.
pub const KEY: &str = "demo-stage";

pub const DISPLAY_W: f32 = 464.0;
const DISPLAY_H: f32 = 290.0;
const BEZEL: f32 = 6.0;
const BEZEL_RADIUS: f32 = 16.0;
const SCREEN_W: f32 = DISPLAY_W - 2.0 * BEZEL;
const SCREEN_H: f32 = DISPLAY_H - 2.0 * BEZEL;
/// Concentric with the bezel.
const SCREEN_RADIUS: f32 = BEZEL_RADIUS - BEZEL;
const MENU_H: f32 = 14.0;
/// From the screen's edge to a window, and between two windows.
const EDGE: f32 = 8.0;
const GAP: f32 = 8.0;
/// How much of the window behind shows above the one filling the screen, and how far it is
/// inset each side.
const PEEK: f32 = 5.0;
const PEEK_INSET: f32 = 12.0;

const SLIDE: Spring = Spring::new(0.38, 0.12);
const WINDOW: Spring = Spring::new(0.36, 0.14);
/// The wallpaper's light, which trails the workspaces it sits behind.
const DRIFT: Spring = Spring::new(0.6, 0.0);
const DRIFT_BY: f32 = 16.0;
const MARKER: Spring = Spring::new(0.24, 0.0);
const MARKER_LEAD: Spring = Spring::new(0.2, 0.0);
const MARKER_TRAIL: Spring = Spring::new(0.3, 0.0);
const HUD_IN: Spring = Spring::new(0.16, 0.0);
const HUD_OUT: Spring = Spring::new(0.28, 0.0);
const HUD_DROP: f32 = 5.0;
/// The workspace out of view, a little smaller and dimmer, so the one arriving grows in.
const AWAY_SCALE: f32 = 0.94;
const AWAY_OPACITY: f32 = 0.35;

const PILL_H: f32 = 24.0;
const PILL_EDGE: f32 = 9.0;
const PILL_REST: f32 = 10.0;
const GROUP_GAP: f32 = 12.0;
const NUMBER_W: f32 = 8.0;
const HUD_ICON: f32 = 11.0;
const HUD_GAP: f32 = 3.0;
const GROUP_W: f32 = NUMBER_W + 2.0 * (HUD_GAP + HUD_ICON);
const PILL_W: f32 = 2.0 * PILL_EDGE + 2.0 * GROUP_W + GROUP_GAP;
const MARKER_PAD: f32 = 6.0;
const MARKER_H: f32 = 18.0;
const DIM: f32 = 0.42;

/// What the stage shows.
#[derive(Clone, Copy, Debug)]
pub struct Scene {
    pub demo: Demo,
    /// The Workspace HUD is up, as it is for a moment after a switch.
    pub hud: bool,
    /// Reduce Motion: cross-fade instead of travelling.
    pub still: bool,
}

/// A window's rectangle on the screen from its unit frame, in a workspace laid out as `mode`.
#[must_use]
pub fn rect(frame: Frame, mode: Mode) -> (f32, f32, f32, f32) {
    let desk_w = SCREEN_W - 2.0 * EDGE + GAP;
    let desk_h = SCREEN_H - MENU_H - 2.0 * EDGE + GAP;
    let x = EDGE + frame.x * desk_w;
    let y = MENU_H + EDGE + frame.y * desk_h;
    let w = frame.w * desk_w - GAP;
    let h = frame.h * desk_h - GAP;
    match (frame.pose, mode) {
        // A tab of the window behind, above the one in front.
        (Pose::Behind, _) => (
            x + PEEK_INSET,
            y,
            w - 2.0 * PEEK_INSET,
            PEEK + window::RADIUS,
        ),
        (Pose::Tiled, Mode::Fullscreen) => (x, y + PEEK, w, h - PEEK),
        _ => (x, y, w, h),
    }
}

pub fn stage(ui: &Ui, p: Paint, scene: Scene) -> El {
    let Scene { demo, hud, still } = scene;
    let desk = if still {
        // One workspace at a time, fading into the next.
        workspace(ui, p, demo, demo.workspace(), still)
            .absolute(0.0, 0.0)
            .transition(Transition::opacity().springs(SETTLE))
    } else {
        el().key("strip")
            .absolute(-(demo.workspace() as f32) * SCREEN_W, 0.0)
            .size(2.0 * SCREEN_W, SCREEN_H)
            .motion(SLIDE)
            .children((0..APPS.len()).map(|index| {
                let showing = index == demo.workspace();
                workspace(ui, p, demo, index, still)
                    .absolute(index as f32 * SCREEN_W, 0.0)
                    .scale(if showing { 1.0 } else { AWAY_SCALE })
                    .scale_motion(SLIDE)
                    .opacity(if showing { 1.0 } else { AWAY_OPACITY })
                    .opacity_motion(SLIDE)
            }))
    };
    let screen = el()
        .key("screen")
        .size(SCREEN_W, SCREEN_H)
        .radius(SCREEN_RADIUS)
        .clip_rounded(SCREEN_RADIUS)
        .fill(oklch(0.15, 0.03, 272.0))
        .gradient(oklch(0.2, 0.04, 266.0), oklch(0.12, 0.03, 285.0))
        .child(wallpaper(demo.workspace(), still))
        .child(desk)
        .child(menu_bar(demo))
        .child(hud_pill(ui, p, demo, hud, still));
    let dark = Paint::new(true, 1.0 / p.hair);
    let mut display = el()
        .key(KEY)
        .role("group")
        .label("Practice desktop")
        .size(DISPLAY_W, DISPLAY_H)
        .padding(BEZEL)
        .radius(BEZEL_RADIUS)
        .fill(oklch(0.1, 0.004, 270.0))
        // The bezel catches the light along its top edge.
        .border_gradient(
            p.hair.max(0.5),
            Color::WHITE.alpha(0.3),
            Color::WHITE.alpha(0.07),
        );
    for shadow in &dark.k.elevation.overlay {
        display = display.box_shadow(Shadow {
            color: shadow
                .color
                .alpha(shadow.color.a * if p.k.dark { 1.0 } else { 0.4 }),
            ..*shadow
        });
    }
    display.child(screen)
}

/// The desktop picture: a deep slate night with an indigo light high on the left and a teal one
/// low on the right. The lights drift a little with the workspaces, behind them.
fn wallpaper(workspace: usize, still: bool) -> El {
    // Eased toward nothing, so a light has no edge.
    let glow = |color: Color, strength: f32| {
        [
            (0.0, color.alpha(strength)),
            (0.3, color.alpha(strength * 0.7)),
            (0.6, color.alpha(strength * 0.26)),
            (0.82, color.alpha(strength * 0.07)),
            (1.0, color.alpha(0.0)),
        ]
    };
    let light = |key: &str, center: [f32; 2], radius: [f32; 2], color: Color, strength: f32| {
        el().key(key)
            .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
            .radial_gradient(center, radius, glow(color, strength))
    };
    let drift = if still {
        0.0
    } else {
        -(workspace as f32) * DRIFT_BY
    };
    el().key("wallpaper")
        .absolute(-DRIFT_BY, 0.0)
        .size(SCREEN_W + 2.0 * DRIFT_BY, SCREEN_H)
        .offset(DRIFT_BY / 2.0 + drift, 0.0)
        .offset_motion(DRIFT)
        .a11y_hidden()
        .child(light(
            "indigo",
            [0.2, 0.16],
            [300.0, 210.0],
            oklch(0.5, 0.19, 277.0),
            0.62,
        ))
        .child(light(
            "teal",
            [0.86, 0.94],
            [280.0, 190.0],
            oklch(0.62, 0.12, 200.0),
            0.44,
        ))
        .child(light(
            "violet",
            [0.72, 0.08],
            [170.0, 110.0],
            oklch(0.56, 0.17, 320.0),
            0.2,
        ))
}

/// One workspace's windows. The focused window comes last, so it draws over the other while
/// it lifts off the grid and settles back, and comes forward when its number cycles to it.
fn workspace(ui: &Ui, p: Paint, demo: Demo, index: usize, still: bool) -> El {
    let desk = demo.desk(index);
    let frames = demo::frames(desk);
    let look = |slot: usize| Look {
        bundle: APPS[index][slot],
        frame: frames[slot],
        mode: desk.mode,
        focused: slot == desk.focus,
        first: slot == 0,
        still,
    };
    let behind = 1 - desk.focus;
    el().key(format!("ws{index}"))
        .size(SCREEN_W, SCREEN_H)
        .child(window::window(p, ui, look(behind)))
        .child(window::window(p, ui, look(desk.focus)))
}

/// An app's icon, or its initial on a quiet tile when this Mac doesn't have the app.
fn app_badge(ui: &Ui, bundle: &str, size: f32) -> El {
    match ui.apps.cached(bundle).and_then(|app| app.icon.as_ref()) {
        Some(pixels) => el().size(size, size).pixels(pixels),
        None => el()
            .size(size, size)
            .radius(size * 0.24)
            .fill(oklch(0.42, 0.02, 265.0))
            .center()
            .child(
                text(
                    apps::fallback_name(bundle)
                        .chars()
                        .next()
                        .map(String::from)
                        .unwrap_or_default(),
                )
                .font(TextStyle::new((size * 0.56).max(6.0), 650.0))
                .color(Color::WHITE.alpha(0.95)),
            ),
    }
}

/// The menu bar: the Apple menu, the focused app's name a touch brighter than its menus, and
/// the status items and clock at the far end. It is nearly a quarter past two.
fn menu_bar(demo: Demo) -> El {
    let ws = demo.workspace();
    let bundle = APPS[ws][demo.desk(ws).focus];
    let word = |key: &str, label: &str, alpha: f32, weight: f32| {
        text(label.to_owned())
            .key(key)
            .font(TextStyle::new(5.5, weight).lh(7.0))
            .color(Color::WHITE.alpha(alpha))
    };
    let name = apps::fallback_name(bundle);
    row()
        .key("menu-bar")
        .absolute(0.0, 0.0)
        .w(SCREEN_W)
        .h(MENU_H)
        .px(9.0)
        .gap(7.0)
        .center_y()
        .fill(Color::BLACK.alpha(0.26))
        .a11y_hidden()
        .child(
            el().key("apple")
                .size(6.0, 6.0)
                .radius(2.2)
                .fill(Color::WHITE.alpha(0.86)),
        )
        // The name cross-fades as focus moves to another app.
        .child(
            el().key("app-slot").child(
                word(&format!("name-{bundle}"), &name, 0.86, 650.0)
                    .transition(Transition::opacity().springs(SETTLE)),
            ),
        )
        .child(word("menu0", "File", 0.5, 450.0))
        .child(word("menu1", "Edit", 0.5, 450.0))
        .child(word("menu2", "View", 0.5, 450.0))
        .child(word("menu3", "Window", 0.5, 450.0))
        .child(el().key("space").grow(1.0))
        .child(
            el().key("status0")
                .size(5.0, 5.0)
                .radius(2.5)
                .fill(Color::WHITE.alpha(0.46)),
        )
        .child(
            el().key("battery")
                .size(10.0, 5.0)
                .radius(1.8)
                .border(1.0, Color::WHITE.alpha(0.46)),
        )
        .child(word("clock", "Tue 2:11 PM", 0.66, 500.0))
}

/// The Workspace HUD in miniature, in its Icons style: each workspace's number and apps, and a
/// marker that stretches to the one showing. Like the real one it shows after a switch and
/// leaves again.
fn hud_pill(ui: &Ui, p: Paint, demo: Demo, shown: bool, still: bool) -> El {
    let showing = demo.workspace();
    let group_x = |index: usize| PILL_EDGE + index as f32 * (GROUP_W + GROUP_GAP);
    let number = TextStyle {
        features: "'tnum' 1",
        ..TextStyle::new(10.0, 620.0).lh(12.0)
    };
    let mut marker = el()
        .key(if still {
            format!("marker{showing}")
        } else {
            "marker".to_owned()
        })
        .absolute(
            group_x(showing) - MARKER_PAD,
            (PILL_H - MARKER_H) / 2.0 - p.hair,
        )
        .size(GROUP_W + 2.0 * MARKER_PAD, MARKER_H)
        .radius(MARKER_H / 2.0)
        .fill(Color::WHITE.alpha(0.10))
        .border(p.hair, Color::WHITE.alpha(0.16))
        .transition(Transition::opacity().springs(SETTLE));
    if !still {
        marker = marker.motion(MARKER).lead(MARKER_LEAD, MARKER_TRAIL);
    }
    let coming = if shown { HUD_IN } else { HUD_OUT };
    let mut pill = el()
        .key("hud")
        .absolute((SCREEN_W - PILL_W) / 2.0, SCREEN_H - PILL_REST - PILL_H)
        .size(PILL_W, PILL_H)
        .radius(PILL_H / 2.0)
        .fill(oklch(0.2, 0.004, 285.0))
        .border(p.hair, Color::WHITE.alpha(0.14))
        .box_shadow(Shadow {
            y: 1.0,
            blur: 2.0,
            color: Color::BLACK.alpha(0.5),
            ..Shadow::default()
        })
        .box_shadow(Shadow {
            y: 6.0,
            blur: 16.0,
            color: Color::BLACK.alpha(0.5),
            ..Shadow::default()
        })
        .opacity(if shown { 1.0 } else { 0.0 })
        .opacity_motion(coming)
        .scale(if shown || still { 1.0 } else { 0.96 })
        .scale_motion(coming)
        .offset(0.0, if shown || still { 0.0 } else { HUD_DROP })
        .offset_motion(coming)
        .a11y_hidden()
        .child(marker);
    for (index, bundles) in APPS.iter().enumerate() {
        let mut group = row()
            .key(format!("group{index}"))
            .absolute(group_x(index), -p.hair)
            .size(GROUP_W, PILL_H)
            .gap(HUD_GAP)
            .center_y()
            .opacity(if index == showing { 1.0 } else { DIM })
            .opacity_motion(MARKER)
            .child(
                row().key("number").w(NUMBER_W).center().child(
                    text((index + 1).to_string())
                        .font(number)
                        .color(Color::WHITE.alpha(0.92)),
                ),
            );
        for bundle in bundles {
            group = group.child(app_badge(ui, bundle, HUD_ICON).key(format!("app-{bundle}")));
        }
        pill = pill.child(group);
    }
    pill
}
