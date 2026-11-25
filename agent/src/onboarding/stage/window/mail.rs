//! Mail, where the building has noticed.

use rune::{Color, El, TextStyle, col, el, oklch, row};

use super::{SMALL, STRONG, TINY, glyph, ink, prose, space, toolbar, words};

const SUBJECT: TextStyle = TextStyle::new(7.0, 680.0).lh(9.0);

/// A mail client: the inbox with one message picked, and that message open beside it.
pub(super) fn mail(focused: bool) -> El {
    let blue = oklch(0.62, 0.17, 255.0);
    let message = |key: &str, from: &str, subject: &str, picked: bool, unread: bool| {
        let (strong, soft) = if picked {
            (ink(0.96), ink(0.72))
        } else {
            (ink(0.62), ink(0.34))
        };
        row()
            .key(key)
            .w_full()
            .px(4.0)
            .py(4.0)
            .gap(3.0)
            .radius(4.0)
            .fill(if picked { blue } else { Color::TRANSPARENT })
            .child(
                el().key("unread")
                    .size(4.0, 4.0)
                    .radius(2.0)
                    .fill(if unread { blue } else { Color::TRANSPARENT }),
            )
            .child(
                col()
                    .key("text")
                    .grow(1.0)
                    .w(0.0)
                    .gap(1.0)
                    .child(words("from", from, TINY.lh(6.0), strong))
                    .child(words("subject", subject, TINY, soft)),
            )
    };
    col()
        .child(toolbar(
            focused,
            vec![
                words("mailbox", "Inbox (47)", STRONG, ink(0.72)),
                space("lead"),
                glyph("reply"),
                glyph("archive"),
                glyph("compose"),
            ],
        ))
        .child(
            row()
                .key("panes")
                .w_full()
                .grow(1.0)
                .child(
                    col()
                        .key("inbox")
                        .w(92.0)
                        .h_full()
                        .padding(4.0)
                        .gap(1.0)
                        .fill(ink(0.03))
                        .border_sides([0.0, 1.0, 0.0, 0.0], Color::BLACK.alpha(0.36))
                        .child(message(
                            "mail0",
                            "Building Mgmt",
                            "Re: Apt 4B bass",
                            true,
                            false,
                        ))
                        .child(message(
                            "mail1",
                            "Mom",
                            "Home for Thanksgiving?",
                            false,
                            true,
                        ))
                        .child(message("mail2", "Board", "Vote: 7-0", false, true))
                        .child(message(
                            "mail3",
                            "Anomaly",
                            "Welcome to Spaceman",
                            false,
                            false,
                        )),
                )
                .child(
                    col()
                        .key("reading")
                        .grow(1.0)
                        .w(0.0)
                        .padding(10.0)
                        .gap(5.0)
                        .child(
                            row()
                                .key("sender")
                                .w_full()
                                .gap(5.0)
                                .center_y()
                                .child(
                                    el().key("face")
                                        .size(14.0, 14.0)
                                        .radius(7.0)
                                        .fill(oklch(0.62, 0.13, 300.0))
                                        .gradient(
                                            oklch(0.72, 0.12, 320.0),
                                            oklch(0.52, 0.15, 285.0),
                                        ),
                                )
                                .child(
                                    col()
                                        .key("who")
                                        .grow(1.0)
                                        .w(0.0)
                                        .child(words("name", "Building Mgmt", STRONG, ink(0.9)))
                                        .child(words("to", "To: Greg, Apt 4B", TINY, ink(0.36))),
                                ),
                        )
                        .child(words("subject", "Re: Apt 4B bass", SUBJECT, ink(0.92)))
                        .child(el().key("rule").w_full().h(1.0).fill(ink(0.07)))
                        .child(prose(
                            "body0",
                            "Hi Greg, the ceiling plaster in 3B is vibrating again.",
                            SMALL,
                            ink(0.62),
                        ))
                        .child(prose(
                            "body1",
                            "Please lower the volume before 2:15.",
                            SMALL,
                            ink(0.62),
                        )),
                ),
        )
}
