//! The miniature desktop's windows: a dark macOS window in each app's own shape, drawn from
//! bars and blocks with no words, wearing the app's icon in its corner. A window's contents are
//! laid out by its frame, so when the frame springs to a new place they travel with it.

use rune::{Color, El, Shadow, Shape, Transition, col, el, oklch, row, tok::motion::pair::SETTLE};

use super::{WINDOW, app_badge};
use crate::{
    Ui,
    controls::Paint,
    onboarding::demo::{Frame, Mode, Pose},
};

pub const RADIUS: f32 = 8.0;
const LIGHT: f32 = 5.0;
const TOOLBAR_H: f32 = 20.0;
const ICON: f32 = 26.0;
const ICON_INSET: f32 = 8.0;
/// How much an unfocused window gives up, so the focused one reads first: a little beside
/// it, more under a window floating over it.
const CALM: f32 = 0.2;
const COVERED: f32 = 0.34;
/// The window tucked behind one that fills the screen is a paler sheet's edge above it.
const TUCKED: f32 = 0.09;

fn ink(alpha: f32) -> Color {
    Color::WHITE.alpha(alpha)
}

fn bar(key: &str, w: f32, h: f32, color: Color) -> El {
    el().key(key).size(w, h).radius(h / 2.0).fill(color)
}

/// A line of text as wide as `part` of its column.
fn line(key: &str, part: f32, h: f32, color: Color) -> El {
    el().key(key).w_frac(part).h(h).radius(h / 2.0).fill(color)
}

/// One window. `focused` is the workspace's first window, which also wears its icon in the
/// corner away from its neighbour, clear of the Workspace HUD between them.
pub fn window(
    p: Paint,
    ui: &Ui,
    bundle: &str,
    frame: Frame,
    mode: Mode,
    focused: bool,
    still: bool,
) -> El {
    let (x, y, w, h) = super::rect(frame, mode);
    let lifted = frame.pose == Pose::Lifted;
    let behind = frame.pose == Pose::Behind;
    let lift = if lifted { 1.0 } else { 0.0 };
    let key = if still {
        // Under Reduce Motion a window fades into its new place.
        format!("w-{bundle}-{mode:?}")
    } else {
        format!("w-{bundle}")
    };
    let veil = if behind {
        Color::WHITE.alpha(TUCKED)
    } else if focused {
        Color::BLACK.alpha(0.0)
    } else if mode == Mode::Floated {
        Color::BLACK.alpha(COVERED)
    } else {
        Color::BLACK.alpha(CALM)
    };
    let ring = if lifted {
        // Off the grid, a lifted window is edged by light instead of the tiles' focus ring.
        Color::WHITE.alpha(0.2)
    } else {
        oklch(0.82, 0.08, 250.0).alpha(if focused { 0.4 } else { 0.0 })
    };
    let cast = |y: f32, blur: f32, alpha: f32| Shadow {
        y,
        blur,
        color: Color::BLACK.alpha(alpha),
        ..Shadow::default()
    };
    let mut window = el()
        .key(key)
        .absolute(x, y)
        .size(w, h)
        .radius(RADIUS)
        .clip_rounded(RADIUS)
        .fill(surface(bundle))
        // Lit from above: the rim is brightest along its top edge.
        .border_gradient(
            p.hair,
            ink(if focused {
                0.46
            } else if behind {
                0.36
            } else {
                0.2
            }),
            ink(if focused { 0.16 } else { 0.07 }),
        )
        .outline(1.0, 0.0, ring)
        // Shadow offsets never spring, so the resting and lifted casts trade strength.
        .box_shadow(cast(1.0, 2.0, 0.45))
        .box_shadow(cast(3.0, 8.0, 0.28 * (1.0 - lift)))
        .box_shadow(cast(4.0, 10.0, 0.38 * lift))
        .box_shadow(cast(14.0, 28.0, 0.46 * lift))
        .box_shadow(cast(28.0, 56.0, 0.36 * lift))
        .transition(Transition::opacity().springs(SETTLE))
        .a11y_hidden();
    if !still {
        window = window.motion(WINDOW);
    }
    window
        .child(chrome(bundle, focused, lifted))
        .child(
            app_badge(ui, bundle, ICON)
                .key("icon")
                .pin(
                    focused.then_some(ICON_INSET),
                    None,
                    (!focused).then_some(ICON_INSET),
                    Some(ICON_INSET),
                )
                .opacity(if behind { 0.0 } else { 1.0 })
                .box_shadow(cast(2.0, 6.0, 0.4)),
        )
        .child(
            el().key("veil")
                .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
                .fill(veil),
        )
}

