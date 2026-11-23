//! The HUD capsule and how it moves: one row of workspaces with a marker on the one you are on.
//! Everything springs from where it is drawn, so a second switch mid-flight turns the motion
//! around instead of restarting it. The two styles differ only in their cells
//! (`grow.rs`, `icons.rs`); the capsule, marker, overflow and pose are shared here.
//!
//! | What changes                     | What moves                                            | Spring (duration / bounce) |
//! | -------------------------------- | ----------------------------------------------------- | -------------------------- |
//! | The HUD appears                  | fades in, scales from .96 and rises 6 pt              | .2 / 0                     |
//! | The HUD leaves                   | the same path back                                    | .15 / 0                    |
//! | Another workspace                | the marker's leading edge, then its trailing edge     | .26 / 0, .42 / 0           |
//! |                                  | Layouts: the cells grow and shrink, neighbours slide  | .35 / 0                    |
//! |                                  | Icons: the groups brighten and dim                    | .3 / 0                     |
//! | Shown again after hiding         | the marker starts on the workspace you left and       | as above                   |
//! |                                  | travels while the capsule fades in                    |                            |
//! | A workspace joins or leaves      | it fades and scales in or out in place                | settle                     |
//! | A layout changes                 | each tile springs to its new frame                    | .35 / 0                    |
//! | A window moves                   | its icon leaves one workspace for the other           | settle                     |
//! | More workspaces than fit         | the row scrolls to keep yours in view, edges fade     | .35 / 0                    |
//!
//! Under Reduce Motion nothing travels or scales; the row cross-fades instead.

use std::collections::HashMap;

use rune::{Color, El, Pixels, Spring, Transition, el, row, text, tok::motion::pair::SETTLE};
use spaceman::{
    WorkspaceId,
    app::{UiHudDisplay, UiHudWorkspace},
    config::HudStyle,
};

use super::{grow, icons};
use crate::controls::Paint;

pub const PILL_HEIGHT: f32 = 44.0;
/// The capsule inside its border, where the row is laid out.
pub const INNER: f32 = PILL_HEIGHT - 2.0;
/// How far the capsule's shadow reaches below it: the lowest layer's offset plus its blur.
pub const SHADOW_REACH: f32 = 48.0;
/// Room each side of the capsule for its shadow's widest blur.
pub const SHADOW_SIDE: f32 = 32.0;

/// The capsule answering a shortcut: quick in, a quarter faster out.
const ENTER: Spring = Spring::new(0.2, 0.0);
const EXIT: Spring = Spring::new(0.15, 0.0);
const REST_SCALE: f32 = 0.96;
const DROP: f32 = 6.0;
/// The marker's leading edge, quicker so it stretches toward where it is going, and its
/// trailing edge catching up; with nowhere to go, one spring for both.
const MARKER_LEAD: Spring = Spring::new(0.26, 0.0);
const MARKER_TRAIL: Spring = Spring::new(0.42, 0.0);
const MARKER: Spring = Spring::new(0.3, 0.0);
/// The row's own layout: scrolling, and the capsule's width.
pub const ROW: Spring = Spring::new(0.35, 0.0);
/// The fade at an edge the row is cut at, and at an edge it fits inside.
const CLIPPED_FADE: f32 = 24.0;
const EDGE_FADE: f32 = 6.0;

/// App icons by bundle ID, looked up before the view runs.
pub type Icons = HashMap<String, Pixels>;

/// One build of the capsule: what it shows and how it got here.
#[derive(Clone, Copy, Debug)]
pub struct HudView<'a> {
    pub row: &'a UiHudDisplay,
    /// Up, or on its way out.
    pub shown: bool,
    /// Just shown: everything lands where it is drawn, with the marker on `focus`.
    pub fresh: bool,
    /// Where the marker sits: the visible workspace, or while fresh the one a switch left.
    pub focus: Option<WorkspaceId>,
    /// Reduce Motion: it fades where it rests, with no scale or drop to land in one frame.
    pub still: bool,
}

impl HudView<'_> {
    /// The row's workspaces, with the one the marker starts on while it's no longer listed (an
    /// empty workspace just left), so the marker has somewhere to leave from.
    #[must_use]
    pub fn entries(&self) -> Vec<UiHudWorkspace> {
        let mut entries = self.row.workspaces.clone();
        if let Some(focus) = self.focus
            && !entries.iter().any(|entry| entry.workspace == focus)
        {
            let at = entries.partition_point(|entry| entry.workspace < focus);
            entries.insert(
                at,
                UiHudWorkspace {
                    workspace: focus,
                    fullscreen: false,
                    tiles: Vec::new(),
                    floating: Vec::new(),
                },
            );
        }
        entries
    }
}

