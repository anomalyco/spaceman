//! Where the card sits: centred on the part of its window you can see, and kept whole inside
//! the display.
//!
//! The card takes the centre of the largest part of its window that nothing in front covers,
//! when the card fits there, and the window's own centre when it fits nowhere. Then it moves
//! the least that brings it whole inside the display, so a narrow window, or one partly off
//! the display, still shows all of its card.
//!
//! The same windows always give the same place.

use spaceman::Rect;

/// The closest a card comes to its display's edges, in points.
pub const EDGE_MARGIN: f32 = 12.0;
/// The room a card keeps from the edges of the part of the window it sits on.
pub const CARD_GAP: f32 = 8.0;
/// The most free parts of a window kept while the windows in front are cut out of it.
const MAX_PARTS: usize = 8;

/// A rectangle in display-local points.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Area {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Area {
    fn of(rect: Rect, screen: Rect) -> Self {
        Self {
            x: (rect.x - screen.x) as f32,
            y: (rect.y - screen.y) as f32,
            w: rect.width as f32,
            h: rect.height as f32,
        }
    }

    const fn right(self) -> f32 {
        self.x + self.w
    }

    const fn bottom(self) -> f32 {
        self.y + self.h
    }

    fn area(self) -> f32 {
        self.w.max(0.0) * self.h.max(0.0)
    }

    /// The part of `self` inside `other`; empty when they are apart.
    fn within(self, other: Self) -> Self {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        Self {
            x,
            y,
            w: (self.right().min(other.right()) - x).max(0.0),
            h: (self.bottom().min(other.bottom()) - y).max(0.0),
        }
    }

    /// What is left of `self` beside `cut`: the strips to its left, right, top and bottom.
    fn without(self, cut: Self) -> Vec<Self> {
        if self.within(cut).area() == 0.0 {
            return vec![self];
        }
        [
            Self {
                w: cut.x - self.x,
                ..self
            },
            Self {
                x: cut.right(),
                w: self.right() - cut.right(),
                ..self
            },
            Self {
                h: cut.y - self.y,
                ..self
            },
            Self {
                y: cut.bottom(),
                h: self.bottom() - cut.bottom(),
                ..self
            },
        ]
        .into_iter()
        .filter(|part| part.w > 0.0 && part.h > 0.0)
        .collect()
    }
}

/// The lowest and highest origin a card `size` long takes along a display axis `length` long.
/// A card longer than the display sits at its near margin.
fn span(length: f32, size: f32) -> (f32, f32) {
    (EDGE_MARGIN, (length - size - EDGE_MARGIN).max(EDGE_MARGIN))
}

/// The card's top-left corner in display-local points: centred on `area`, then clamped so a
/// narrow window, or one partly off the display, keeps its card whole on `screen` (the
/// display's full bounds). A card wider than the display sits at its left margin.
fn centred(area: Area, screen: Rect, card: (f32, f32)) -> (f32, f32) {
    let axis = |start: f32, length: f32, screen_length: i32, size: f32| {
        let (low, high) = span(screen_length as f32, size);
        (start + (length - size) / 2.0).clamp(low, high).round()
    };
    (
        axis(area.x, area.w, screen.width, card.0),
        axis(area.y, area.h, screen.height, card.1),
    )
}

/// The largest part of `window` that no window in `fronts` covers and that holds a card
/// `size` big with room to spare, or the whole window when no part does.
fn open_part(window: Area, fronts: impl Iterator<Item = Area>, size: (f32, f32)) -> Area {
    let mut parts = vec![window];
    for front in fronts {
        parts = parts
            .into_iter()
            .flat_map(|part| part.without(front))
            .collect();
        parts.sort_by(|a, b| b.area().total_cmp(&a.area()));
        parts.dedup();
        parts.truncate(MAX_PARTS);
    }
    parts
        .into_iter()
        .find(|part| part.w >= size.0 + 2.0 * CARD_GAP && part.h >= size.1 + 2.0 * CARD_GAP)
        .unwrap_or(window)
}

