//! Scrolling Strip, a Spaceman Lab: a workspace's tiles as one horizontal ribbon of columns seen
//! through a viewport camera, as in niri. Each column keeps its own width (a share of the
//! display); the camera slides along the ribbon on a spring and only moves when the focused
//! column would otherwise leave the screen (niri's minimal scroll).
//!
//! Past a free display edge an off-screen column keeps a [`PEEK_PX`] sliver visible, so it
//! glides off and back on instead of vanishing. Against an edge shared with another monitor a
//! column stays inside the display (or, with `scrolling_strip_shared_edge = "slide"`, keeps its
//! center inside), and once the camera rests the columns beyond it are parked like an inactive
//! workspace's windows. Everything here is pure geometry: the worker decides
//! rest layouts with it, and the main thread samples a [`StripMotion`] every display frame.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::{
    PhysicalWindowId, Rect, SlotId, WorkspaceId,
    motion::{CameraSpring, SETTLE_TOLERANCE_PT, ScrollingDisplayBounds},
};

/// How much of an off-screen column stays visible past a free display edge.
pub const PEEK_PX: i32 = 16;

/// The longest a motion is assumed to run before it counts as settled anyway.
pub const MAX_MOTION: Duration = Duration::from_secs(3);

/// A column's width as a share of its display.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StripWidth {
    /// 33%: three columns side by side.
    Third,
    /// 50%, niri's default: two columns fill the display.
    #[default]
    Half,
    /// 66%, with a neighbor showing in the remaining third.
    TwoThirds,
    /// 80%.
    FourFifths,
    /// 100%: the column alone.
    Full,
}

impl StripWidth {
    /// Every preset, narrowest first; `⌥-` and `⌥=` step through them.
    pub const PRESETS: [Self; 5] = [
        Self::Third,
        Self::Half,
        Self::TwoThirds,
        Self::FourFifths,
        Self::Full,
    ];

    const fn index(self) -> usize {
        match self {
            Self::Third => 0,
            Self::Half => 1,
            Self::TwoThirds => 2,
            Self::FourFifths => 3,
            Self::Full => 4,
        }
    }

    #[must_use]
    pub const fn narrower(self) -> Self {
        Self::PRESETS[self.index().saturating_sub(1)]
    }

    #[must_use]
    pub const fn wider(self) -> Self {
        let next = self.index() + 1;
        if next < Self::PRESETS.len() {
            Self::PRESETS[next]
        } else {
            self
        }
    }

    #[must_use]
    pub const fn percent(self) -> i32 {
        match self {
            Self::Third => 33,
            Self::Half => 50,
            Self::TwoThirds => 66,
            Self::FourFifths => 80,
            Self::Full => 100,
        }
    }

    /// This column's share of `width` points.
    #[must_use]
    pub const fn share(self, width: i32) -> i32 {
        let (numerator, denominator) = match self {
            Self::Third => (1, 3),
            Self::Half => (1, 2),
            Self::TwoThirds => (2, 3),
            Self::FourFifths => (4, 5),
            Self::Full => (1, 1),
        };
        (width as i64 * numerator / denominator) as i32
    }
}

/// One column of the strip: a bound slot in layout order and its width.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StripColumn {
    pub slot: SlotId,
    pub width: StripWidth,
}

/// A column's place on the ribbon: its left edge in points from the ribbon's start, and its
/// width in points.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RibbonColumn {
    pub slot: SlotId,
    pub x: i32,
    pub width: i32,
}

/// Lays `columns` end to end for a display `view_width` points wide.
#[must_use]
pub fn ribbon(columns: &[StripColumn], view_width: i32) -> Vec<RibbonColumn> {
    let mut x = 0;
    columns
        .iter()
        .map(|column| {
            let width = column.width.share(view_width).max(1);
            let placed = RibbonColumn {
                slot: column.slot,
                x,
                width,
            };
            x += width;
            placed
        })
        .collect()
}