/// The window's own colour: a neutral charcoal, a shade apart for each app. The terminal is
/// the darkest and lets a little of the desktop through.
fn surface(bundle: &str) -> Color {
    match bundle {
        "com.apple.Terminal" => oklch(0.15, 0.008, 268.0).alpha(0.86),
        "com.apple.Notes" => oklch(0.225, 0.006, 80.0),
        _ => oklch(0.215, 0.007, 265.0),
    }
}

fn chrome(bundle: &str, focused: bool, lifted: bool) -> El {
    let body = match bundle {
        "com.apple.Safari" => safari(focused, lifted),
        "com.apple.Terminal" => terminal(focused),
        "com.apple.Notes" => notes(focused),
        _ => mail(focused),
    };
    body.key("chrome").w_full().h_full()
}

fn lights(focused: bool) -> El {
    let colors = if focused {
        [
            oklch(0.70, 0.19, 25.0),
            oklch(0.83, 0.16, 85.0),
            oklch(0.74, 0.19, 145.0),
        ]
    } else {
        [ink(0.16); 3]
    };
    let mut lamps = row().key("lights").gap(3.5).center_y();
    for (index, color) in colors.into_iter().enumerate() {
        lamps = lamps.child(
            el().key(format!("light{index}"))
                .size(LIGHT, LIGHT)
                .radius(LIGHT / 2.0)
                .fill(color),
        );
    }
    lamps
}

/// A toolbar's row: the traffic lights, then `items`, over a hairline.
fn toolbar(focused: bool, items: Vec<El>) -> El {
    row()
        .key("toolbar")
        .w_full()
        .h(TOOLBAR_H)
        .px(8.0)
        .gap(6.0)
        .center_y()
        .fill(ink(0.045))
        .border_sides([0.0, 0.0, 1.0, 0.0], Color::BLACK.alpha(0.42))
        .child(lights(focused))
        .children(items)
}

fn glyph(key: &str) -> El {
    el().key(key).size(7.0, 6.0).radius(2.0).fill(ink(0.2))
}

fn space(key: &str) -> El {
    el().key(key).grow(1.0)
}

