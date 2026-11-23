//! The Icons style: each workspace as its number and the icons of its apps, with no layouts.
//! Nothing changes size; the group you are on is bright and the others dim.

use rune::{El, Spring, TextStyle, row, text};
use spaceman::{WorkspaceId, app::UiHudWorkspace};

use super::pill::{INNER, Icons, Track, app_icon, presence, srgb};
use crate::controls::Paint;

pub const EDGE: f32 = 14.0;
pub const ICON: f32 = 16.0;
const ICON_GAP: f32 = 3.0;
/// From the number to its first icon.
const NUMBER_GAP: f32 = 5.0;
pub const GROUP_GAP: f32 = 14.0;
/// The marker's reach past the current group, and its height.
pub const PAD: f32 = 6.0;
pub const MARKER_H: f32 = 30.0;
const NUMBER_MIN_W: f32 = 9.0;
const DIM: f32 = 0.4;
const GROUP: Spring = Spring::new(0.3, 0.0);
/// One icon per app, so a workspace full of windows still fits.
const MAX_APPS: usize = 12;

/// Tabular figures, so a group's width doesn't change with its digit.
fn number_style() -> TextStyle {
    TextStyle {
        features: "'tnum' 1",
        ..TextStyle::new(13.0, 600.0).lh(16.0)
    }
}

/// Each app once, in tile order, then the floating ones.
#[must_use]
pub fn apps(entry: &UiHudWorkspace) -> Vec<&str> {
    let mut apps: Vec<&str> = Vec::new();
    let tiles = entry.tiles.iter().filter_map(|tile| tile.bundle.as_deref());
    for bundle in tiles.chain(entry.floating.iter().map(String::as_str)) {
        if apps.len() == MAX_APPS {
            break;
        }
        if !apps.contains(&bundle) {
            apps.push(bundle);
        }
    }
    apps
}

fn number_width(label: &str) -> f32 {
    rune::text::advance(label, number_style())
        .ceil()
        .max(NUMBER_MIN_W)
}

fn group_width(number: f32, apps: usize) -> f32 {
    if apps == 0 {
        return number;
    }
    number + NUMBER_GAP + apps as f32 * ICON + (apps - 1) as f32 * ICON_GAP
}

pub fn track(
    entries: &[UiHudWorkspace],
    focus: Option<WorkspaceId>,
    icons: &Icons,
    _p: Paint,
) -> Track {
    let mut x = EDGE;
    let mut marker = None;
    let mut cells = Vec::with_capacity(entries.len());
    for entry in entries {
        let label = entry.workspace.to_string();
        let number = number_width(&label);
        let apps = apps(entry);
        let width = group_width(number, apps.len());
        let current = Some(entry.workspace) == focus;
        if current {
            marker = Some([
                x - PAD,
                (INNER - MARKER_H) / 2.0,
                width + 2.0 * PAD,
                MARKER_H,
            ]);
        }
        cells.push(
            group(entry.workspace, &label, number, &apps, icons)
                .absolute(x, 0.0)
                .w(width)
                .h(INNER)
                .opacity(if current { 1.0 } else { DIM })
                .opacity_motion(GROUP)
                .motion(GROUP)
                .transition(presence()),
        );
        x += width + GROUP_GAP;
    }
    let width = if entries.is_empty() {
        2.0 * EDGE
    } else {
        x - GROUP_GAP + EDGE
    };
    Track {
        width,
        marker,
        cells,
    }
}

fn group(workspace: WorkspaceId, label: &str, number: f32, apps: &[&str], icons: &Icons) -> El {
    let digits = row().key("number").w(number).h(16.0).center().child(
        text(label.to_owned())
            .font(number_style())
            .color(srgb(0xf4f4f6)),
    );
    let mut group = row().key(format!("ws{workspace}")).center_y().child(digits);
    if !apps.is_empty() {
        group = group.child(
            row()
                .key("apps")
                .pl(NUMBER_GAP)
                .gap(ICON_GAP)
                .center_y()
                .children(apps.iter().map(|bundle| app_icon(icons, bundle, ICON))),
        );
    }
    group
}
