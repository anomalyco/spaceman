//! The Try it step's miniature desktop, drawn: a small dark display with a menu bar, two
//! workspaces of two windows each and a Workspace HUD capsule resting on its bottom edge. It
//! draws `Demo` and nothing else, so every state is one build of the same elements and each
//! change springs from where it is drawn.
//!
//! | What changes          | What moves                                                    | Spring |
//! | --------------------- | ------------------------------------------------------------- | ------ |
//! | Another workspace     | both workspaces slide; the one arriving grows to full size    | .36 / .12 |
//! |                       | the HUD marker's leading edge, then its trailing edge         | .20 / 0, .30 / 0 |
//! | Float                 | the focused window lifts to the middle, the other goes wide   | .34 / .16 |
//! | Fullscreen Layout     | the focused window fills the screen, the other tucks behind   | .34 / .16 |
//! | The last move is made | a ring round the display shows, then widens and fades         | .12 / 0, .70 / 0 |
//!
//! Under Reduce Motion nothing travels: the workspace and its windows cross-fade.

use rune::{
    Color, El, Shadow, Spring, TextStyle, Transition, el, row, text, tok::motion::pair::SETTLE,
};

use super::demo::{self, APPS, Demo, Frame, Mode, Pose};
use crate::{
    Ui, apps,
    controls::Paint,
    hud::{rgba, srgb},
};

/// The display's key: it takes the keyboard while the step is up.
pub const KEY: &str = "demo-stage";

const DISPLAY_W: f32 = 424.0;
const DISPLAY_H: f32 = 252.0;
const BEZEL: f32 = 5.0;
const BEZEL_RADIUS: f32 = 15.0;
const SCREEN_W: f32 = DISPLAY_W - 2.0 * BEZEL;
const SCREEN_H: f32 = DISPLAY_H - 2.0 * BEZEL;
/// Concentric with the bezel.
const SCREEN_RADIUS: f32 = BEZEL_RADIUS - BEZEL;
const MENU_H: f32 = 13.0;
/// From the screen's edge to a window, and between two windows.
const EDGE: f32 = 6.0;
const GAP: f32 = 6.0;
/// How much of the window behind shows above the one filling the screen, and how far it is
/// inset each side.
const PEEK: f32 = 6.0;
const PEEK_INSET: f32 = 14.0;
const WINDOW_RADIUS: f32 = 7.0;
const LIGHT: f32 = 4.0;
const ICON: f32 = 34.0;

const SLIDE: Spring = Spring::new(0.36, 0.12);
const WINDOW: Spring = Spring::new(0.34, 0.16);
const MARKER: Spring = Spring::new(0.24, 0.0);
const MARKER_LEAD: Spring = Spring::new(0.2, 0.0);
const MARKER_TRAIL: Spring = Spring::new(0.3, 0.0);
const RING_IN: Spring = Spring::new(0.12, 0.0);
const RING_OUT: Spring = Spring::new(0.7, 0.0);
/// The workspace out of view, a little smaller and dimmer, so the one arriving grows in.
const AWAY_SCALE: f32 = 0.94;
const AWAY_OPACITY: f32 = 0.35;

const EMERALD: u32 = 0x34d399;

const PILL_H: f32 = 24.0;
const PILL_EDGE: f32 = 9.0;
const PILL_REST: f32 = 9.0;
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
    /// The completion ring is showing.
    pub cheering: bool,
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
            PEEK + WINDOW_RADIUS,
        ),
        (Pose::Tiled, Mode::Fullscreen) => (x, y + PEEK, w, h - PEEK),
        _ => (x, y, w, h),
    }
}

