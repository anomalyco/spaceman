//! The overlay's look: one obsidian card over one dimmed window, three neutral inks on one cool
//! hue, and no colour at all. What is switched on or under the pointer is brighter, never tinted.
//!
//! | What changes                | What moves                                   | Spring (duration / bounce) |
//! | --------------------------- | -------------------------------------------- | -------------------------- |
//! | The overlay appears         | the window's dim fades in; its card fades in | .14 / 0                    |
//! |                             | and scales from .96                          |                            |
//! | The overlay leaves          | the same path back                           | .11 / 0                    |
//! | The pointer reaches another | that window's dim and card enter where they  | .14 / 0 in, .11 / 0 out    |
//! | window                      | rest as the last window's leave where they   |                            |
//! |                             | were: nothing crosses the display            |                            |
//! | A window moves (a swap)     | its dim and its card glide to it             | .22 / 0                    |
//! | The pointer reaches a       | the chip or button lights; a hint fades in   | .07 / 0                    |
//! | control                     | beside the app's name: what a click would do | .09 / 0 in, .07 / 0 out    |
//! | A press                     | the control sinks to .96 while held          | Rune's press spring        |
//!
//! Under Reduce Motion nothing scales or travels: the card and the dim fade where they rest.

use rune::{Color, El, Shadow, Spring, SpringPair, TextStyle, oklch};

/// Every neutral sits on this hue, a cool slate, so no grey reads warm beside another.
const HUE: f32 = 255.0;

pub const ENTER: Spring = Spring::new(0.14, 0.0);
pub const EXIT: Spring = Spring::new(0.11, 0.0);
/// A card and its dim following their window to a new place.
pub const TRAVEL: Spring = Spring::new(0.22, 0.0);
/// Colours answering the pointer: quick enough to read as immediate, long enough not to pop.
pub const HOVER: Spring = Spring::new(0.07, 0.0);
/// The hint coming and going beside the app's name.
pub const HINT: SpringPair = SpringPair::new(Spring::new(0.09, 0.0), Spring::new(0.07, 0.0));
pub const REST_SCALE: f32 = 0.96;
pub const PRESS_SCALE: f32 = 0.96;

/// A card's inset, and its buttons' corner: the card's own corner is their sum, so the two
/// curves stay concentric.
pub const PAD: f32 = 10.0;
pub const CONTROL_RADIUS: f32 = 8.0;
pub const CARD_RADIUS: f32 = CONTROL_RADIUS + PAD;
const _: () = assert!(CARD_RADIUS == 18.0);

/// Primary ink: an app's name, a lit control.
pub fn text() -> Color {
    oklch(0.925, 0.005, HUE)
}

/// Secondary ink: the hint, a resting chip or button.
pub fn quiet() -> Color {
    oklch(0.785, 0.007, HUE)
}

/// Tertiary ink: the next empty workspace's plus, another display's label.
pub fn faint() -> Color {
    oklch(0.63, 0.009, HUE)
}

/// Ink on the bright chip of the window's own workspace.
pub fn ink_on_bright() -> Color {
    oklch(0.21, 0.010, HUE)
}

/// The wash over the window the card belongs to: rgb(8 10 14), drawn at [`DIM`].
pub fn obsidian() -> Color {
    oklch(0.144, 0.0095, 263.0)
}

/// How dark the wash is: enough for the card to stand clear of a white page, little enough
/// that the window still reads.
pub const DIM: f32 = 0.34;

pub fn white(alpha: f32) -> Color {
    Color::WHITE.alpha(alpha)
}

pub fn black(alpha: f32) -> Color {
    Color::BLACK.alpha(alpha)
}

pub fn name_style() -> TextStyle {
    TextStyle::new(13.0, 590.0).lh(16.0)
}

pub fn hint_style() -> TextStyle {
    TextStyle::new(11.0, 450.0).lh(14.0)
}

/// Another display's label in its track.
pub fn label_style() -> TextStyle {
    TextStyle::new(11.0, 500.0).lh(14.0)
}

/// Tabular figures, so every chip's number sits the same. The window's own workspace is
/// heavier, as its chip is bright.
pub fn chip_style(current: bool) -> TextStyle {
    TextStyle {
        features: "'tnum' 1",
        ..TextStyle::new(12.0, if current { 620.0 } else { 540.0 }).lh(16.0)
    }
}

/// The card's surface: opaque layered obsidian (with no blur behind it, any translucency would
/// show the window's text through the card's own), lit from above. A bright hairline rim, its
/// top edge the specular, dims toward the bottom; a dark hairline outside it holds the edge on
/// a white page; and four shadows, from contact to ambient, hold it on a dark one.
pub fn glass(card: El, hair: f32) -> El {
    let shadow = |y: f32, blur: f32, spread: f32, alpha: f32| Shadow {
        y,
        blur,
        spread,
        color: black(alpha),
        ..Shadow::default()
    };
    card.radius(CARD_RADIUS)
        .continuous_corners()
        .fill(oklch(0.205, 0.008, HUE))
        .gradient(oklch(0.232, 0.008, HUE), oklch(0.205, 0.008, HUE))
        .border_gradient(hair, white(0.30), white(0.11))
        .outline(hair, 0.0, black(0.60))
        .box_shadow(shadow(1.0, 2.0, 0.0, 0.24))
        .box_shadow(shadow(6.0, 14.0, -3.0, 0.26))
        .box_shadow(shadow(18.0, 38.0, -10.0, 0.34))
        .box_shadow(shadow(40.0, 80.0, -20.0, 0.30))
}

/// The recessed track a card's chips sit in: a fill and nothing else.
pub fn track() -> Color {
    white(0.065)
}

/// A chip's fill and ink: the window's own workspace is bright, the others rest on their
/// track and light under the pointer; the next empty one's plus recedes until then.
pub fn chip(current: bool, hot: bool, plus: bool) -> (Color, Color) {
    match (current, hot, plus) {
        (true, ..) => (text(), ink_on_bright()),
        (false, true, _) => (white(0.14), text()),
        (false, false, true) => (white(0.0), faint()),
        (false, false, false) => (white(0.0), quiet()),
    }
}

/// A button's face, resting, under the pointer, or switched on (brighter still under the
/// pointer): its fill, its rim and its ink.
pub fn button(on: bool, hot: bool) -> (Color, Color, Color) {
    match (on, hot) {
        (false, false) => (white(0.0), white(0.0), quiet()),
        (false, true) => (white(0.10), white(0.0), text()),
        (true, false) => (white(0.17), white(0.14), text()),
        (true, true) => (white(0.23), white(0.18), text()),
    }
}
