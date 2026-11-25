//! Safari, on Spaceman's page, with the afternoon's other two tabs still open.

use rune::{Color, El, TextStyle, col, el, oklch, row};

use super::{SMALL, STRONG, TINY, WINDOW, glyph, ink, space, toolbar, words};

const HEADLINE: TextStyle = TextStyle::new(11.0, 700.0).lh(13.0).tracking(-0.2);
const EYEBROW: TextStyle = TextStyle::new(4.0, 650.0).lh(5.0).tracking(0.6);

/// A browser: the address, three tabs, and a page with a hero, the shopping it has seen today
/// and its copy. Lifted off the grid the window is shorter, and the copy gives its place to the
/// icon.
pub(super) fn safari(focused: bool, lifted: bool) -> El {
    let address = row()
        .key("address")
        .grow(6.0)
        .max_w(132.0)
        .h(11.0)
        .gap(3.0)
        .center()
        .radius(5.5)
        .fill(ink(0.085))
        .child(el().key("lock").size(3.0, 3.5).radius(1.0).fill(ink(0.42)))
        .child(words("host", "anoma.ly/spaceman", TINY, ink(0.78)));
    col()
        .child(toolbar(
            focused,
            vec![
                glyph("sidebar"),
                space("lead"),
                address,
                space("trail"),
                glyph("share"),
                glyph("tabs"),
            ],
        ))
        .child(tabs())
        .child(
            col()
                .key("page")
                .w_full()
                .grow(1.0)
                .padding(9.0)
                .gap(6.0)
                .child(hero())
                .child(words("viewed", "RECENTLY VIEWED", EYEBROW, ink(0.34)))
                .child(
                    row()
                        .key("cards")
                        .w_full()
                        .gap(6.0)
                        .child(card(
                            "card0",
                            Thumb::Speaker,
                            "90 lb Vintage Speaker",
                            "$40 \u{b7} pickup only",
                        ))
                        .child(card(
                            "card1",
                            Thumb::Anchors,
                            "4 drywall anchors",
                            "Rated 15 lbs \u{b7} $3",
                        ))
                        .child(card(
                            "card2",
                            Thumb::Clinic,
                            "Ringing in one ear",
                            "Probably fine.",
                        )),
                )
                .child(
                    col()
                        .key("copy")
                        .w_full()
                        .gap(2.0)
                        .opacity(if lifted { 0.0 } else { 1.0 })
                        .opacity_motion(WINDOW)
                        .child(words(
                            "copy0",
                            "Every Space, every display, tiled.",
                            SMALL,
                            ink(0.5),
                        ))
                        .child(words(
                            "copy1",
                            "Free for macOS 14 and later. No SIP changes.",
                            SMALL,
                            ink(0.32),
                        ))
                        .child(words(
                            "copy2",
                            "\u{201c}It even remembered where my speaker window was.\u{201d}",
                            SMALL,
                            ink(0.32),
                        )),
                ),
        )
}

/// The tab bar: this page, and the two looked up on the way.
fn tabs() -> El {
    let tab = |key: &str, label: &str, active: bool| {
        row()
            .key(key)
            .grow(1.0)
            .w(0.0)
            .h(9.0)
            .px(4.0)
            .center()
            .radius(3.0)
            .fill(if active {
                ink(0.08)
            } else {
                Color::TRANSPARENT
            })
            .child(words(
                "title",
                label,
                TINY,
                ink(if active { 0.82 } else { 0.4 }),
            ))
    };
    row()
        .key("tab-bar")
        .w_full()
        .h(13.0)
        .px(6.0)
        .gap(2.0)
        .center_y()
        .fill(ink(0.02))
        .border_sides([0.0, 0.0, 1.0, 0.0], Color::BLACK.alpha(0.36))
        .child(tab("tab0", "anoma.ly/spaceman", true))
        .child(tab("tab1", "Drywall Anchors", false))
        .child(tab("tab2", "WebMD", false))
}

fn hero() -> El {
    col()
        .key("hero")
        .w_full()
        .h(62.0)
        .padding(8.0)
        .gap(2.0)
        .radius(6.0)
        .fill(oklch(0.31, 0.09, 268.0))
        .gradient(oklch(0.38, 0.105, 258.0), oklch(0.26, 0.085, 285.0))
        .child(words(
            "eyebrow",
            "ANOMALY \u{b7} SPACEMAN",
            EYEBROW,
            ink(0.6),
        ))
        .child(words("headline", "Spaces, tiled.", HEADLINE, ink(0.96)))
        .child(words(
            "standfirst",
            "Every window back where you left it.",
            SMALL,
            ink(0.64),
        ))
        .child(space("space"))
        .child(
            row()
                .key("actions")
                .w_full()
                .gap(5.0)
                .center_y()
                .child(
                    row()
                        .key("download")
                        .h(10.0)
                        .px(5.0)
                        .center()
                        .radius(5.0)
                        .fill(ink(0.92))
                        .child(words(
                            "label",
                            "Download",
                            STRONG.lh(6.0),
                            Color::BLACK.alpha(0.82),
                        )),
                )
                .child(words(
                    "note",
                    "0 windows lost behind the notch",
                    TINY,
                    ink(0.46),
                )),
        )
}

#[derive(Clone, Copy)]
enum Thumb {
    Speaker,
    Anchors,
    Clinic,
}

/// A product's picture, drawn in a few shapes.
fn thumb(kind: Thumb) -> El {
    let plate = el().key("thumb").w_full().h(14.0).radius(3.0).center();
    match kind {
        // A cabinet with its woofer and tweeter.
        Thumb::Speaker => plate.fill(oklch(0.32, 0.03, 50.0)).child(
            col()
                .key("cabinet")
                .size(9.0, 12.0)
                .gap(1.0)
                .center()
                .radius(1.5)
                .fill(oklch(0.2, 0.02, 50.0))
                .child(
                    el().key("tweeter")
                        .size(2.5, 2.5)
                        .radius(1.25)
                        .fill(ink(0.3)),
                )
                .child(
                    el().key("woofer")
                        .size(6.0, 6.0)
                        .radius(3.0)
                        .fill(ink(0.18))
                        .border(0.5, ink(0.32)),
                ),
        ),
        // Four little plastic plugs.
        Thumb::Anchors => {
            let mut plugs = row().key("plugs").gap(2.0).center_y();
            for index in 0..4 {
                plugs = plugs.child(
                    el().key(format!("plug{index}"))
                        .size(2.0, 7.0)
                        .radius(1.0)
                        .fill(oklch(0.78, 0.06, 90.0)),
                );
            }
            plate.fill(oklch(0.3, 0.01, 250.0)).child(plugs)
        }
        // A pale cross on teal.
        Thumb::Clinic => plate.fill(oklch(0.42, 0.07, 195.0)).child(
            el().key("cross")
                .size(7.0, 7.0)
                .center()
                .child(el().key("bar").size(7.0, 2.0).radius(0.5).fill(ink(0.85)))
                .child(
                    el().key("post")
                        .absolute(2.5, 0.0)
                        .size(2.0, 7.0)
                        .radius(0.5)
                        .fill(ink(0.85)),
                ),
        ),
    }
}

fn card(key: &str, kind: Thumb, title: &str, meta: &str) -> El {
    col()
        .key(key)
        .grow(1.0)
        .w(0.0)
        .padding(4.0)
        .gap(2.0)
        .radius(5.0)
        .fill(ink(0.06))
        .child(thumb(kind))
        .child(words("title", title, TINY, ink(0.72)))
        .child(words("meta", meta, TINY, ink(0.34)))
}
