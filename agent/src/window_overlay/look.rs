//! The overlay's look: obsidian cards over whatever the windows show, three neutral inks on one
//! hue, and one amber accent for where the pointer is and what is switched on.
//!
//! | What changes                | What moves                                   | Spring (duration / bounce) |
//! | --------------------------- | -------------------------------------------- | -------------------------- |
//! | The overlay appears         | cards fade in and scale from .96; outlines   | .14 / 0                    |
//! |                             | fade in where they rest                      |                            |
//! | The overlay leaves          | the same path back                           | .11 / 0                    |
//! | A window moves (a swap)     | its outline and its card glide to it         | .22 / 0                    |
//! | The pointer reaches a       | the outline, chip or button lights; the      | .07 / 0                    |
//! | window or a control         | card's status line cross-fades to what a     | .09 / 0 in, .07 / 0 out    |
//! |                             | click would do                               |                            |
//! | A press                     | the control sinks to .96 while held          | Rune's press spring        |
//!
//! Under Reduce Motion nothing scales or travels: cards and outlines fade where they rest.

use rune::{Color, El, Shadow, Spring, SpringPair, TextStyle, oklch, oklcha};

/// Every neutral sits on this hue, a cool slate, so no grey reads warm beside another.
const HUE: f32 = 262.0;
const AMBER_HUE: f32 = 84.0;

pub const ENTER: Spring = Spring::new(0.14, 0.0);
pub const EXIT: Spring = Spring::new(0.11, 0.0);
/// A card and its outline following their window to a new place.
pub const TRAVEL: Spring = Spring::new(0.22, 0.0);
/// Colours answering the pointer: quick enough to read as immediate, long enough not to pop.
pub const HOVER: Spring = Spring::new(0.07, 0.0);
/// The status line's swap between what is and what a click would do.
pub const STATUS: SpringPair = SpringPair::new(Spring::new(0.09, 0.0), Spring::new(0.07, 0.0));
pub const REST_SCALE: f32 = 0.96;
pub const PRESS_SCALE: f32 = 0.96;

/// A card's inset, and its controls' corner: the card's own corner is their sum, so the two
/// curves stay concentric.
pub const PAD: f32 = 10.0;
pub const CONTROL_RADIUS: f32 = 8.0;
pub const CARD_RADIUS: f32 = CONTROL_RADIUS + PAD;
const _: () = assert!(CARD_RADIUS == 18.0);

/// Primary ink: an app's name, a lit control.
pub fn text() -> Color {
    oklch(0.93, 0.006, HUE)
}

/// Secondary ink: the status line, a resting button's label.
pub fn quiet() -> Color {
    oklch(0.78, 0.008, HUE)
}

/// Tertiary ink: an empty workspace's number.
pub fn faint() -> Color {
    oklch(0.62, 0.010, HUE)
}

/// Ink on the bright chip of the window's own workspace.
pub fn ink_on_bright() -> Color {
    oklch(0.20, 0.012, HUE)
}

/// The accent: the window under the pointer, the control about to be pressed, a mode that is on.
pub fn accent() -> Color {
    oklch(0.85, 0.15, AMBER_HUE)
}

pub fn accent_a(alpha: f32) -> Color {
    oklcha(0.85, 0.15, AMBER_HUE, alpha)
}

pub fn white(alpha: f32) -> Color {
    Color::WHITE.alpha(alpha)
}

pub fn black(alpha: f32) -> Color {
    Color::BLACK.alpha(alpha)
}

pub fn name_style() -> TextStyle {
    TextStyle::new(13.0, 590.0).lh(16.0)
}

pub fn status_style() -> TextStyle {
    TextStyle::new(11.0, 480.0).lh(14.0)
}

pub fn label_style() -> TextStyle {
    TextStyle::new(12.0, 540.0).lh(16.0)
}

/// Tabular figures, so every chip's number sits the same.
pub fn chip_style() -> TextStyle {
    TextStyle {
        features: "'tnum' 1",
        ..TextStyle::new(12.0, 600.0).lh(16.0)
    }
}

/// The card's surface: opaque obsidian (with no blur behind it, any translucency would show the
/// window's text through the card's own), lit from above, with a bright hairline rim that dims toward the
/// bottom, a dark hairline outside it for light windows, and a contact, key and ambient shadow.
pub fn glass(card: El, hair: f32) -> El {
    card.radius(CARD_RADIUS)
        .continuous_corners()
        .fill(oklch(0.225, 0.012, HUE))
        .gradient(white(0.045), black(0.10))
        .border_gradient(hair, white(0.22), white(0.07))
        .outline(hair, 0.0, black(0.55))
        .box_shadow(Shadow {
            y: 1.0,
            blur: 2.0,
            color: black(0.40),
            ..Shadow::default()
        })
        .box_shadow(Shadow {
            y: 8.0,
            blur: 20.0,
            spread: -4.0,
            color: black(0.38),
            ..Shadow::default()
        })
        .box_shadow(Shadow {
            y: 28.0,
            blur: 64.0,
            spread: -12.0,
            color: black(0.50),
            ..Shadow::default()
        })
}

/// A control's face: resting, under the pointer, or switched on (amber, brighter under the
/// pointer). Returns its fill, its rim and its ink.
pub fn control(on: bool, hot: bool) -> (Color, Color, Color) {
    match (on, hot) {
        (false, false) => (white(0.06), white(0.07), quiet()),
        (false, true) => (white(0.13), white(0.16), text()),
        (true, false) => (accent_a(0.15), accent_a(0.42), accent()),
        (true, true) => (accent_a(0.24), accent_a(0.60), accent()),
    }
}
