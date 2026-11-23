use std::{fmt, str::FromStr};

use crate::{PhysicalWindowId, Rect, WorkspaceId};

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Left,
    Down,
    Up,
    Right,
}

impl fmt::Display for Direction {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Left => "left",
            Self::Down => "down",
            Self::Up => "up",
            Self::Right => "right",
        })
    }
}

impl FromStr for Direction {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "left" => Ok(Self::Left),
            "down" => Ok(Self::Down),
            "up" => Ok(Self::Up),
            "right" => Ok(Self::Right),
            _ => Err(format!("invalid direction {value:?}")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct NavigationTile {
    pub workspace: WorkspaceId,
    pub window: PhysicalWindowId,
    pub frame: Rect,
}

/// Tiles this close past the current tile's edge still count as lying beyond
/// it, absorbing rounding in computed splits.
const EDGE_TOLERANCE: i64 = 8;

/// The tile a directional key moves to from the focused tile.
///
/// Neighbors whose perpendicular band overlaps the current tile win, nearest
/// edge first, the way i3 and AeroSpace pick them: from either tile of a
/// stacked column, left always reaches the column beside it, however narrow
/// or wide either is. Only when no tile shares the band does a tile off to
/// the side count, which covers staggered displays.
#[must_use]
pub fn target<'a>(
    tiles: &'a [NavigationTile],
    focused: &PhysicalWindowId,
    direction: Direction,
) -> Option<&'a NavigationTile> {
    let current = tiles.iter().find(|tile| &tile.window == focused)?.frame;
    neighbor(
        tiles.iter().filter(|tile| &tile.window != focused),
        current,
        direction,
    )
}

/// The tile a directional key moves to when focus is on a window that isn't a
/// tile (a floating window, dialog or popup) at `from`: its neighbor in that
/// direction, else the nearest tile whose center lies that way, else the
/// nearest tile, so the key always brings focus back onto the tiles.
#[must_use]
pub fn rescue(
    tiles: &[NavigationTile],
    from: Rect,
    direction: Direction,
) -> Option<&NavigationTile> {
    neighbor(tiles.iter(), from, direction)
        .or_else(|| {
            tiles
                .iter()
                .map(|tile| (Reach::new(from, tile.frame, direction), tile))
                .filter(|(reach, _)| reach.travel > 0)
                .min_by_key(|(reach, _)| (reach.band_gap(), reach.travel, reach.offset))
                .map(|(_, tile)| tile)
        })
        .or_else(|| {
            tiles
                .iter()
                .min_by_key(|tile| center_distance(from, tile.frame))
        })
}

fn neighbor<'a>(
    candidates: impl Iterator<Item = &'a NavigationTile>,
    from: Rect,
    direction: Direction,
) -> Option<&'a NavigationTile> {
    let ahead = candidates
        .map(|tile| (Reach::new(from, tile.frame, direction), tile))
        .filter(|(reach, _)| reach.gap >= -EDGE_TOLERANCE && reach.travel > 0)
        .collect::<Vec<_>>();
    ahead
        .iter()
        .filter(|(reach, _)| reach.overlap > EDGE_TOLERANCE)
        .min_by_key(|(reach, _)| (reach.gap.max(0), reach.offset, reach.travel))
        .or_else(|| {
            ahead
                .iter()
                .min_by_key(|(reach, _)| (reach.band_gap(), reach.gap.max(0), reach.offset))
        })
        .map(|(_, tile)| *tile)
}

/// Where a candidate lies relative to the current frame, measured as if
/// travelling right/down along the requested direction. Centers are doubled
/// to stay in integers.
struct Reach {
    /// From the current frame's far edge to the candidate's near edge.
    gap: i64,
    /// From the current center to the candidate center along the direction.
    travel: i64,
    /// How much the two frames share across the direction; negative is a gap.
    overlap: i64,
    /// Distance between the two centers across the direction.
    offset: i64,
}

impl Reach {
    fn new(from: Rect, to: Rect, direction: Direction) -> Self {
        let (from_along, from_across) = spans(from, direction);
        let (to_along, to_across) = spans(to, direction);
        Self {
            gap: to_along.0 - from_along.1,
            travel: (to_along.0 + to_along.1) - (from_along.0 + from_along.1),
            overlap: to_across.1.min(from_across.1) - to_across.0.max(from_across.0),
            offset: ((to_across.0 + to_across.1) - (from_across.0 + from_across.1)).abs(),
        }
    }

    fn band_gap(&self) -> i64 {
        (-self.overlap).max(0)
    }
}

/// A frame's span along the direction (mirrored for left and up, so travel is
/// always toward larger values) and across it.
fn spans(rect: Rect, direction: Direction) -> ((i64, i64), (i64, i64)) {
    let x = (i64::from(rect.x), i64::from(rect.x) + i64::from(rect.width));
    let y = (
        i64::from(rect.y),
        i64::from(rect.y) + i64::from(rect.height),
    );
    match direction {
        Direction::Right => (x, y),
        Direction::Left => ((-x.1, -x.0), y),
        Direction::Down => (y, x),
        Direction::Up => ((-y.1, -y.0), x),
    }
}