/// The ribbon's total width.
#[must_use]
pub fn ribbon_width(ribbon: &[RibbonColumn]) -> i32 {
    ribbon.last().map_or(0, |column| column.x + column.width)
}

/// `camera` kept where the ribbon covers the whole view where it can.
#[must_use]
pub fn clamp_camera(camera: f64, view_width: i32, total: i32) -> f64 {
    camera.clamp(0.0, f64::from((total - view_width).max(0)))
}

/// Niri's minimal scroll: the camera stays put while `column` is wholly in view, and otherwise
/// moves the least distance that brings it in. A column wider than the view aligns its left
/// edge.
#[must_use]
pub fn minimal_scroll(camera: f64, view_width: i32, column: RibbonColumn, total: i32) -> f64 {
    let (x, width, view) = (
        f64::from(column.x),
        f64::from(column.width),
        f64::from(view_width),
    );
    let target = if width >= view || x < camera {
        x
    } else if x + width > camera + view {
        x + width - view
    } else {
        camera
    };
    clamp_camera(target, view_width, total)
}

/// The camera that puts `anchor` at the view's left edge, for a strip restored from disk.
#[must_use]
pub fn anchor_camera(ribbon: &[RibbonColumn], anchor: Option<SlotId>, view_width: i32) -> f64 {
    let x = anchor
        .and_then(|anchor| ribbon.iter().find(|column| column.slot == anchor))
        .map_or(0, |column| column.x);
    clamp_camera(f64::from(x), view_width, ribbon_width(ribbon))
}

/// A column's left edge on screen for a camera: where the ribbon puts it, clamped so a column
/// past a free edge still peeks and one past a shared edge stays on its own display.
#[must_use]
pub fn column_x(
    output: Rect,
    bounds: &ScrollingDisplayBounds,
    world_x: f64,
    width: i32,
    camera: f64,
) -> i32 {
    let desired = (f64::from(output.x) + world_x - camera).round() as i32;
    bounds.clamp_column_x(desired, width, PEEK_PX)
}

/// Where a column rests once the camera stops.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColumnRest {
    /// At least partly in view, at this frame.
    Visible(Rect),
    /// Out of view past a free edge, showing a sliver at this frame. Recorded as parked.
    Peeking(Rect),
    /// Out of view past an edge shared with another display: parked like a hidden
    /// workspace's window.
    Parked,
}

/// Every column's rest for one camera, in ribbon order.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StripLayout {
    pub columns: Vec<(SlotId, ColumnRest)>,
    /// The leftmost column in view: what a restart scrolls back to.
    pub anchor: Option<SlotId>,
}

impl StripLayout {
    /// Columns in view and their frames.
    pub fn visible(&self) -> impl Iterator<Item = (SlotId, Rect)> + '_ {
        self.columns.iter().filter_map(|(slot, rest)| match rest {
            ColumnRest::Visible(frame) => Some((*slot, *frame)),
            _ => None,
        })
    }

    /// Columns out of view, peeking or parked.
    pub fn hidden(&self) -> impl Iterator<Item = SlotId> + '_ {
        self.columns
            .iter()
            .filter(|(_, rest)| !matches!(rest, ColumnRest::Visible(_)))
            .map(|(slot, _)| *slot)
    }

    #[must_use]
    pub fn rest(&self, slot: SlotId) -> Option<ColumnRest> {
        self.columns
            .iter()
            .find(|(column, _)| *column == slot)
            .map(|(_, rest)| *rest)
    }
}

