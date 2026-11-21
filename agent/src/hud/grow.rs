//! The Layouts style: each workspace as its layout, an app icon in every tile, the one you are
//! on larger than the rest.

use rune::{El, Spring, el, row};
use spaceman::{
    Rect, WorkspaceId,
    app::{HUD_PREVIEW_RECT, UiHudWorkspace},
};

use super::pill::{INNER, Icons, ROW, Track, app_icon, presence, rgba};
use crate::controls::Paint;

/// The current workspace's screen; the others draw at `SMALL` of it.
pub const SCREEN_W: f32 = 50.0;
pub const SCREEN_H: f32 = 32.0;
pub const SMALL: f32 = 0.62;
pub const GAP: f32 = 8.0;
/// From the capsule's inside edge to the first screen.
pub const EDGE: f32 = 12.0;
/// The marker's reach past the current screen.
pub const PAD: f32 = 3.0;
const DIM: f32 = 0.5;
/// Sizes, places and brightness all spring together, so the row stays one row mid-flight.
const GROW: Spring = ROW;
const INSET: f32 = 3.0;
const TILE_GAP: f32 = 0.8;
const ICON_MAX: f32 = 18.0;
/// Fullscreen Layout's other apps, beside the one in front.
const BEHIND_SCALE: f32 = 0.7;
const BEHIND_OPACITY: f32 = 0.55;
const BEHIND_GAP: f32 = 3.0;

/// Where each screen rests: its left edge and scale.
#[must_use]
pub fn rest(entries: &[UiHudWorkspace], focus: Option<WorkspaceId>) -> (Vec<(f32, f32)>, f32) {
    let mut x = EDGE;
    let cells = entries
        .iter()
        .map(|entry| {
            let scale = if Some(entry.workspace) == focus {
                1.0
            } else {
                SMALL
            };
            let cell = (x, scale);
            x += SCREEN_W * scale + GAP;
            cell
        })
        .collect::<Vec<_>>();
    let width = if cells.is_empty() {
        2.0 * EDGE
    } else {
        x - GAP + EDGE
    };
    (cells, width)
}

pub fn track(
    entries: &[UiHudWorkspace],
    focus: Option<WorkspaceId>,
    icons: &Icons,
    p: Paint,
) -> Track {
    let (rests, width) = rest(entries, focus);
    let mut marker = None;
    let cells = entries
        .iter()
        .zip(&rests)
        .map(|(entry, &(x, scale))| {
            let current = Some(entry.workspace) == focus;
            if current {
                let height = SCREEN_H + 2.0 * PAD;
                marker = Some([
                    x - PAD,
                    (INNER - height) / 2.0,
                    SCREEN_W + 2.0 * PAD,
                    height,
                ]);
            }
            // The screen keeps its full size and draws scaled about its centre, which sits in
            // the middle of the room its scaled width takes.
            screen(entry, icons, p)
                .absolute(
                    x + SCREEN_W * scale / 2.0 - SCREEN_W / 2.0,
                    (INNER - SCREEN_H) / 2.0,
                )
                .scale(scale)
                .scale_motion(GROW)
                .opacity(if current { 1.0 } else { DIM })
                .opacity_motion(GROW)
                .motion(GROW)
                .transition(presence())
        })
        .collect();
    Track {
        width,
        marker,
        cells,
    }
}

fn screen(entry: &UiHudWorkspace, icons: &Icons, p: Paint) -> El {
    let inner_w = SCREEN_W - 2.0 * p.hair - 2.0 * INSET;
    let inner_h = SCREEN_H - 2.0 * p.hair - 2.0 * INSET;
    let base = el()
        .key(format!("ws{}", entry.workspace))
        .size(SCREEN_W, SCREEN_H)
        .radius(5.0)
        .fill(rgba(0xffffff, 0.05))
        .border(p.hair, rgba(0xffffff, 0.22));
    if entry.fullscreen
        && let Some(front) = entry.tiles.first()
    {
        let (x, y, w, h) = (
            INSET + TILE_GAP,
            INSET + TILE_GAP,
            inner_w - 2.0 * TILE_GAP,
            inner_h - 2.0 * TILE_GAP,
        );
        let mut behind: Vec<&str> = Vec::new();
        for bundle in entry.tiles[1..]
            .iter()
            .filter_map(|tile| tile.bundle.as_deref())
            .chain(entry.floating.iter().map(String::as_str))
        {
            if Some(bundle) != front.bundle.as_deref() && !behind.contains(&bundle) {
                behind.push(bundle);
            }
        }
        return base.child(tile("full", (x, y, w, h)).child(icon_row(
            front.bundle.as_deref(),
            &behind,
            w,
            h,
            icons,
        )));
    }
    if entry.tiles.is_empty() {
        if !entry.floating.is_empty() {
            // Only floating windows: their icons, small, with no layout to sit in.
            let floating: Vec<&str> = entry.floating.iter().map(String::as_str).collect();
            return base.child(
                row()
                    .key("floating")
                    .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
                    .center()
                    .child(icon_row(None, &floating, inner_w, inner_h, icons)),
            );
        }
        return base.child(
            el().key("empty")
                .absolute(
                    (SCREEN_W - 4.0) / 2.0 - p.hair,
                    (SCREEN_H - 4.0) / 2.0 - p.hair,
                )
                .size(4.0, 4.0)
                .radius(2.0)
                .fill(rgba(0xffffff, 0.28))
                .transition(presence()),
        );
    }
    base.children(entry.tiles.iter().map(|placed| {
        let frame = scale_tile(placed.rect, INSET, inner_w, inner_h, TILE_GAP, 2.0);
        let size = icon_size(frame.2, frame.3);
        let tile = tile(&format!("t{}", placed.key), frame).motion(GROW);
        match (&placed.bundle, size) {
            (Some(bundle), Some(size)) => tile.child(app_icon(icons, bundle, size).motion(GROW)),
            _ => tile,
        }
    }))
}

fn tile(key: &str, (x, y, w, h): (f32, f32, f32, f32)) -> El {
    row()
        .key(key)
        .absolute(x, y)
        .size(w, h)
        .radius(3.0)
        .fill(rgba(0xffffff, 0.11))
        .center()
        .transition(presence())
}

/// The largest icon a `w` × `h` tile holds with room around it, if any is worth drawing.
fn icon_size(w: f32, h: f32) -> Option<f32> {
    let size = ICON_MAX.min(w.min(h) - 3.0);
    (size >= 6.0).then_some(size)
}

/// The app in front at full size, then as many of the others as fit, smaller and dimmer.
fn icon_row(front: Option<&str>, behind: &[&str], w: f32, h: f32, icons: &Icons) -> El {
    let size = icon_size(w, h).unwrap_or(6.0);
    let small = size * BEHIND_SCALE;
    let mut room = w - 2.0 * TILE_GAP - front.map_or(-BEHIND_GAP, |_| size);
    let mut icons_row = row().key("apps").gap(BEHIND_GAP).center_y();
    if let Some(front) = front {
        icons_row = icons_row.child(app_icon(icons, front, size));
    }
    for bundle in behind {
        room -= BEHIND_GAP + small;
        if room < 0.0 {
            break;
        }
        icons_row = icons_row.child(app_icon(icons, bundle, small).opacity(BEHIND_OPACITY));
    }
    icons_row
}

/// `rect`, in [`HUD_PREVIEW_RECT`] space, inside an `inner_w` × `inner_h` screen.
#[must_use]
pub fn scale_tile(
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
