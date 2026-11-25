//! Terminal, where an agent has been asked to automate a whole life and has taken it literally.

use rune::{Color, El, Shape, col, el, oklch, row};

use super::{LIGHT, MONO, TINY, TOOLBAR_H, ink, lights, space, words};

/// A terminal: one command typed at the prompt, the jobs it scheduled, and a warning nobody
/// will read until a quarter past two.
pub(super) fn terminal(focused: bool) -> El {
    let mint = oklch(0.82, 0.13, 165.0);
    let amber = oklch(0.83, 0.13, 80.0);
    let slate = oklch(0.74, 0.02, 255.0).alpha(0.62);
    let chevron = || {
        el().key("prompt").size(4.0, 6.0).shape(Shape::new().stroke(
            vec![[0.5, 0.5], [3.5, 3.0], [0.5, 5.5]],
            1.3,
            mint,
        ))
    };
    let line = |key: &str, parts: &[(&str, Color)]| {
        let mut line = row().key(key).w_full().h(8.0).gap(3.5).center_y();
        for (index, (said, color)) in parts.iter().enumerate() {
            line = line.child(words(&format!("part{index}"), said, MONO, *color));
        }
        line
    };
    col()
        .child(
            row()
                .key("title-bar")
                .w_full()
                .h(TOOLBAR_H - 2.0)
                .px(8.0)
                .center_y()
                .child(lights(focused))
                .child(space("lead"))
                .child(words("title", "zsh \u{2014} opencode", TINY, ink(0.42)))
                .child(space("trail"))
                // As wide as the lights, so the title sits in the middle.
                .child(el().key("balance").w(3.0 * LIGHT + 7.0)),
        )
        .child(
            col()
                .key("lines")
                .w_full()
                .grow(1.0)
                .px(10.0)
                .pt(4.0)
                .gap(2.0)
                .child(
                    row()
                        .key("line0")
                        .w_full()
                        .h(8.0)
                        .gap(4.0)
                        .center_y()
                        .child(chevron())
                        .child(words(
                            "command",
                            "opencode run \"automate my whole life\"",
                            MONO,
                            ink(0.9),
                        )),
                )
                .child(line("line1", &[("Planning\u{2026} 3 jobs", slate)]))
                .child(line(
                    "line2",
                    &[
                        ("\u{2713} cron: water-fern", mint),
                        ("\u{2713} cron: text-mom", mint),
                    ],
                ))
                .child(line(
                    "line3",
                    &[("\u{2713} cron: standup-bass-drop (2:15 PM)", mint)],
                ))
                .child(line(
                    "line4",
                    &[("\u{26a0} warning: drywall load at 612%", amber)],
                ))
                .child(line(
                    "line5",
                    &[("Done in 4.2s. Have a great afternoon.", slate)],
                ))
                .child(
                    row()
                        .key("line6")
                        .h(8.0)
                        .gap(5.0)
                        .center_y()
                        .child(chevron())
                        .child(el().key("cursor").size(4.0, 7.0).radius(1.0).fill(ink(0.8))),
                ),
        )
}