pub fn stage(ui: &Ui, p: Paint, scene: Scene) -> El {
    let Scene {
        demo,
        cheering,
        still,
    } = scene;
    let complete = demo.complete();
    let emerald = srgb(EMERALD);
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
        .fill(srgb(0x1d2748))
        .gradient(srgb(0x1b2a52), srgb(0x3b2a5e))
        .child(desk)
        .child(menu_bar())
        .child(hud(ui, p, demo, still));
    // The ring shows at the display's size and leaves a little wider.
    let ring = el()
        .key("cheer")
        .pin(Some(-4.0), Some(-4.0), Some(-4.0), Some(-4.0))
        .radius(BEZEL_RADIUS + 4.0)
        .border(1.5, emerald)
        .opacity(if cheering { 0.9 } else { 0.0 })
        .opacity_motion(if cheering { RING_IN } else { RING_OUT })
        .scale(if complete && !cheering && !still {
            1.035
        } else {
            1.0
        })
        .scale_motion(RING_OUT)
        .a11y_hidden();
    let dark = Paint::new(true, 1.0 / p.hair);
    let mut display = el()
        .key(KEY)
        .role("group")
        .label("Practice desktop")
        .size(DISPLAY_W, DISPLAY_H)
        .padding(BEZEL)
        .radius(BEZEL_RADIUS)
        .fill(srgb(0x0b0b0d))
        .border(
            p.hair,
            if complete {
                emerald.alpha(0.55)
            } else {
                srgb(0x34343a)
            },
        )
        .box_shadow(Shadow {
            blur: 22.0,
            color: emerald.alpha(if complete { 0.2 } else { 0.0 }),
            ..Shadow::default()
        });
    for shadow in &dark.k.elevation.overlay {
        display = display.box_shadow(Shadow {
            color: shadow
                .color
                .alpha(shadow.color.a * if p.k.dark { 1.0 } else { 0.45 }),
            ..*shadow
        });
    }
    display.child(screen).child(ring)
}

/// One workspace's windows. The focused window comes last, so it draws over the other while
/// it lifts off the grid and settles back.
fn workspace(ui: &Ui, p: Paint, demo: Demo, index: usize, still: bool) -> El {
    let mode = demo.mode(index);
    let [front, back] = demo::frames(mode);
    let showing = demo.workspace() == index;
    let [first, second] = APPS[index];
    el().key(format!("ws{index}"))
        .size(SCREEN_W, SCREEN_H)
        .child(window(p, ui, second, back, mode, false, still))
        .child(window(p, ui, first, front, mode, showing, still))
}

fn window(
    p: Paint,
    ui: &Ui,
    bundle: &str,
    frame: Frame,
    mode: Mode,
    focused: bool,
    still: bool,
) -> El {
    let (x, y, w, h) = rect(frame, mode);
    let lifted = frame.pose == Pose::Lifted;
    let behind = frame.pose == Pose::Behind;
    let base = srgb(0x1b1b1f);
    let tint = tint(bundle);
    let lift = if lifted { 1.0 } else { 0.0 };
    let lights = if focused {
        [0xff5f57, 0xfebc2e, 0x28c840]
    } else {
        [0x4b4b52; 3]
    };
    let settle = |el: El| if still { el } else { el.motion(WINDOW) };
    let key = if still {
        // Under Reduce Motion a window fades into its new place.
        format!("w-{bundle}-{mode:?}")
    } else {
        format!("w-{bundle}")
    };
    let mut lamps = row()
        .key("lights")
        .absolute(7.0, 5.0)
        .gap(3.0)
        .a11y_hidden();
    for (index, color) in lights.into_iter().enumerate() {
        lamps = lamps.child(
            el().key(format!("light{index}"))
                .size(LIGHT, LIGHT)
                .radius(LIGHT / 2.0)
                .fill(srgb(color)),
        );
    }
    // A title bar's worth of title, centred like a browser's address field.
    let title_w = (w * 0.34).min(84.0);
    let title = el()
        .key("title")
        .absolute((w - title_w) / 2.0, 5.0)
        .size(title_w, LIGHT)
        .radius(LIGHT / 2.0)
        .fill(rgba(0xffffff, if focused { 0.2 } else { 0.1 }))
        .a11y_hidden();
    let icon = app_badge(ui, bundle, ICON)
        .key("icon")
        .absolute((w - ICON) / 2.0, (h - ICON) / 2.0 + 3.0)
        .opacity(if behind { 0.0 } else { 1.0 });
    settle(
        el().key(key)
            .absolute(x, y)
            .size(w, h)
            .radius(WINDOW_RADIUS)
            .fill(base)
            .gradient(base.mix(tint, 0.17), base.mix(tint, 0.05))
            .border(
                p.hair,
                rgba(
                    0xffffff,
                    if focused {
                        0.46
                    } else if behind {
                        0.2
                    } else {
                        0.12
                    },
                ),
            )
            // Shadow offsets never spring, so the resting and lifted casts trade strength.
            .box_shadow(Shadow {
                y: 1.0,
                blur: 3.0,
                color: Color::BLACK.alpha(0.35),
                ..Shadow::default()
            })
            .box_shadow(Shadow {
                y: 5.0,
                blur: 10.0,
                color: Color::BLACK.alpha(0.3 * lift),
                ..Shadow::default()
            })
            .box_shadow(Shadow {
                y: 16.0,
                blur: 32.0,
                color: Color::BLACK.alpha(0.55 * lift),
                ..Shadow::default()
            })
            .transition(Transition::opacity().springs(SETTLE)),
    )
    .child(settle(lamps))
    .child(settle(title))
    .child(settle(icon))
}

