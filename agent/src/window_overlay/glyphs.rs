//! The card's glyphs, one family: 14 points square, 1.4-point round strokes, drawn as shapes so
//! each takes its control's ink. Strokes take an opaque colour, so no joint shows twice.

use rune::{Color, El, LineCap, Shape, el};
use spaceman::Direction;

pub const SIZE: f32 = 14.0;
const LINE: f32 = 1.4;

/// The points of an arc around `c`, from `from` to `to` degrees, clockwise from three o'clock.
fn arc(c: [f32; 2], r: f32, from: f32, to: f32) -> Vec<[f32; 2]> {
    (0..=4)
        .map(|i| {
            let a = (from + (to - from) * i as f32 / 4.0).to_radians();
            [c[0] + r * a.cos(), c[1] + r * a.sin()]
        })
        .collect()
}

/// A rounded rectangle's outline, clockwise from its top-left corner.
fn rounded(x: f32, y: f32, w: f32, h: f32, r: f32) -> Vec<[f32; 2]> {
    let mut points = arc([x + r, y + r], r, 180.0, 270.0);
    points.extend(arc([x + w - r, y + r], r, 270.0, 360.0));
    points.extend(arc([x + w - r, y + h - r], r, 0.0, 90.0));
    points.extend(arc([x + r, y + h - r], r, 90.0, 180.0));
    points
}

fn glyph(key: &'static str, shape: Shape) -> El {
    el().key(key).size(SIZE, SIZE).shape(shape).a11y_hidden()
}

/// Float: a window lifted over another. `under` is the ink of the one beneath.
pub fn float(ink: Color, under: Color) -> El {
    let back = rounded(1.3, 4.6, 7.8, 7.8, 1.8);
    let front = rounded(4.9, 1.3, 7.8, 7.8, 1.8);
    let shape = Shape::new()
        .stroke_with(back, true, LINE, LineCap::Round, under)
        .fill(front.clone(), ink.alpha(0.18))
        .stroke_with(front, true, LINE, LineCap::Round, ink);
    glyph("float", shape)
}

/// Pin: a pushpin, filled.
pub fn pin(ink: Color) -> El {
    // Head, collar and needle, as one outline.
    let outline = vec![
        [4.9, 1.3],
        [9.1, 1.3],
        [8.5, 5.2],
        [10.9, 7.3],
        [10.9, 8.4],
        [7.6, 8.4],
        [7.6, 12.0],
        [7.0, 13.1],
        [6.4, 12.0],
        [6.4, 8.4],
        [3.1, 8.4],
        [3.1, 7.3],
        [5.5, 5.2],
    ];
    glyph("pin", Shape::new().fill(outline, ink))
}

/// Fullscreen Layout: four corners, pushed out.
pub fn fullscreen(ink: Color) -> El {
    let (a, b, c, d) = (1.8, 5.2, 8.8, 12.2);
    let shape = Shape::new()
        .stroke(vec![[a, b], [a, a], [b, a]], LINE, ink)
        .stroke(vec![[c, a], [d, a], [d, b]], LINE, ink)
        .stroke(vec![[a, c], [a, d], [b, d]], LINE, ink)
        .stroke(vec![[c, d], [d, d], [d, c]], LINE, ink);
    glyph("fullscreen", shape)
}

/// Plus: the next empty workspace.
pub fn plus(ink: Color) -> El {
    let shape = Shape::new()
        .line([7.0, 3.2], [7.0, 10.8], LINE, LineCap::Round, ink)
        .line([3.2, 7.0], [10.8, 7.0], LINE, LineCap::Round, ink);
    glyph("plus", shape)
}

/// An arrow toward `side`: the neighbour a swap trades places with.
pub fn arrow(side: Direction, ink: Color) -> El {
    // Drawn pointing left, then turned.
    let turn = |[x, y]: [f32; 2]| match side {
        Direction::Left => [x, y],
        Direction::Right => [SIZE - x, y],
        Direction::Up => [y, x],
        Direction::Down => [y, SIZE - x],
    };
    let shaft = [[11.4, 7.0], [2.8, 7.0]].map(turn);
    let head = [[6.3, 3.4], [2.7, 7.0], [6.3, 10.6]].map(turn);
    let shape = Shape::new()
        .line(shaft[0], shaft[1], LINE, LineCap::Round, ink)
        .stroke(head.to_vec(), LINE, ink);
    glyph("arrow", shape)
}