fn center_distance(a: Rect, b: Rect) -> i64 {
    let dx = (i64::from(a.x) * 2 + i64::from(a.width)) - (i64::from(b.x) * 2 + i64::from(b.width));
    let dy =
        (i64::from(a.y) * 2 + i64::from(a.height)) - (i64::from(b.y) * 2 + i64::from(b.height));
    dx * dx + dy * dy
}

#[cfg(test)]
mod tests {
    use crate::AppInstanceId;

    use super::*;

    fn tile(id: u32, workspace: u8, x: i32, y: i32) -> NavigationTile {
        sized(id, workspace, x, y, 500, 500)
    }

    fn sized(id: u32, workspace: u8, x: i32, y: i32, width: i32, height: i32) -> NavigationTile {
        NavigationTile {
            workspace: WorkspaceId::new(workspace).unwrap(),
            window: PhysicalWindowId {
                app: AppInstanceId {
                    pid: i32::try_from(id).unwrap(),
                    launched_at: 1,
                },
                window_id: id,
            },
            frame: Rect {
                x,
                y,
                width,
                height,
            },
        }
    }

    fn id(tiles: &[NavigationTile], from: u32, direction: Direction) -> Option<u32> {
        let focused = &tiles
            .iter()
            .find(|tile| tile.window.window_id == from)
            .unwrap()
            .window;
        target(tiles, focused, direction).map(|tile| tile.window.window_id)
    }

    /// A narrow master column beside three stacked tiles on a portrait
    /// display, 8 px gaps.
    fn master_and_stack() -> Vec<NavigationTile> {
        vec![
            sized(1, 1, 0, 0, 600, 2560),
            sized(2, 1, 608, 0, 832, 848),
            sized(3, 1, 608, 856, 832, 848),
            sized(4, 1, 608, 1712, 832, 848),
        ]
    }

    #[test]
    fn left_from_any_tile_of_a_stack_reaches_the_master_column() {
        let tiles = master_and_stack();
        for stacked in [2, 3, 4] {
            assert_eq!(
                id(&tiles, stacked, Direction::Left),
                Some(1),
                "from {stacked}"
            );
            assert_eq!(
                id(&tiles, stacked, Direction::Right),
                None,
                "from {stacked}"
            );
        }
        assert_eq!(id(&tiles, 2, Direction::Down), Some(3));
        assert_eq!(id(&tiles, 4, Direction::Up), Some(3));
        assert_eq!(id(&tiles, 2, Direction::Up), None);
        assert_eq!(id(&tiles, 4, Direction::Down), None);
        // The master reaches the stacked tile level with its center.
        assert_eq!(id(&tiles, 1, Direction::Right), Some(3));
        assert_eq!(id(&tiles, 1, Direction::Left), None);
    }

    #[test]
    fn down_from_any_tile_of_a_row_reaches_the_wide_tile_below() {
        let tiles = vec![
            sized(1, 1, 0, 0, 848, 600),
            sized(2, 1, 856, 0, 848, 600),
            sized(3, 1, 1712, 0, 848, 600),
            sized(4, 1, 0, 608, 2560, 832),
        ];
        for top in [1, 2, 3] {
            assert_eq!(id(&tiles, top, Direction::Down), Some(4), "from {top}");
            assert_eq!(id(&tiles, top, Direction::Up), None, "from {top}");
        }
        assert_eq!(id(&tiles, 1, Direction::Right), Some(2));
        assert_eq!(id(&tiles, 3, Direction::Left), Some(2));
        assert_eq!(id(&tiles, 4, Direction::Up), Some(2));
        assert_eq!(id(&tiles, 4, Direction::Down), None);
        assert_eq!(id(&tiles, 4, Direction::Left), None);
        assert_eq!(id(&tiles, 4, Direction::Right), None);
    }

    #[test]
    fn resized_panes_keep_their_adjacent_neighbors() {
        // A 20% left column split top/bottom beside an 80% column.
        let tiles = vec![
            sized(1, 1, 0, 0, 512, 300),
            sized(2, 1, 0, 308, 512, 1132),
            sized(3, 1, 520, 0, 2040, 1440),
        ];
        assert_eq!(id(&tiles, 1, Direction::Right), Some(3));
        assert_eq!(id(&tiles, 2, Direction::Right), Some(3));
        assert_eq!(id(&tiles, 3, Direction::Left), Some(2));
        assert_eq!(id(&tiles, 1, Direction::Down), Some(2));
        assert_eq!(id(&tiles, 2, Direction::Up), Some(1));
    }