/// An app's icon, or its initial on its colour when this Mac doesn't have the app.
fn app_badge(ui: &Ui, bundle: &str, size: f32) -> El {
    match ui.apps.cached(bundle).and_then(|app| app.icon.as_ref()) {
        Some(pixels) => el().size(size, size).pixels(pixels),
        None => el()
            .size(size, size)
            .radius(size * 0.24)
            .fill(tint(bundle))
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

/// A window's colour, after its app.
fn tint(bundle: &str) -> Color {
    srgb(match bundle {
        "com.apple.Safari" => 0x2f8cff,
        "com.apple.Terminal" => 0x30d158,
        "com.apple.Notes" => 0xffd60a,
        _ => 0x40c8e0,
    })
}

fn menu_bar() -> El {
    let bar = |key: &str, width: f32| {
        el().key(key)
            .size(width, 3.0)
            .radius(1.5)
            .fill(rgba(0xffffff, 0.34))
    };
    row()
        .key("menu-bar")
        .absolute(0.0, 0.0)
        .w(SCREEN_W)
        .h(MENU_H)
        .px(8.0)
        .gap(6.0)
        .center_y()
        .fill(rgba(0x000000, 0.34))
        .a11y_hidden()
        .child(
            el().key("mark")
                .size(6.0, 6.0)
                .radius(2.0)
                .fill(rgba(0xffffff, 0.8)),
        )
        .child(bar("menu0", 18.0))
        .child(bar("menu1", 12.0))
        .child(bar("menu2", 15.0))
        .child(el().key("space").grow(1.0))
        .child(bar("status0", 9.0))
        .child(bar("status1", 9.0))
        .child(bar("status2", 20.0))
}

/// The Workspace HUD in miniature, in its Icons style: each workspace's number and apps, and a
/// marker that stretches to the one showing.
fn hud(ui: &Ui, p: Paint, demo: Demo, still: bool) -> El {
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
        .fill(rgba(0xffffff, 0.10))
        .border(p.hair, rgba(0xffffff, 0.16))
        .transition(Transition::opacity().springs(SETTLE));
    if !still {
        marker = marker.motion(MARKER).lead(MARKER_LEAD, MARKER_TRAIL);
    }
    let mut pill = el()
        .key("hud")
        .absolute((SCREEN_W - PILL_W) / 2.0, SCREEN_H - PILL_REST - PILL_H)
        .size(PILL_W, PILL_H)
        .radius(PILL_H / 2.0)
        .fill(srgb(0x161618))
        .border(p.hair, srgb(0x303036))
        .box_shadow(Shadow {
            y: 4.0,
            blur: 12.0,
            color: Color::BLACK.alpha(0.5),
            ..Shadow::default()
        })
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
                        .color(rgba(0xffffff, 0.92)),
                ),
            );
        for bundle in bundles {
            group = group.child(app_badge(ui, bundle, HUD_ICON).key(format!("app-{bundle}")));
        }
        pill = pill.child(group);
    }
    pill
}
