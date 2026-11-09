//! The HUD capsule and how it moves. Everything springs from where it is drawn, so a second
//! switch mid-flight turns the motion around instead of restarting it.
//!
//! | What changes                   | What moves                                              | Spring (duration / bounce) |
//! | ------------------------------ | ------------------------------------------------------- | -------------------------- |
//! | The HUD appears                | fades in, scales from .96 and rises 6 pt                | .2 / 0                     |
//! | The HUD leaves                 | the same path back                                      | .15 / 0                    |
//! | Another workspace              | changed characters roll, up going forward and down back | .45 / 0, width .35 / 0     |
//! |                                | old tiles slide out 10 pt and new ones in, the way of   | settle: .3 / 0, .24 / 0    |
//! |                                | travel                                                  |                            |
//! | Another kind of message        | the phrase blurs out as the new one blurs in, in place  | settle                     |
//! | The same workspace's layout    | each tile springs to its new frame                      | settle                     |
//!
//! Under Reduce Motion nothing travels or scales; the same changes take a short fade.

use rune::{
    Color, Edge, El, Spring, TextStyle, Transition, el, row, text, tok::motion::pair::SETTLE,
};
use rune_kit::RollingText;
use spaceman::{
    Axis, Rect,
    app::{HUD_PREVIEW_RECT, UiHudEvent, UiHudKind, UiHudLayout},
};

use super::Travel;
use crate::controls::Paint;

pub const PILL_HEIGHT: f32 = 44.0;
/// How far the capsule's shadow reaches below it: the lowest layer's offset plus its blur.
pub const SHADOW_REACH: f32 = 48.0;

/// The capsule answering a shortcut: quick in, a quarter faster out.
const ENTER: Spring = Spring::new(0.2, 0.0);
const EXIT: Spring = Spring::new(0.15, 0.0);
const REST_SCALE: f32 = 0.96;
const DROP: f32 = 6.0;
/// Labels roll to the next word; the capsule's width leads at 0.78 of this.
const ROLL: Spring = Spring::new(0.45, 0.0);
/// How far a workspace's tiles slide in the mini layout.
const SLIDE: f32 = 10.0;

/// One build of the capsule: what it shows and how it got here.
#[derive(Clone, Copy, Debug)]
pub struct HudView<'a> {
    pub event: &'a UiHudEvent,
    /// Up, or on its way out.
    pub shown: bool,
    /// It was hidden when this event arrived, so its content lands at rest inside the entrance.
    pub fresh: bool,
    pub travel: Travel,
    /// Reduce Motion: it fades where it rests, with no scale or drop to land in one frame.
    pub still: bool,
    /// Names its rolling labels, which remember their last text per key: the overlay and the
    /// Playground stage each keep their own.
    pub scope: &'static str,
}

#[must_use]
pub fn labels(event: &UiHudEvent) -> (String, String) {
    let ws = event.workspace;
    let count_label = match event.windows {
        0 => "Empty".to_owned(),
        1 => "1 window".to_owned(),
        n => format!("{n} windows"),
    };
    match event.kind {
        UiHudKind::Workspace => {
            // The count comes first so it holds its place from one workspace to the next.
            let detail = match event.preview {
                UiHudLayout::Fullscreen => format!("{count_label} · Fullscreen"),
                UiHudLayout::Split(_) => count_label,
            };
            (format!("Workspace {ws}"), detail)
        }
        UiHudKind::Moved => (format!("Moved to Workspace {ws}"), count_label),
        UiHudKind::Layout { axis } => {
            let split = match axis {
                Axis::Horizontal => "Horizontal split",
                Axis::Vertical => "Vertical split",
            };
            (format!("Workspace {ws}"), split.to_owned())
        }
        UiHudKind::Fullscreen { enabled } => (
            format!("Workspace {ws}"),
            if enabled {
                "Fullscreen layout".to_owned()
            } else {
                "Split layout".to_owned()
            },
        ),
    }
}

/// The capsule in the fixed-size row that carries its entrance and exit, so the capsule's own
/// width can change underneath without moving the pose.
pub fn slot(view: &HudView, p: Paint) -> El {
    let (spring, opacity) = if view.shown {
        (ENTER, 1.0)
    } else {
        (EXIT, 0.0)
    };
    let (scale, drop) = if view.shown || view.still {
        (1.0, 0.0)
    } else {
        (REST_SCALE, DROP)
    };
    row()
        .key("hud-slot")
        .w_full()
        .h(PILL_HEIGHT)
        .center()
        .opacity(opacity)
        .opacity_motion(spring)
        .scale(scale)
        .scale_motion(spring)
        .offset(0.0, drop)
        .offset_motion(spring)
        .child(pill(view, p))
}

/// Which phrase each label holds. Within one, only the characters that changed roll
/// ("Workspace 1" to "Workspace 2", "3 windows" to "2 windows · Fullscreen"); a different one
/// replaces the whole label in place, so no letter is left sliding across the others.
const fn phrases(kind: UiHudKind) -> (&'static str, &'static str) {
    match kind {
        UiHudKind::Workspace => ("workspace", "count"),
        UiHudKind::Moved => ("moved", "count"),
        UiHudKind::Layout { .. } => ("workspace", "split"),
        UiHudKind::Fullscreen { .. } => ("workspace", "mode"),
    }
}

