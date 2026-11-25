//! The Scrolling Strip's picture: the strip running under the display. A sliver past the left
//! edge, two columns in view (the focused one marked), and the columns waiting to the right
//! fading out. Illustration geometry is its own; its corners and type are the system's.

use rune::{Color, El, col, el, row, text};

use crate::controls::Paint;

const COLUMN_W: f32 = 116.0;
/// Low enough that the whole sheet fits Settings' smallest window without scrolling.
const COLUMN_H: f32 = 40.0;
const SLIVER_W: f32 = 12.0;
/// The display's bezel, and the columns' inset inside it: the display's corner is the card
/// radius, so a column 4 in takes the small one (8 - 4).
const BEZEL: f32 = 1.5;
const INSET: f32 = 4.0;
/// How far each end of the strip dissolves.
const FADE: f32 = 32.0;

pub(super) fn picture(p: Paint) -> El {
    let c = &p.k.c;
    let radius = p.k.radius.lg;
    col()
        .key("strip-picture")
        .w_full()
        .py(8.0)
        .radius(radius)
        .clip_rounded(radius)
        .fill(c.bg_deep)
        .border(p.hair, c.border_weak)
        .a11y_hidden()
        .child(ribbon(p))
}

fn ribbon(p: Paint) -> El {
    let c = &p.k.c;
    let column = |key: &str, label: &str, fill: Color, ink: Color| {
        col()
            .key(key)
            .w(COLUMN_W)
            .h(COLUMN_H)
            .radius(p.k.radius.sm)
            .fill(fill)
            .border(p.hair, c.border_weak)
            .center()
            .child(text(label).key("label").font(p.tag()).color(ink))
    };
    let display = col()
        .key("display")
        .gap(6.0)
        .items_center()
        .child(
            row()
                .key("screen")
                .padding(INSET)
                .gap(INSET)
                .radius(p.k.radius.lg)
                .border(BEZEL, c.border_focus)
                .child(column("focused", "Focused", c.bg_accent, c.text_contrast))
                .child(column("next", "In view", c.layer_03, p.text())),
        )
        .child(
            text("Your display")
                .key("caption")
                .font(p.meta())
                .color(p.muted()),
        );
    // Columns outside the display sit on the same line as the ones inside its bezel.
    let outside = INSET + BEZEL;
    let waiting = |key: &str, opacity: f32| {
        column(key, "", c.layer_02, p.faint())
            .mt(outside)
            .opacity(opacity)
    };
    row()
        .key("ribbon")
        .w_full()
        .gap(INSET)
        .items_start()
        // Only the strip's two ends fade; the display's caption underneath stays crisp.
        .clip_fade(FADE, 0.0, FADE, 0.0)
        .child(
            el().key("sliver")
                .w(SLIVER_W + FADE)
                .h(COLUMN_H)
                .mt(outside)
                .radius(p.k.radius.sm)
                .fill(c.layer_03)
                .opacity(0.7),
        )
        .child(display)
        .child(waiting("later", 0.6))
        .child(waiting("last", 0.35))
        .child(waiting("beyond", 0.2))
}
