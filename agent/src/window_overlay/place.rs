//! Where each card sits: centred on the part of its window you can see, kept whole inside the
//! display, and never on another card.
//!
//! 1. A card takes the centre of the largest part of its window that nothing in front covers,
//!    when the card fits there, and the window's own centre when it fits nowhere. Floats are
//!    in front of tiles, and each float is in front of the floats given before it.
//! 2. The frontmost float places first, then the floats behind it, then the tiles in the order
//!    given. A card that would land on one already placed moves the least distance that clears
//!    every placed card, staying over its own window when it can and inside the display always.
//!
//! The same windows always give the same places.

use spaceman::Rect;

/// The closest a card comes to its display's edges, in points.
pub const EDGE_MARGIN: f32 = 12.0;
/// The closest two cards come to each other.
pub const CARD_GAP: f32 = 8.0;
/// What leaving its own window costs a card, in squared points: more than any move inside it.
const AWAY: f32 = 1.0e7;
/// The most free parts of a tile kept while floats are cut out of it.
const MAX_PARTS: usize = 8;

/// One card to place.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slot {
    /// The card's window, in global points.
    pub window: Rect,
    /// The card's size.
    pub size: (f32, f32),
    /// The window is a float: in front of the tiles, and of the floats before it in the list.
    pub front: bool,
}

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

    /// How much of `self` lies on `other` grown by `gap` on every side.
    fn overlap(self, other: Self, gap: f32) -> f32 {
        let w = self.right().min(other.right() + gap) - self.x.max(other.x - gap);
        let h = self.bottom().min(other.bottom() + gap) - self.y.max(other.y - gap);
        if w > 0.0 && h > 0.0 { w * h } else { 0.0 }
    }

    fn holds(self, (x, y): (f32, f32)) -> bool {
        x >= self.x && x <= self.right() && y >= self.y && y <= self.bottom()
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

/// Every card's top-left corner in display-local points, in `slots` order.
#[must_use]
pub fn place(screen: Rect, slots: &[Slot]) -> Vec<(f32, f32)> {
    let display = Area {
        x: 0.0,
        y: 0.0,
        w: screen.width as f32,
        h: screen.height as f32,
    };
    // Each float's place in the list and the part of it on the display.
    let floats: Vec<(usize, Area)> = slots
        .iter()
        .enumerate()
        .filter(|(_, slot)| slot.front)
        .map(|(index, slot)| (index, Area::of(slot.window, screen).within(display)))
        .collect();
    let tiles = (0..slots.len()).filter(|&index| !slots[index].front);
    // The frontmost float first, so the card moved off its window is one whose window is
    // mostly hidden anyway.
    let order = floats.iter().rev().map(|(index, _)| *index).chain(tiles);

    let mut placed: Vec<Area> = Vec::with_capacity(slots.len());
    let mut origins = vec![(0.0, 0.0); slots.len()];
    for index in order {
        let slot = slots[index];
        let shown = match Area::of(slot.window, screen).within(display) {
            part if part.area() > 0.0 => part,
            // Off the display altogether: its nearest corner.
            _ => Area::of(slot.window, screen),
        };
        let fronts = floats
            .iter()
            .filter(|(float, _)| !slot.front || *float > index)
            .map(|(_, area)| *area);
        let home = open_part(shown, fronts, slot.size);
        let wanted = centred(home, screen, slot.size);
        let origin = clear_of(&placed, wanted, slot.size, shown, display);
        placed.push(Area {
            x: origin.0,
            y: origin.1,
            w: slot.size.0,
            h: slot.size.1,
        });
        origins[index] = origin;
    }
    origins
}

/// `wanted`, or the nearest place to it that touches no card in `placed`: beside one of them
/// on either axis, inside the display, over `window` when such a place exists. With no room
/// anywhere, the place that covers the least of the other cards.
fn clear_of(
    placed: &[Area],
    wanted: (f32, f32),
    size: (f32, f32),
    window: Area,
    display: Area,
) -> (f32, f32) {
    let card = |(x, y): (f32, f32)| Area {
        x,
        y,
        w: size.0,
        h: size.1,
    };
    let covered = |at: (f32, f32)| -> f32 {
        placed
            .iter()
            .map(|other| card(at).overlap(*other, CARD_GAP))
            .sum()
    };
    if covered(wanted) == 0.0 {
        return wanted;
    }
    let (low_x, high_x) = span(display.w, size.0);
    let (low_y, high_y) = span(display.h, size.1);
    let mut xs = vec![wanted.0];
    let mut ys = vec![wanted.1];
    for other in placed {
        xs.push(other.x - size.0 - CARD_GAP);
        xs.push(other.right() + CARD_GAP);
        ys.push(other.y - size.1 - CARD_GAP);
        ys.push(other.bottom() + CARD_GAP);
    }
    // The best place yet, as its rank: how much it covers, its cost, its y and its x.
    let mut best: Option<[f32; 4]> = None;
    for &x in &xs {
        for &y in &ys {
            let at = (
                x.clamp(low_x, high_x).round(),
                y.clamp(low_y, high_y).round(),
            );
            let centre = (at.0 + size.0 / 2.0, at.1 + size.1 / 2.0);
            let moved = (at.0 - wanted.0).powi(2) + (at.1 - wanted.1).powi(2);
            let cost = moved + if window.holds(centre) { 0.0 } else { AWAY };
            // Least covered, then nearest, then topmost and leftmost: one answer every time.
            let rank = [covered(at), cost, at.1, at.0];
            let better = best.is_none_or(|held| {
                rank.iter()
                    .zip(&held)
                    .map(|(new, old)| new.total_cmp(old))
                    .find(|order| order.is_ne())
                    .is_some_and(std::cmp::Ordering::is_lt)
            });
            if better {
                best = Some(rank);
            }
        }
    }
    best.map_or(wanted, |[_, _, y, x]| (x, y))
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
    const CARD: (f32, f32) = (340.0, 126.0);

    /// A lone window's card.
    fn card_origin(window: Rect, screen: Rect, card: (f32, f32)) -> (f32, f32) {
        let slot = Slot {
            window,
            size: card,
            front: false,
        };
        place(screen, &[slot])[0]
    }

    fn rect(x: i32, y: i32, width: i32, height: i32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    fn tile(x: i32, y: i32, width: i32, height: i32) -> Slot {
        Slot {
            window: rect(x, y, width, height),
            size: CARD,
            front: false,
        }
    }

    fn float(x: i32, y: i32, width: i32, height: i32) -> Slot {
        Slot {
            front: true,
            ..tile(x, y, width, height)
        }
    }

    fn cards(slots: &[Slot], origins: &[(f32, f32)]) -> Vec<Area> {
        slots
            .iter()
            .zip(origins)
            .map(|(slot, origin)| Area {
                x: origin.0,
                y: origin.1,
                w: slot.size.0,
                h: slot.size.1,
            })
            .collect()
    }

    /// No two cards closer than the gap, and every card whole inside the display's margins.
    fn assert_clear_and_inside(screen: Rect, slots: &[Slot], origins: &[(f32, f32)]) {
        let cards = cards(slots, origins);
        for (index, card) in cards.iter().enumerate() {
            assert!(
                card.x >= EDGE_MARGIN
                    && card.y >= EDGE_MARGIN
                    && card.right() <= screen.width as f32 - EDGE_MARGIN
                    && card.bottom() <= screen.height as f32 - EDGE_MARGIN,
                "card {index} leaves the display: {card:?}"
            );
            for (other_index, other) in cards.iter().enumerate().skip(index + 1) {
                assert_eq!(
                    card.overlap(*other, CARD_GAP - 0.5),
                    0.0,
                    "cards {index} and {other_index} touch: {card:?} {other:?}"
                );
            }
        }
    }

    #[test]
    fn a_card_centres_on_its_window_in_display_local_points() {
        // A window at (1400, 200) 800×600 on a display whose origin is (1000, 0).
        let origin = card_origin(rect(1400, 200, 800, 600), SCREEN, (200.0, 100.0));
        assert_eq!(origin, (700.0, 450.0));
    }

    #[test]
    fn a_window_at_an_edge_keeps_its_card_whole_inside_the_display() {
        let card = (240.0, 110.0);
        // Hard against the top-left corner, narrower than its card.
        assert_eq!(
            card_origin(rect(1000, 0, 80, 40), SCREEN, card),
            (EDGE_MARGIN, EDGE_MARGIN)
        );
        // Mostly off the right and bottom edges.
        let (x, y) = card_origin(rect(2500, 950, 400, 300), SCREEN, card);
        assert_eq!(x, 1600.0 - 240.0 - EDGE_MARGIN);
        assert_eq!(y, 1000.0 - 110.0 - EDGE_MARGIN);
    }

    #[test]
    fn a_card_wider_than_the_display_sits_at_its_margin() {
        let tiny = rect(0, 0, 200, 150);
        assert_eq!(
            card_origin(rect(0, 0, 200, 150), tiny, (400.0, 300.0)),
            (EDGE_MARGIN, EDGE_MARGIN)
        );
    }

    #[test]
    fn tiles_side_by_side_keep_their_cards_centred() {
        let slots = [tile(1000, 0, 800, 1000), tile(1800, 0, 800, 1000)];
        let origins = place(SCREEN, &slots);
        assert_eq!(origins, vec![(230.0, 437.0), (1030.0, 437.0)]);
        assert_clear_and_inside(SCREEN, &slots, &origins);
    }

    #[test]
    fn a_window_partly_off_the_display_centres_its_card_on_the_part_that_shows() {
        // 600 wide, 400 of it off the left edge: the card would centre at x = -100 + 300.
        let slots = [tile(600, 200, 600, 600)];
        let (x, _) = place(SCREEN, &slots)[0];
        // The 200 points that show are narrower than the card: it rests at the margin.
        assert_eq!(x, EDGE_MARGIN);
        // A wide enough part: centred on what shows (x 0..800 of a 1000-wide window).
        let slots = [tile(800, 200, 1000, 600)];
        assert_eq!(place(SCREEN, &slots)[0].0, (800.0 - CARD.0) / 2.0);
    }

    #[test]
    fn two_windows_in_one_place_get_cards_side_by_side_not_on_each_other() {
        // Two floats stacked on the same frame: v1 drew one card over the other.
        let slots = [float(1500, 300, 700, 400), float(1500, 300, 700, 400)];
        let origins = place(SCREEN, &slots);
        assert_clear_and_inside(SCREEN, &slots, &origins);
        // The one in front (the later) keeps the centre; the one behind it moves the least
        // distance that clears it (up and down tie, and up wins), and stays over its window.
        assert_eq!(origins[1], (680.0, 437.0));
        assert_eq!(origins[0], (680.0, 437.0 - CARD.1 - CARD_GAP));
    }

    #[test]
    fn a_float_behind_another_shows_its_card_on_the_part_that_shows() {
        // The back float's right 400 points are under the front one.
        let slots = [float(1100, 300, 900, 400), float(1600, 250, 600, 500)];
        let origins = place(SCREEN, &slots);
        assert_clear_and_inside(SCREEN, &slots, &origins);
        // The front one's card is centred on it.
        assert_eq!(origins[1], (730.0, 437.0));
        // The back one's is centred on the 500 points of it left showing, not under the other.
        assert_eq!(origins[0], (100.0 + (500.0 - CARD.0) / 2.0, 437.0));
    }

    #[test]
    fn a_tile_under_a_float_shows_its_card_beside_the_float() {
        // A float over the middle of a full-display tile.
        let slots = [tile(1000, 0, 1600, 1000), float(1500, 300, 600, 400)];
        let origins = place(SCREEN, &slots);
        assert_clear_and_inside(SCREEN, &slots, &origins);
        // The float's card is centred on the float.
        assert_eq!(origins[1], (630.0, 437.0));
        // The tile's card takes the largest part the float leaves (the strips beside it tie,
        // and the left one wins), not the float's middle.
        let float_area = Area::of(slots[1].window, SCREEN);
        let tile_card = cards(&slots, &origins)[0];
        assert_eq!(tile_card.overlap(float_area, 0.0), 0.0);
        assert_eq!(origins[0], ((500.0 - CARD.0) / 2.0, 437.0));
    }

    #[test]
    fn a_small_float_over_a_small_tile_still_separates_their_cards() {
        // Nothing the float leaves of the tile holds a card: the cards separate anyway.
        let slots = [tile(1200, 200, 420, 300), float(1230, 230, 380, 240)];
        let origins = place(SCREEN, &slots);
        assert_clear_and_inside(SCREEN, &slots, &origins);
        // The float keeps its centre.
        assert_eq!(origins[1], card_origin(slots[1].window, SCREEN, CARD));
    }

    #[test]
    fn a_grid_of_narrow_tiles_never_stacks_cards() {
        // Six columns of 266: each narrower than a card.
        let slots: Vec<Slot> = (0..6)
            .map(|column| tile(1000 + column * 266, 0, 266, 1000))
            .collect();
        let origins = place(SCREEN, &slots);
        assert_clear_and_inside(SCREEN, &slots, &origins);
        // Many small windows piled in a corner.
        let piled: Vec<Slot> = (0..8)
            .map(|index| float(1010 + index * 14, 10 + index * 9, 300, 200))
            .collect();
        let origins = place(SCREEN, &piled);
        assert_clear_and_inside(SCREEN, &piled, &origins);
    }

    #[test]
    fn more_cards_than_the_display_holds_still_all_land_inside_it() {
        let small = rect(0, 0, 500, 300);
        let slots: Vec<Slot> = (0..9).map(|_| tile(0, 0, 500, 300)).collect();
        let origins = place(small, &slots);
        assert_eq!(origins.len(), 9);
        for (x, y) in origins {
            assert!(
                (EDGE_MARGIN..=500.0 - CARD.0 - EDGE_MARGIN).contains(&x),
                "{x}"
            );
            assert!(
                (EDGE_MARGIN..=300.0 - CARD.1 - EDGE_MARGIN).contains(&y),
                "{y}"
            );
        }
    }

    #[test]
    fn the_same_windows_always_give_the_same_places_in_the_order_given() {
        let slots = [
            tile(1000, 0, 800, 500),
            float(1300, 200, 500, 400),
            tile(1000, 500, 800, 500),
            tile(1800, 0, 800, 1000),
            float(1320, 220, 470, 400),
        ];
        let first = place(SCREEN, &slots);
        assert_eq!(first, place(SCREEN, &slots));
        assert_eq!(first.len(), slots.len());
        assert_clear_and_inside(SCREEN, &slots, &first);
        // A lone window's card does not depend on where it is in the list.
        let alone = place(SCREEN, &slots[3..4]);
        assert_eq!(first[3], alone[0]);
    }
}
