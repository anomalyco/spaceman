use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Axis {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    #[must_use]
    pub fn approximately_equals(self, other: Self, tolerance: i32) -> bool {
        self.position_approximately_equals(other, tolerance)
            && (self.width - other.width).abs() <= tolerance
            && (self.height - other.height).abs() <= tolerance
    }

    /// Position-only tolerance comparison; sizes may legitimately differ when
    /// an application constrains its own dimensions.
    #[must_use]
    pub fn position_approximately_equals(self, other: Self, tolerance: i32) -> bool {
        (self.x - other.x).abs() <= tolerance && (self.y - other.y).abs() <= tolerance
    }

    #[must_use]
    pub const fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    #[must_use]
    pub const fn contains_center(self, other: Self) -> bool {
        self.contains(other.x + other.width / 2, other.y + other.height / 2)
    }
}

/// Width and height of the intersection of two rectangles; not positive when disjoint.
fn overlap(a: Rect, b: Rect) -> (i64, i64) {
    let width = (i64::from(a.x) + i64::from(a.width)).min(i64::from(b.x) + i64::from(b.width))
        - i64::from(a.x.max(b.x));
    let height = (i64::from(a.y) + i64::from(a.height)).min(i64::from(b.y) + i64::from(b.height))
        - i64::from(a.y.max(b.y));
    (width, height)
}

/// Whether more of `frame` than the one-pixel parking strip shows on any display.
/// Center containment is not enough: a window wider than its tile overflows the
/// display edge with its center outside while most of it stays visible.
#[must_use]
pub fn shows_on(frame: Rect, displays: &[Rect]) -> bool {
    displays.iter().any(|display| {
        let (width, height) = overlap(frame, *display);
        width > 1 && height > 0
    })
}

/// `frame` itself while it shows on a display, otherwise moved (size kept) inside
/// `fallback`: the display it was recorded on has been disconnected.
#[must_use]
pub fn onto_displays(frame: Rect, displays: &[Rect], fallback: Rect) -> Rect {
    if shows_on(frame, displays) {
        return frame;
    }
    Rect {
        x: frame.x.clamp(
            fallback.x,
            fallback.x + (fallback.width - frame.width).max(0),
        ),
        y: frame.y.clamp(
            fallback.y,
            fallback.y + (fallback.height - frame.height).max(0),
        ),
        ..frame
    }
}

/// Whether a window at `actual` sits where parking it at `target` leaves it.
/// macOS keeps a window's title bar on screen, so it may hold the window above
/// the requested bottom corner; the horizontal position is what hides it.
#[must_use]
pub fn parked_at(actual: Rect, target: Rect, tolerance: i32) -> bool {
    (actual.x - target.x).abs() <= tolerance && actual.y <= target.y + tolerance
}