/// A style's row: cells laid out from `x = 0`, and where the marker goes.
pub struct Track {
    pub width: f32,
    /// The marker's left edge, top and size, when the focus is in the row.
    pub marker: Option<[f32; 4]>,
    pub cells: Vec<El>,
}

/// The widest the row gets before it scrolls, for a panel `panel_width` wide.
#[must_use]
pub fn max_row_width(panel_width: f32) -> f32 {
    panel_width - 2.0 * SHADOW_SIDE - 2.0
}

/// The capsule in the fixed-size row that carries its entrance and exit, so the capsule's own
/// width can change underneath without moving the pose.
pub fn slot(view: &HudView, style: HudStyle, icons: &Icons, panel_width: f32, p: Paint) -> El {
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
    let entries = view.entries();
    let track = match style {
        HudStyle::Layouts => grow::track(&entries, view.focus, icons, p),
        HudStyle::Icons => icons::track(&entries, view.focus, icons, p),
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
        .child(capsule(view, track, max_row_width(panel_width), p))
}

fn capsule(view: &HudView, track: Track, max_width: f32, p: Paint) -> El {
    let dark = Paint::new(true, 1.0 / p.hair);
    let width = track.width.min(max_width);
    let overflow = track.width - width;
    // Keep the marker's workspace in the middle of what shows, as far as the row reaches.
    let scroll = track.marker.map_or(0.0, |[x, _, w, _]| {
        (x + w / 2.0 - width / 2.0).clamp(0.0, overflow.max(0.0))
    });
    let fade = |clipped: bool| if clipped { CLIPPED_FADE } else { EDGE_FADE };
    let mut content = el()
        .key(match (view.still, view.focus) {
            // Under Reduce Motion the whole row cross-fades instead of anything moving.
            (true, Some(focus)) => format!("hud-row-{focus}"),
            _ => "hud-row".to_owned(),
        })
        .absolute(-scroll, 0.0)
        .w(track.width)
        .h(INNER)
        .motion(ROW)
        .transition(Transition::opacity().springs(SETTLE));
    if let Some([x, y, w, h]) = track.marker {
        let marker = el()
            .key(match view.still {
                true => format!(
                    "hud-marker-{}",
                    view.focus.map(|f| f.to_string()).unwrap_or_default()
                ),
                false => "hud-marker".to_owned(),
            })
            .absolute(x, y)
            .w(w)
            .h(h)
            .radius(9.0)
            .fill(rgba(0xffffff, 0.1))
            .border(p.hair, rgba(0xffffff, 0.2))
            .motion(MARKER)
            .lead(MARKER_LEAD, MARKER_TRAIL)
            .transition(Transition::opacity().springs(SETTLE));
        content = content.child(marker);
    }
    let viewport = el()
        .key("hud-view")
        .w(width)
        .h(INNER)
        .motion(ROW)
        .clip_fade(fade(scroll > 0.5), 0.0, fade(scroll < overflow - 0.5), 0.0)
        .child(content.children(track.cells));
    let mut capsule = row()
        .key("hud-pill")
        .w(width + 2.0)
        .h(PILL_HEIGHT)
        .center()
        .radius(PILL_HEIGHT / 2.0)
        .fill(srgb(0x161618))
        .border(p.hair, srgb(0x303036))
        .motion(ROW)
        .child(viewport);
    if view.fresh {
        // Just shown: the row lands as drawn, inside the capsule's entrance.
        capsule = capsule.instant_changes();
    }
    for shadow in &dark.k.elevation.overlay {
        capsule = capsule.box_shadow(*shadow);
    }
    capsule
}

/// How a workspace joins or leaves the row: in place, with a fade and a little scale.
pub fn presence() -> Transition {
    Transition::scale(0.9).springs(SETTLE)
}

/// An app's icon at `size`, or its initial in a rounded square when it has none here.
pub fn app_icon(icons: &Icons, bundle: &str, size: f32) -> El {
    let base = el()
        .key(format!("app-{bundle}"))
        .size(size, size)
        .transition(presence());
    match icons.get(bundle) {
        Some(pixels) => base.pixels(pixels),
        None => {
            let initial = crate::apps::fallback_name(bundle)
                .chars()
                .next()
                .map(String::from)
                .unwrap_or_default();
            base.radius(size * 0.25)
                .fill(rgba(0xffffff, 0.16))
                .center()
                .child(
                    text(initial)
                        .font(rune::TextStyle::new((size * 0.6).max(6.0), 600.0))
                        .color(rgba(0xffffff, 0.7)),
                )
        }
    }
}

pub fn srgb(hex: u32) -> Color {
    rgba(hex, 1.0)
}

pub fn rgba(hex: u32, alpha: f32) -> Color {
    let channel = |shift: u32| rune::color::srgb_decode(((hex >> shift) & 0xff) as f32 / 255.0);
    Color {
        r: channel(16),
        g: channel(8),
        b: channel(0),
        a: alpha,
    }
}