/// A browser: an address field in the toolbar, and a page with a hero, three cards and copy.
/// Lifted off the grid the window is shorter, and the copy gives its place to the icon.
fn safari(focused: bool, lifted: bool) -> El {
    let address = row()
        .key("address")
        .grow(6.0)
        .max_w(132.0)
        .h(11.0)
        .gap(3.0)
        .center()
        .radius(5.5)
        .fill(ink(0.085))
        .child(el().key("lock").size(3.0, 3.0).radius(1.0).fill(ink(0.42)))
        .child(bar("host", 34.0, 3.0, ink(0.4)));
    let hero = col()
        .key("hero")
        .w_full()
        .h(62.0)
        .padding(9.0)
        .gap(5.0)
        .radius(6.0)
        .fill(oklch(0.31, 0.09, 268.0))
        .gradient(oklch(0.38, 0.105, 258.0), oklch(0.26, 0.085, 285.0))
        .child(bar("headline", 66.0, 5.0, ink(0.94)))
        .child(bar("standfirst", 92.0, 3.5, ink(0.5)))
        .child(space("space"))
        .child(bar("action", 28.0, 9.0, ink(0.92)));
    let card = |key: &str| {
        col()
            .key(key)
            .grow(1.0)
            .w(0.0)
            .h(36.0)
            .padding(5.0)
            .gap(4.0)
            .radius(5.0)
            .fill(ink(0.06))
            .child(
                el().key("thumb")
                    .w_full()
                    .h(13.0)
                    .radius(3.0)
                    .fill(ink(0.09)),
            )
            .child(line("title", 0.8, 3.0, ink(0.34)))
            .child(line("more", 0.5, 3.0, ink(0.16)))
    };
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
        .child(
            col()
                .key("page")
                .w_full()
                .grow(1.0)
                .padding(9.0)
                .gap(7.0)
                .child(hero)
                .child(
                    row()
                        .key("cards")
                        .w_full()
                        .gap(6.0)
                        .child(card("card0"))
                        .child(card("card1"))
                        .child(card("card2")),
                )
                .child(
                    col()
                        .key("copy")
                        .w_full()
                        .gap(7.0)
                        .opacity(if lifted { 0.0 } else { 1.0 })
                        .opacity_motion(WINDOW)
                        .child(line("copy0", 0.62, 3.5, ink(0.2)))
                        .child(line("copy1", 0.5, 3.5, ink(0.13)))
                        .child(line("copy2", 0.56, 3.5, ink(0.13))),
                ),
        )
}

/// A terminal: prompts, the commands typed at them and what they printed.
fn terminal(focused: bool) -> El {
    let mint = oklch(0.82, 0.13, 165.0);
    let amber = oklch(0.83, 0.13, 80.0);
    let slate = oklch(0.68, 0.03, 255.0).alpha(0.5);
    let chevron = || {
        el().key("prompt").size(4.0, 6.0).shape(Shape::new().stroke(
            vec![[0.5, 0.5], [3.5, 3.0], [0.5, 5.5]],
            1.3,
            mint,
        ))
    };
    let out = |key: &str, parts: &[(f32, Color)]| {
        let mut line = row().key(key).h(6.0).gap(4.0).center_y();
        for (index, (w, color)) in parts.iter().enumerate() {
            line = line.child(bar(&format!("word{index}"), *w, 3.5, *color));
        }
        line
    };
    let typed = |key: &str, w: f32| {
        row()
            .key(key)
            .h(6.0)
            .gap(5.0)
            .center_y()
            .child(chevron())
            .child(bar("command", w, 3.5, ink(0.86)))
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
                .child(bar("title", 36.0, 3.5, ink(0.2)))
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
                .pt(5.0)
                .gap(6.0)
                .child(typed("line0", 58.0))
                .child(out("line1", &[(92.0, slate)]))
                .child(out("line2", &[(22.0, mint), (56.0, slate)]))
                .child(out("line3", &[(22.0, mint), (40.0, slate)]))
                .child(out("line4", &[(26.0, amber), (64.0, slate)]))
                .child(typed("line5", 36.0))
                .child(out("line6", &[(108.0, slate)]))
                .child(out("line7", &[(70.0, slate)]))
                .child(
                    row()
                        .key("line8")
                        .h(8.0)
                        .gap(5.0)
                        .center_y()
                        .child(chevron())
                        .child(el().key("cursor").size(4.5, 8.0).radius(1.0).fill(ink(0.8))),
                ),
        )
}