#[must_use]
pub fn parked_frame(output: Rect, displays: &[Rect], window: Rect) -> Rect {
    // Keep one pixel of a top corner at a display's bottom corner so macOS
    // does not clamp the window back on-screen; a strip along the whole edge
    // would show a sliver of every hidden window. Score whole rectangles, not
    // corners: the nearest output may have a neighbor on both sides.
    std::iter::once(&output)
        .chain(displays)
        .flat_map(|display| {
            [true, false].map(|right| {
                let strip = window.width.clamp(0, 2) / 2;
                let frame = Rect {
                    x: if right {
                        display.x + display.width - strip
                    } else {
                        display.x - window.width + strip
                    },
                    y: display.y + display.height - window.height.clamp(0, 2) / 2,
                    ..window
                };
                let distance = (i64::from(display.x) - i64::from(output.x)).abs()
                    + (i64::from(display.y) - i64::from(output.y)).abs();
                (frame, distance)
            })
        })
        .filter(|(frame, _)| {
            !shows_on(*frame, displays)
                && !displays
                    .iter()
                    .any(|display| display.contains_center(*frame))
        })
        .min_by_key(|(_, distance)| *distance)
        .map_or(
            Rect {
                x: output.x + output.width,
                y: output.y,
                ..window
            },
            |(frame, _)| frame,
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_overflowing_the_display_edge_still_shows() {
        let display = Rect {
            width: 1000,
            height: 800,
            ..Rect::default()
        };
        let overflowing = Rect {
            x: 700,
            y: 20,
            width: 700,
            height: 300,
        };
        let parked = Rect {
            x: 999,
            ..overflowing
        };

        assert!(!display.contains_center(overflowing));
        assert!(shows_on(overflowing, &[display]));
        assert!(!shows_on(parked, &[display]));
    }

    #[test]
    fn frames_on_a_disconnected_display_move_onto_the_fallback() {
        let main = Rect {
            width: 1000,
            height: 800,
            ..Rect::default()
        };
        let on_main = Rect {
            x: 50,
            y: 20,
            width: 400,
            height: 300,
        };
        let on_unplugged = Rect { x: 1050, ..on_main };

        assert_eq!(onto_displays(on_main, &[main], main), on_main);
        assert_eq!(onto_displays(on_unplugged, &[main], main).x, 600);
        assert_eq!(onto_displays(on_unplugged, &[main], main).y, 20);
    }

    #[test]
    fn parks_beyond_outer_edges_without_requesting_a_clamped_position() {
        let left = Rect {
            width: 1000,
            height: 800,
            ..Rect::default()
        };
        let right = Rect {
            x: 1000,
            width: 1000,
            height: 800,
            ..Rect::default()
        };
        let window = Rect {
            width: 400,
            height: 300,
            ..Rect::default()
        };

        assert_eq!(parked_frame(left, &[left, right], window).x, -399);
        assert_eq!(parked_frame(right, &[left, right], window).x, 1999);
        assert_eq!(parked_frame(left, &[left, right], window).y, 799);
    }

    #[test]
    fn parking_leaves_a_single_corner_pixel_on_screen() {
        let display = Rect {
            y: 40,
            width: 1920,
            height: 1000,
            ..Rect::default()
        };
        for (width, height) in [(400, 300), (1920, 1000), (3000, 1500)] {
            let window = Rect {
                width,
                height,
                ..Rect::default()
            };
            let parked = parked_frame(display, &[display], window);

            assert_eq!(overlap(parked, display), (1, 1), "{parked:?}");
        }
    }

    #[test]
    fn a_window_held_above_its_parking_corner_is_still_parked() {
        let target = Rect {
            x: 1919,
            y: 1079,
            width: 600,
            height: 400,
        };

        assert!(parked_at(target, target, 1));
        assert!(parked_at(Rect { y: 1014, ..target }, target, 1));
        assert!(!parked_at(Rect { y: 1081, ..target }, target, 1));
        assert!(!parked_at(Rect { x: 1900, ..target }, target, 1));
    }

    #[test]
    fn parking_avoids_neighbors_in_middle_and_staggered_display_topologies() {
        for offsets in [[0, 0, 0], [-400, 0, 300], [0, 900, 0]] {
            let displays = offsets
                .into_iter()
                .enumerate()
                .map(|(index, y)| Rect {
                    x: (index as i32 - 1) * 1000,
                    y,
                    width: 1000,
                    height: 800,
                })
                .collect::<Vec<_>>();
            for output in &displays {
                for width in [100, 400, 1200, 3000] {
                    for height in [100, 800, 1500] {
                        let window = Rect {
                            width,
                            height,
                            ..Rect::default()
                        };
                        let parked = parked_frame(*output, &displays, window);
                        for display in &displays {
                            let overlap_width = (parked.x + parked.width)
                                .min(display.x + display.width)
                                - parked.x.max(display.x);
                            let overlap_height = (parked.y + parked.height)
                                .min(display.y + display.height)
                                - parked.y.max(display.y);
                            assert!(
                                overlap_width <= 1 || overlap_height <= 0,
                                "parked {parked:?} overlaps {display:?}"
                            );
                            assert!(!display.contains_center(parked));
                        }
                    }
                }
            }
        }
    }
}