    #[test]
    fn grid_moves_one_cell_at_a_time_and_stops_at_the_edges() {
        let tiles = vec![
            sized(1, 1, 0, 0, 1000, 700),
            sized(2, 1, 1008, 0, 1000, 700),
            sized(3, 1, 0, 708, 1000, 700),
            sized(4, 1, 1008, 708, 1000, 700),
        ];
        assert_eq!(id(&tiles, 1, Direction::Right), Some(2));
        assert_eq!(id(&tiles, 1, Direction::Down), Some(3));
        assert_eq!(id(&tiles, 4, Direction::Left), Some(3));
        assert_eq!(id(&tiles, 4, Direction::Up), Some(2));
        assert_eq!(id(&tiles, 1, Direction::Left), None);
        assert_eq!(id(&tiles, 1, Direction::Up), None);
        assert_eq!(id(&tiles, 4, Direction::Right), None);
        assert_eq!(id(&tiles, 4, Direction::Down), None);
    }

    #[test]
    fn crosses_to_the_nearest_tile_on_a_staggered_display() {
        // A display split in two, one to its lower right sharing no band, and
        // one below it.
        let tiles = vec![
            sized(1, 1, 0, 0, 1000, 600),
            sized(2, 1, 1008, 0, 1000, 600),
            sized(3, 6, 2600, 700, 1400, 900),
            sized(4, 7, 600, 1800, 800, 400),
        ];
        assert_eq!(id(&tiles, 2, Direction::Right), Some(3));
        assert_eq!(id(&tiles, 3, Direction::Left), Some(2));
        assert_eq!(id(&tiles, 1, Direction::Down), Some(4));
        assert_eq!(id(&tiles, 2, Direction::Down), Some(4));
        assert_eq!(id(&tiles, 1, Direction::Left), None);
        assert_eq!(id(&tiles, 3, Direction::Right), None);
    }

    #[test]
    fn rescues_focus_from_a_floating_window_onto_the_tiles() {
        let tiles = vec![
            sized(1, 1, 0, 0, 1000, 800),
            sized(2, 1, 1008, 0, 1000, 800),
        ];
        let rescue_id =
            |from, direction| rescue(&tiles, from, direction).map(|tile| tile.window.window_id);
        // A dialog floating over the right tile.
        let dialog = Rect {
            x: 1200,
            y: 200,
            width: 400,
            height: 300,
        };
        assert_eq!(rescue_id(dialog, Direction::Left), Some(1));
        // A panel straddling both tiles still lands on the side pressed.
        let panel = Rect {
            x: 700,
            y: 300,
            width: 600,
            height: 200,
        };
        assert_eq!(rescue_id(panel, Direction::Left), Some(1));
        assert_eq!(rescue_id(panel, Direction::Right), Some(2));
        // Nothing lies that way: the nearest tile still takes focus.
        assert_eq!(rescue_id(dialog, Direction::Right), Some(2));
        assert_eq!(rescue(&[], dialog, Direction::Left), None);
    }

    #[test]
    fn chooses_geometric_neighbors_in_all_directions() {
        let center = tile(1, 1, 500, 500);
        let tiles = vec![
            center.clone(),
            tile(2, 1, 0, 500),
            tile(3, 1, 500, 1000),
            tile(4, 1, 500, 0),
            tile(5, 1, 1000, 500),
        ];

        assert_eq!(
            target(&tiles, &center.window, Direction::Left)
                .unwrap()
                .window
                .window_id,
            2
        );
        assert_eq!(
            target(&tiles, &center.window, Direction::Down)
                .unwrap()
                .window
                .window_id,
            3
        );
        assert_eq!(
            target(&tiles, &center.window, Direction::Up)
                .unwrap()
                .window
                .window_id,
            4
        );
        assert_eq!(
            target(&tiles, &center.window, Direction::Right)
                .unwrap()
                .window
                .window_id,
            5
        );
    }

    #[test]
    fn ignores_small_vertical_offset_between_side_by_side_displays() {
        let left = tile(1, 1, 0, 30);
        let right = tile(2, 6, 2560, 0);
        let tiles = vec![left.clone(), right.clone()];

        assert_eq!(target(&tiles, &left.window, Direction::Right), Some(&right));
        assert_eq!(target(&tiles, &left.window, Direction::Up), None);
        assert_eq!(target(&tiles, &right.window, Direction::Left), Some(&left));
        assert_eq!(target(&tiles, &right.window, Direction::Down), None);
    }

    #[test]
    fn direction_strings_round_trip() {
        for direction in [
            Direction::Left,
            Direction::Down,
            Direction::Up,
            Direction::Right,
        ] {
            assert_eq!(direction.to_string().parse(), Ok(direction));
        }
        assert_eq!(
            "diagonal".parse::<Direction>(),
            Err("invalid direction \"diagonal\"".into())
        );
    }
}
