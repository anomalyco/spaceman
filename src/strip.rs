//! Scrolling Strip, a Spaceman Lab: a workspace's tiles as one horizontal row of wide columns.
//! At most two columns are on screen, filling the display edge to edge; the rest are parked
//! off-screen by the same durable parking that hides inactive workspaces. No private APIs and no
//! frames past a display edge: every visible column sits wholly inside the display.

use serde::{Deserialize, Serialize};

use crate::{Rect, SlotId};

/// A column's preferred width while it is the focused one.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StripWidth {
    /// 50%, beside an equal neighbor.
    Half,
    /// 66%, with the neighbor peeking in the remaining 34%.
    #[default]
    TwoThirds,
    /// 100%: the column alone, its neighbors one step away.
    Full,
}

impl StripWidth {
    #[must_use]
    pub const fn narrower(self) -> Self {
        match self {
            Self::Full => Self::TwoThirds,
            Self::TwoThirds | Self::Half => Self::Half,
        }
    }

    #[must_use]
    pub const fn wider(self) -> Self {
        match self {
            Self::Half => Self::TwoThirds,
            Self::TwoThirds | Self::Full => Self::Full,
        }
    }

    #[must_use]
    pub const fn percent(self) -> i32 {
        match self {
            Self::Half => 50,
            Self::TwoThirds => 66,
            Self::Full => 100,
        }
    }

    /// This column's share of `width` points.
    #[must_use]
    pub const fn share(self, width: i32) -> i32 {
        width.saturating_mul(self.percent()) / 100
    }
}

/// One column of the strip: a bound slot in layout order and its preferred width.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StripColumn {
    pub slot: SlotId,
    pub width: StripWidth,
}

/// Where each column goes for one viewport.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StripLayout {
    /// On-screen columns, left to right, each wholly inside the area.
    pub visible: Vec<(SlotId, Rect)>,
    /// Columns outside the viewport, in strip order. They are parked.
    pub hidden: Vec<SlotId>,
    /// The leftmost visible column: the next layout's viewport anchor.
    pub anchor: Option<SlotId>,
}

/// Lays out `columns` (strip order) in `area` around the `active` column.
///
/// A single column fills the area. A `Full` active column shows alone. Otherwise two adjacent
/// columns share the area: the active one at its preferred width, its neighbor in the rest. The
/// viewport stays where `anchor` put it while the active column is already inside it, and
/// otherwise slides the least distance that brings the active column in, entering from the side
/// it was stepped toward.
#[must_use]
pub fn compute_strip_layout(
    area: Rect,
    columns: &[StripColumn],
    active: Option<SlotId>,
    anchor: Option<SlotId>,
) -> StripLayout {
    let position = |slot: Option<SlotId>| {
        slot.and_then(|slot| columns.iter().position(|column| column.slot == slot))
    };
    let Some(first) = columns.first() else {
        return StripLayout::default();
    };
    if columns.len() == 1 {
        return StripLayout {
            visible: vec![(first.slot, area)],
            hidden: Vec::new(),
            anchor: Some(first.slot),
        };
    }
    let anchor_index = position(anchor);
    let active_index = position(active).or(anchor_index).unwrap_or(0);
    let focused = columns[active_index];
    let (left, widths) = if focused.width == StripWidth::Full {
        (active_index, vec![area.width])
    } else {
        let last_left = columns.len() - 2;
        let left = match anchor_index {
            Some(anchor)
                if anchor <= last_left && (anchor..=anchor + 1).contains(&active_index) =>
            {
                anchor
            }
            Some(anchor) if active_index > anchor => active_index - 1,
            _ => active_index,
        }
        .min(last_left);
        let share = focused.width.share(area.width);
        let rest = area.width - share;
        (
            left,
            if left == active_index {
                vec![share, rest]
            } else {
                vec![rest, share]
            },
        )
    };
    let mut x = area.x;
    let visible = widths
        .iter()
        .enumerate()
        .map(|(offset, &width)| {
            let frame = Rect { x, width, ..area };
            x += width;
            (columns[left + offset].slot, frame)
        })
        .collect::<Vec<_>>();
    let hidden = columns
        .iter()
        .enumerate()
        .filter(|(index, _)| !(left..left + widths.len()).contains(index))
        .map(|(_, column)| column.slot)
        .collect();
    StripLayout {
        visible,
        hidden,
        anchor: Some(columns[left].slot),
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
