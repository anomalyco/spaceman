//! Notes, on this quarter's priorities, two of them already done.

use rune::{Color, El, TextStyle, col, el, oklch, row};

use super::{SMALL, TINY, glyph, ink, space, toolbar, words};

const HEADING: TextStyle = TextStyle::new(9.0, 700.0).lh(11.0);

/// A notebook: its folders down the side, then the open note's title, date and checklist.
pub(super) fn notes(focused: bool) -> El {
    let warm = oklch(0.84, 0.14, 88.0);
    let folder = |key: &str, name: &str, count: &str, picked: bool| {
        let (strong, soft) = if picked {
            (Color::BLACK.alpha(0.78), Color::BLACK.alpha(0.5))
        } else {
            (ink(0.5), ink(0.26))
        };
        row()
            .key(key)
            .w_full()
            .px(5.0)
            .py(3.0)
            .gap(3.0)
            .center_y()
            .radius(4.0)
            .fill(if picked { warm } else { Color::TRANSPARENT })
            .child(
                el().key("name")
                    .grow(1.0)
                    .w(0.0)
                    .child(words("label", name, TINY, strong)),
            )
            .child(words("count", count, TINY, soft))
    };
    let todo = |key: &str, task: &str, done: bool| {
        row()
            .key(key)
            .w_full()
            .gap(4.0)
            .center_y()
            .child(
                el().key("box")
                    .size(6.0, 6.0)
                    .radius(3.0)
                    .fill(if done { warm } else { Color::TRANSPARENT })
                    .border(1.0, if done { warm } else { ink(0.32) }),
            )
            .child(el().key("task").grow(1.0).w(0.0).child(words(
                "text",
                task,
                SMALL,
                ink(if done { 0.36 } else { 0.72 }),
            )))
    };
    col()
        .child(toolbar(
            focused,
            vec![
                space("lead"),
                glyph("format"),
                glyph("list"),
                glyph("share"),
            ],
        ))
        .child(
            row()
                .key("panes")
                .w_full()
                .grow(1.0)
                .child(
                    col()
                        .key("list")
                        .w(62.0)
                        .h_full()
                        .padding(5.0)
                        .gap(2.0)
                        .fill(ink(0.03))
                        .border_sides([0.0, 1.0, 0.0, 0.0], Color::BLACK.alpha(0.36))
                        .child(folder("folder0", "Notes", "12", false))
                        .child(folder("folder1", "Anomaly", "4", false))
                        .child(folder("folder2", "Totally Fine", "1", true)),
                )
                .child(
                    col()
                        .key("note")
                        .grow(1.0)
                        .w(0.0)
                        .padding(10.0)
                        .gap(5.0)
                        .child(words("heading", "Q4 Priorities", HEADING, ink(0.94)))
                        .child(words("date", "Today at 1:58 PM", TINY, ink(0.3)))
                        .child(todo("todo0", "Ship Spaceman", true))
                        .child(todo("todo1", "Mount 90 lb speaker above chair", true))
                        .child(todo("todo2", "Buy better anchors (later)", false))
                        .child(todo("todo3", "Call Mom", false)),
                ),
        )
}