/// Where every column of `ribbon` rests in `output` with the camera at `camera`.
#[must_use]
pub fn rest_layout(
    output: Rect,
    bounds: &ScrollingDisplayBounds,
    ribbon: &[RibbonColumn],
    camera: f64,
) -> StripLayout {
    let (left, right) = (output.x, output.x + output.width);
    let columns: Vec<_> = ribbon
        .iter()
        .map(|column| {
            let desired = (f64::from(output.x) + f64::from(column.x) - camera).round() as i32;
            let frame = Rect {
                x: bounds.clamp_column_x(desired, column.width, PEEK_PX),
                y: output.y,
                width: column.width,
                height: output.height,
            };
            let overlap = (desired + column.width).min(right) - desired.max(left);
            let shared = if desired >= right {
                bounds.adjacent_right
            } else {
                bounds.adjacent_left
            };
            let rest = if overlap > 0 {
                ColumnRest::Visible(frame)
            } else if shared {
                ColumnRest::Parked
            } else {
                ColumnRest::Peeking(frame)
            };
            (column.slot, rest)
        })
        .collect();
    let anchor = columns
        .iter()
        .find(|(_, rest)| matches!(rest, ColumnRest::Visible(_)))
        .map(|(slot, _)| *slot);
    StripLayout { columns, anchor }
}

/// One window riding a [`StripMotion`]: its column's resting ribbon position and width, and a
/// spring carrying it there from where it was before the ribbon changed (a neighbor resized or
/// columns reordered).
#[derive(Clone, Debug, PartialEq)]
pub struct MotionColumn {
    pub window: PhysicalWindowId,
    pub x: i32,
    pub width: i32,
    /// Offset from `x`, springing to zero.
    pub shift: Option<CameraSpring>,
}

impl MotionColumn {
    fn world_x(&self, now: Instant) -> f64 {
        f64::from(self.x) + self.shift.map_or(0.0, |shift| shift.position(now))
    }
}

/// See [`StripMotion::curtain`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Curtain {
    /// Columns held inside the display at a shared edge for part of the glide.
    pub tucked: Vec<PhysicalWindowId>,
    /// Columns that slide across a tucked one and must stay above it.
    pub over: Vec<PhysicalWindowId>,
}

/// A strip's camera glide, handed from the worker to the main thread. Closed form: the main
/// thread samples it at each display frame without asking the worker anything, and the worker
/// samples the same springs to retarget mid-glide.
#[derive(Clone, Debug, PartialEq)]
pub struct StripMotion {
    pub workspace: WorkspaceId,
    pub display_id: String,
    pub output: Rect,
    pub bounds: ScrollingDisplayBounds,
    pub camera: CameraSpring,
    pub columns: Vec<MotionColumn>,
    /// When the worker aimed the springs. The main thread starts presenting a little later and
    /// measures that delay from here.
    pub issued: Instant,
}

impl StripMotion {
    /// Each window's top-left corner at `now`.
    #[must_use]
    pub fn positions(&self, now: Instant) -> Vec<(PhysicalWindowId, (i32, i32))> {
        let camera = self.camera.position(now);
        self.columns
            .iter()
            .map(|column| {
                let x = column_x(
                    self.output,
                    &self.bounds,
                    column.world_x(now),
                    column.width,
                    camera,
                );
                (column.window.clone(), (x, self.output.y))
            })
            .collect()
    }

    /// Each window's top-left corner once everything has arrived.
    #[must_use]
    pub fn final_positions(&self) -> Vec<(PhysicalWindowId, (i32, i32))> {
        let camera = self.camera.target();
        self.columns
            .iter()
            .map(|column| {
                let x = column_x(
                    self.output,
                    &self.bounds,
                    f64::from(column.x),
                    column.width,
                    camera,
                );
                (column.window.clone(), (x, self.output.y))
            })
            .collect()
    }