/// The card's top-left corner in display-local points, for a card `size` big on `window`
/// (global points) of the display whose bounds are `screen`. `fronts`: the windows in front
/// of it.
#[must_use]
pub fn place(screen: Rect, window: Rect, size: (f32, f32), fronts: &[Rect]) -> (f32, f32) {
    let display = Area {
        x: 0.0,
        y: 0.0,
        w: screen.width as f32,
        h: screen.height as f32,
    };
    let shown = match Area::of(window, screen).within(display) {
        part if part.area() > 0.0 => part,
        // Off the display altogether: its nearest corner.
        _ => Area::of(window, screen),
    };
    let fronts = fronts
        .iter()
        .map(|front| Area::of(*front, screen).within(display));
    centred(open_part(shown, fronts, size), screen, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Rect = Rect {
        x: 1000,
        y: 0,
        width: 1600,
        height: 1000,
    };
    const CARD: (f32, f32) = (340.0, 84.0);

    fn rect(x: i32, y: i32, width: i32, height: i32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    /// The card as an area, at the place it was given.
    fn card(origin: (f32, f32), size: (f32, f32)) -> Area {
        Area {
            x: origin.0,
            y: origin.1,
            w: size.0,
            h: size.1,
        }
    }

    #[test]
    fn a_card_centres_on_its_window_in_display_local_points() {
        // A window at (1400, 200) 800×600 on a display whose origin is (1000, 0).
        let origin = place(SCREEN, rect(1400, 200, 800, 600), (200.0, 100.0), &[]);
        assert_eq!(origin, (700.0, 450.0));
    }

    #[test]
    fn a_window_at_an_edge_keeps_its_card_whole_inside_the_display() {
        let size = (240.0, 110.0);
        // Hard against the top-left corner, narrower than its card.
        assert_eq!(
            place(SCREEN, rect(1000, 0, 80, 40), size, &[]),
            (EDGE_MARGIN, EDGE_MARGIN)
        );
        // Mostly off the right and bottom edges.
        let (x, y) = place(SCREEN, rect(2500, 950, 400, 300), size, &[]);
        assert_eq!(x, 1600.0 - 240.0 - EDGE_MARGIN);
        assert_eq!(y, 1000.0 - 110.0 - EDGE_MARGIN);
    }

    #[test]
    fn a_card_wider_than_the_display_sits_at_its_margin() {
        let tiny = rect(0, 0, 200, 150);
        assert_eq!(
            place(tiny, rect(0, 0, 200, 150), (400.0, 300.0), &[]),
            (EDGE_MARGIN, EDGE_MARGIN)
        );
    }

    #[test]
    fn a_window_partly_off_the_display_centres_its_card_on_the_part_that_shows() {
        // 600 wide, 400 of it off the left edge: the 200 points that show are narrower than
        // the card, so it rests at the margin.
        let (x, _) = place(SCREEN, rect(600, 200, 600, 600), CARD, &[]);
        assert_eq!(x, EDGE_MARGIN);
        // A wide enough part: centred on what shows (x 0..800 of a 1000-wide window).
        let (x, _) = place(SCREEN, rect(800, 200, 1000, 600), CARD, &[]);
        assert_eq!(x, (800.0 - CARD.0) / 2.0);
    }

    #[test]
    fn a_tile_under_a_float_shows_its_card_beside_the_float() {
        // A float over the middle of a full-display tile.
        let (tile, float) = (rect(1000, 0, 1600, 1000), rect(1500, 300, 600, 400));
        let origin = place(SCREEN, tile, CARD, &[float]);
        // The card takes the largest part the float leaves (the strips beside it tie, and the
        // left one wins), not the float's middle.
        assert_eq!(origin, ((500.0 - CARD.0) / 2.0, (1000.0 - CARD.1) / 2.0));
        let covered = card(origin, CARD).within(Area::of(float, SCREEN));
        assert_eq!(covered.area(), 0.0);
        // The float's own card is centred on the float: nothing is in front of it.
        assert_eq!(
            place(SCREEN, float, CARD, &[]),
            (
                500.0 + (600.0 - CARD.0) / 2.0,
                300.0 + (400.0 - CARD.1) / 2.0
            )
        );
    }

    #[test]
    fn a_float_behind_another_shows_its_card_on_the_part_that_shows() {
        // The back float's right 400 points are under the front one.
        let (back, front) = (rect(1100, 300, 900, 400), rect(1600, 250, 600, 500));
        let origin = place(SCREEN, back, CARD, &[front]);
        // Centred on the 500 points of it left showing, not under the other.
        assert_eq!(
            origin,
            (
                100.0 + (500.0 - CARD.0) / 2.0,
                300.0 + (400.0 - CARD.1) / 2.0
            )
        );
    }

    #[test]
    fn a_window_mostly_covered_keeps_its_card_on_its_own_centre() {
        // Nothing the float leaves of the tile holds a card: the tile's centre, as if alone.
        let (tile, float) = (rect(1200, 200, 420, 300), rect(1230, 230, 380, 240));
        assert_eq!(
            place(SCREEN, tile, CARD, &[float]),
            place(SCREEN, tile, CARD, &[])
        );
    }

    #[test]
    fn the_same_windows_always_give_the_same_place() {
        let tile = rect(1000, 0, 800, 500);
        let fronts = [rect(1300, 200, 500, 400), rect(1320, 220, 470, 400)];
        let first = place(SCREEN, tile, CARD, &fronts);
        assert_eq!(first, place(SCREEN, tile, CARD, &fronts));
        let inside = card(first, CARD).within(Area {
            x: EDGE_MARGIN,
            y: EDGE_MARGIN,
            w: 1600.0 - 2.0 * EDGE_MARGIN,
            h: 1000.0 - 2.0 * EDGE_MARGIN,
        });
        assert_eq!(inside.area(), CARD.0 * CARD.1);
    }
}