fn pill(view: &HudView, p: Paint) -> El {
    let (headline, detail) = labels(view.event);
    let (headline_phrase, detail_phrase) = phrases(view.event.kind);
    let dark = Paint::new(true, 1.0 / p.hair);
    // Going forward the new characters come up from below; going back, down from above.
    let from = match view.travel {
        Travel::Back => Edge::Top,
        Travel::Forward | Travel::Stay => Edge::Bottom,
    };
    let label = |phrase: &str, copy: String, weight: f32, color: u32| {
        RollingText::new(format!("{}-{phrase}", view.scope), copy)
            .font(figures(weight))
            .color(srgb(color))
            .spring(ROLL)
            .from(from)
            .build(dark.k)
            .transition(Transition::blur_replace().springs(SETTLE))
    };
    let mut capsule = row()
        .key("hud-pill")
        .h(PILL_HEIGHT)
        .pl(12.0)
        .pr(16.0)
        .gap(12.0)
        .center_y()
        .radius(PILL_HEIGHT / 2.0)
        .fill(srgb(0x161618))
        .border(p.hair, srgb(0x303036))
        .motion(ROLL)
        .width_leads()
        .child(mini_layout(view, p.hair))
        .child(
            row()
                .key("copy")
                .gap(8.0)
                .center_y()
                .child(label(headline_phrase, headline, 620.0, 0xf4f4f6))
                .child(
                    text("·")
                        .key("dot")
                        .font(figures(500.0))
                        .color(srgb(0x6c6c76)),
                )
                .child(label(detail_phrase, detail, 460.0, 0xa4a4ae)),
        );
    if view.fresh {
        capsule = capsule.instant_changes();
    }
    for shadow in &dark.k.elevation.overlay {
        capsule = capsule.box_shadow(*shadow);
    }
    capsule
}

/// Tabular figures, so a workspace or window count changing digit doesn't change the width.
fn figures(weight: f32) -> TextStyle {
    TextStyle {
        features: "'tnum' 1",
        ..TextStyle::new(13.0, weight).lh(16.0)
    }
}

fn mini_layout(view: &HudView, hair: f32) -> El {
    const OUTER_W: f32 = 30.0;
    const OUTER_H: f32 = 20.0;
    const INSET: f32 = 2.5;
    let inner_w = OUTER_W - 2.0 * hair - INSET * 2.0;
    let inner_h = OUTER_H - 2.0 * hair - INSET * 2.0;

    // Workspaces sit side by side: going forward the next one comes in from the right.
    let dx = match view.travel {
        Travel::Forward => SLIDE,
        Travel::Back => -SLIDE,
        Travel::Stay => 0.0,
    };
    let slide = |dx: f32| {
        Transition::offset(dx, 0.0)
            .combined(Transition::opacity())
            .springs(SETTLE)
    };
    let tile = |index: usize, (x, y, w, h): (f32, f32, f32, f32), radius: f32, alpha: f32| {
        el().key(format!("tile{index}"))
            .absolute(x, y)
            .w(w)
            .h(h)
            .radius(radius)
            .fill(rgba(0xffffff, alpha))
            .transition(Transition::scale(0.9).springs(SETTLE))
    };

    let mut tiles = el()
        .key(format!("ws{}", view.event.workspace))
        .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
        .transition(Transition::asymmetric(slide(dx), slide(-dx)));
    if view.travel != Travel::Stay {
        // A workspace's tiles arrive with it instead of each fading in on top of the slide.
        tiles = tiles.instant_changes();
    }
    match &view.event.preview {
        // The first tile grows to fill the screen, as the focused window does.
        UiHudLayout::Fullscreen => {
            tiles = tiles.child(tile(0, (INSET, INSET, inner_w, inner_h), 2.2, 0.86));
        }
        UiHudLayout::Split(rects) if rects.is_empty() => {
            tiles = tiles.child(
                el().key("empty")
                    .absolute((OUTER_W - 3.0) / 2.0 - hair, (OUTER_H - 3.0) / 2.0 - hair)
                    .w(3.0)
                    .h(3.0)
                    .radius(1.5)
                    .fill(rgba(0xffffff, 0.28))
                    .transition(Transition::opacity().springs(SETTLE)),
            );
        }
        UiHudLayout::Split(rects) => {
            for (index, rect) in rects.iter().enumerate() {
                let frame = scale_tile(*rect, INSET, inner_w, inner_h, 0.6, 2.0);
                tiles = tiles.child(tile(index, frame, 1.8, 0.80));
            }
        }
    }

    el().key("mini-screen")
        .w(OUTER_W)
        .h(OUTER_H)
        .radius(4.5)
        .fill(rgba(0xffffff, 0.05))
        .border(hair, rgba(0xffffff, 0.22))
        .motion(SETTLE.enter)
        // Tiles rest inside the inset; sliding ones fade out through it instead of being cut.
        .clip_fade(INSET, 0.0, INSET, 0.0)
        .child_exit(slide(-dx))
        .child(tiles)
}

pub(super) fn scale_tile(
    rect: Rect,
    inset: f32,
    inner_w: f32,
    inner_h: f32,
    gap: f32,
    min_side: f32,
) -> (f32, f32, f32, f32) {
    let pw = HUD_PREVIEW_RECT.width.max(1) as f32;
    let ph = HUD_PREVIEW_RECT.height.max(1) as f32;
    (
        inset + (rect.x as f32 / pw) * inner_w + gap,
        inset + (rect.y as f32 / ph) * inner_h + gap,
        ((rect.width as f32 / pw) * inner_w - gap * 2.0).max(min_side),
        ((rect.height as f32 / ph) * inner_h - gap * 2.0).max(min_side),
    )
}

fn srgb(hex: u32) -> Color {
    rgba(hex, 1.0)
}

fn rgba(hex: u32, alpha: f32) -> Color {
    let channel = |shift: u32| rune::color::srgb_decode(((hex >> shift) & 0xff) as f32 / 255.0);
    Color {
        r: channel(16),
        g: channel(8),
        b: channel(0),
        a: alpha,
    }
}