    /// What the glide must keep stacked: the columns it holds against an edge shared with
    /// another display, now or once it arrives, and the columns that slide across them.
    ///
    /// A column can't cross a shared edge without showing on the other display, so it waits at
    /// that edge (inside the display, or in [`crate::motion::SharedEdge::Slide`] half past it;
    /// see [`column_x`]) while its neighbor slides over or off it. With the neighbor on top, a
    /// leaving column is covered before it is parked.
    ///
    /// `front`, the column taking focus, is never tucked under anything: focus raises it at
    /// once so keys reach it without waiting for the glide, and it glides in on top.
    #[must_use]
    pub fn curtain(&self, now: Instant, front: Option<&PhysicalWindowId>) -> Curtain {
        let (camera_now, camera_end) = (self.camera.position(now), self.camera.target());
        let place = |world_x: f64, width: i32, camera: f64| {
            let desired = (f64::from(self.output.x) + world_x - camera).round() as i32;
            let x = column_x(self.output, &self.bounds, world_x, width, camera);
            (x, self.bounds.holds_at_shared_edge(desired, width, PEEK_PX))
        };
        let spans: Vec<_> = self
            .columns
            .iter()
            .map(|column| {
                let (now_x, tucked_now) = place(column.world_x(now), column.width, camera_now);
                let (end_x, tucked_end) = place(f64::from(column.x), column.width, camera_end);
                let tucked = (front != Some(&column.window) && (tucked_now || tucked_end))
                    .then_some(if tucked_now { now_x } else { end_x });
                (
                    column,
                    now_x.min(end_x),
                    now_x.max(end_x) + column.width,
                    tucked,
                )
            })
            .collect();
        let mut curtain = Curtain::default();
        for (column, from, to, tucked) in &spans {
            if front == Some(&column.window) {
                continue;
            }
            if tucked.is_some() {
                curtain.tucked.push(column.window.clone());
                continue;
            }
            let crosses = spans.iter().any(|(other, _, _, spot)| {
                spot.is_some_and(|x| {
                    other.window != column.window && *from < x + other.width && x < *to
                })
            });
            if crosses {
                curtain.over.push(column.window.clone());
            }
        }
        curtain
    }

    /// Whether the camera and every column have arrived.
    #[must_use]
    pub fn is_settled(&self, now: Instant) -> bool {
        self.camera.is_settled(now, SETTLE_TOLERANCE_PT)
            && self
                .columns
                .iter()
                .filter_map(|column| column.shift)
                .all(|shift| shift.is_settled(now, SETTLE_TOLERANCE_PT))
    }

    /// The first moment at or after `from` when the motion counts as settled, found in steps of
    /// a 240 Hz frame and capped at [`MAX_MOTION`].
    #[must_use]
    pub fn settles_at(&self, from: Instant) -> Instant {
        let step = Duration::from_micros(4_167);
        let mut at = from;
        while at < from + MAX_MOTION {
            if self.is_settled(at) {
                return at;
            }
            at += step;
        }
        from + MAX_MOTION
    }
}

/// A workspace's strip: whether it is on, each column's width and where the viewport sits.
/// Persisted in `workspace-state.json`; every field defaults, so older files load unchanged.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StripState {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub enabled: bool,
    /// The width of columns nobody has sized: the last width picked on this workspace.
    #[serde(default, skip_serializing_if = "is_default_width")]
    pub default_width: StripWidth,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub widths: Vec<(SlotId, StripWidth)>,
    /// The leftmost column in view when the camera last rested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<SlotId>,
}

fn is_default_width(width: &StripWidth) -> bool {
    *width == StripWidth::default()
}

impl StripState {
    #[must_use]
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    #[must_use]
    pub fn width(&self, slot: SlotId) -> StripWidth {
        self.widths
            .iter()
            .find(|(sized, _)| *sized == slot)
            .map_or(self.default_width, |(_, width)| *width)
    }

    /// Sizes `slot`; columns nobody sized follow the same width from now on.
    pub fn set_width(&mut self, slot: SlotId, width: StripWidth) {
        self.default_width = width;
        self.widths.retain(|(sized, _)| *sized != slot);
        self.widths.push((slot, width));
        self.widths.sort_by_key(|(sized, _)| *sized);
    }

    /// Forgets widths and the anchor of slots that no longer exist.
    pub fn retain_slots(&mut self, exists: impl Fn(SlotId) -> bool) {
        self.widths.retain(|(slot, _)| exists(*slot));
        self.anchor = self.anchor.filter(|slot| exists(*slot));
    }
}

#[cfg(test)]
mod tests;