/// A notebook: its notes down the side, the open one's title, lines and a short checklist.
fn notes(focused: bool) -> El {
    let warm = oklch(0.84, 0.14, 88.0);
    let entry = |key: &str, picked: bool| {
        let (strong, soft) = if picked {
            (Color::BLACK.alpha(0.72), Color::BLACK.alpha(0.4))
        } else {
            (ink(0.36), ink(0.16))
        };
        col()
            .key(key)
            .w_full()
            .padding(5.0)
            .gap(3.5)
            .radius(4.0)
            .fill(if picked { warm } else { Color::TRANSPARENT })
            .child(line("name", 0.78, 3.5, strong))
            .child(line("when", 0.5, 3.0, soft))
    };
    let todo = |key: &str, part: f32, done: bool| {
        row()
            .key(key)
            .w_full()
            .gap(5.0)
            .center_y()
            .child(
                el().key("box")
                    .size(7.0, 7.0)
                    .radius(3.5)
                    .fill(if done { warm } else { Color::TRANSPARENT })
                    .border(1.0, if done { warm } else { ink(0.3) }),
            )
            .child(el().key("task").grow(1.0).child(line(
                "text",
                part,
                3.5,
                ink(if done { 0.2 } else { 0.34 }),
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
                        .child(entry("note0", true))
                        .child(entry("note1", false))
                        .child(entry("note2", false))
                        .child(entry("note3", false)),
                )
                .child(
                    col()
                        .key("note")
                        .grow(1.0)
                        .w(0.0)
                        .padding(11.0)
                        .gap(7.0)
                        .child(bar("heading", 64.0, 6.0, ink(0.92)))
                        .child(bar("date", 30.0, 3.0, ink(0.22)))
                        .child(line("line0", 0.92, 3.5, ink(0.32)))
                        .child(line("line1", 0.78, 3.5, ink(0.32)))
                        .child(line("line2", 0.56, 3.5, ink(0.32)))
                        .child(todo("todo0", 0.6, true))
                        .child(todo("todo1", 0.74, true))
                        .child(todo("todo2", 0.48, false)),
                ),
        )
}

/// A mail client: the inbox with one message picked, and that message open beside it.
fn mail(focused: bool) -> El {
    let blue = oklch(0.62, 0.17, 255.0);
    let message = |key: &str, picked: bool, unread: bool| {
        let (strong, soft) = if picked {
            (ink(0.96), ink(0.62))
        } else {
            (ink(0.4), ink(0.17))
        };
        row()
            .key(key)
            .w_full()
            .px(5.0)
            .py(5.0)
            .gap(4.0)
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
                    .gap(3.5)
                    .child(line("from", 0.6, 3.5, strong))
                    .child(line("subject", 0.9, 3.0, soft))
                    .child(line("preview", 0.7, 3.0, soft)),
            )
    };
    col()
        .child(toolbar(
            focused,
            vec![
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
                        .w(84.0)
                        .h_full()
                        .padding(4.0)
                        .gap(1.0)
                        .fill(ink(0.03))
                        .border_sides([0.0, 1.0, 0.0, 0.0], Color::BLACK.alpha(0.36))
                        .child(message("mail0", true, false))
                        .child(message("mail1", false, true))
                        .child(message("mail2", false, true))
                        .child(message("mail3", false, false)),
                )
                .child(
                    col()
                        .key("reading")
                        .grow(1.0)
                        .w(0.0)
                        .padding(10.0)
                        .gap(7.0)
                        .child(
                            row()
                                .key("sender")
                                .w_full()
                                .gap(6.0)
                                .center_y()
                                .child(
                                    el().key("face")
                                        .size(16.0, 16.0)
                                        .radius(8.0)
                                        .fill(oklch(0.62, 0.13, 300.0))
                                        .gradient(
                                            oklch(0.72, 0.12, 320.0),
                                            oklch(0.52, 0.15, 285.0),
                                        ),
                                )
                                .child(
                                    col()
                                        .key("who")
                                        .gap(3.5)
                                        .child(bar("name", 44.0, 4.0, ink(0.9)))
                                        .child(bar("to", 28.0, 3.0, ink(0.24))),
                                ),
                        )
                        .child(el().key("rule").w_full().h(1.0).fill(ink(0.07)))
                        .child(line("line0", 0.94, 3.5, ink(0.3)))
                        .child(line("line1", 0.82, 3.5, ink(0.3)))
                        .child(line("line2", 0.9, 3.5, ink(0.3)))
                        .child(line("line3", 0.4, 3.5, ink(0.3))),
                ),
        )
}
