//! The Try it step's prompt: one lesson at a time under the miniature desktop. It reads as a
//! shortcut's row in Settings does, what it does on the left and its keys on the right, with
//! the step's progress under the title. The keys sink while they are pressed; a learned lesson
//! gets a tick, then leaves upward as the next rises in; after the last, the keys' place is
//! taken by a seal that draws its own check.
//!
//! | What changes         | What moves                                              | Spring |
//! | -------------------- | ------------------------------------------------------- | ------ |
//! | A lesson is learned  | its tick and its progress mark pop in                   | .28 / .42 |
//! | The next lesson      | the old one leaves upward, the new one rises in         | .30 / 0, .22 / 0 |
//! | The last is learned  | the seal pops in, its check draws, a ring widens away   | .42 / .38, .34 / 0, .7 s |

use rune::{
    Beat, Easing, El, MotionIntent, Sequence, Shape, Spring, SpringPair, Transition, col, el, icon,
    row, text,
};
use spaceman::shortcuts::Chord;

use super::{
    demo::{self, LESSONS},
    stage,
    try_it::{Card, Play},
};
use crate::{
    Ui,
    controls::{self, Paint},
};

/// From the card's edge to its title and to its keys.
pub const INSET: f32 = 16.0;
const CARD_H: f32 = 56.0;
const MARK_W: f32 = 12.0;
const MARK_H: f32 = 4.0;
const SEAL: f32 = 28.0;

/// A tick lands with a little bounce.
const TICK: Spring = Spring::new(0.28, 0.42);
const RISE: Spring = Spring::new(0.3, 0.0);
const LEAVE: Spring = Spring::new(0.22, 0.0);
const SEAL_IN: Spring = Spring::new(0.42, 0.38);
const CHECK_DRAW: Spring = Spring::new(0.34, 0.0);
/// The ring the seal sends out as it lands: it widens and fades once.
const RIPPLE: [Beat; 3] = [
    Beat::new(0.0).opacity(0.0).scale(1.0),
    Beat::new(0.12).opacity(0.55).scale(1.15),
    Beat::new(0.82)
        .opacity(0.0)
        .scale(2.1)
        .easing(Easing::EASE_OUT),
];

/// The prompt under the desktop, as wide as the display it belongs to.
pub fn card(ui: &Ui, p: Paint, play: &Play, still: bool) -> El {
    let c = &p.k.c;
    let showing = play.card();
    let chords = demo::chords(ui);
    let (ask, keys, said) = match showing {
        Card::Lesson { index, learned } => {
            let lesson = LESSONS[index];
            let chord = chords[lesson.mv.index()];
            (
                title(p, index, lesson.title, learned),
                keys(p, play, index, lesson.mv, chord),
                format!(
                    "{}, {}, step {} of {}",
                    lesson.title,
                    chord.map_or_else(String::new, |chord| chord.keycaps().join(" ")),
                    index + 1,
                    LESSONS.len()
                ),
            )
        }
        Card::Ready => (
            title(p, LESSONS.len(), "You\u{2019}re ready", false),
            seal(p, still),
            "You\u{2019}re ready".to_owned(),
        ),
    };
    row()
        .key("prompt")
        .role("group")
        .label(said)
        .w(stage::DISPLAY_W)
        .h(CARD_H)
        .px(INSET)
        .gap(12.0)
        .center_y()
        .radius(p.k.radius.xl)
        .fill(c.layer_01)
        .border(p.hair, c.border_weak)
        .child(
            col()
                .key("ask")
                .grow(1.0)
                .w(0.0)
                .gap(7.0)
                .child(el().key("title-slot").w_full().h(20.0).child(ask))
                .child(progress(p, play, showing)),
        )
        .child(
            row()
                .key("keys-slot")
                .h_full()
                .center_y()
                .justify_end()
                .child(keys),
        )
}

/// A lesson leaves upward and the next rises into its place.
fn turn() -> Transition {
    let enter = Transition::offset(0.0, 8.0).combined(Transition::opacity());
    let exit = Transition::offset(0.0, -8.0).combined(Transition::opacity());
    Transition::asymmetric(enter, exit).springs(SpringPair::new(RISE, LEAVE))
}

fn pop() -> Transition {
    Transition::scale(0.2)
        .allow_small_scale()
        .springs(SpringPair::new(TICK, LEAVE))
}

/// What the lesson does, and its tick once it is learned.
fn title(p: Paint, index: usize, label: &str, learned: bool) -> El {
    let tick = learned.then(|| {
        icon(p.k.glyph.check_small, 16.0)
            .key("tick")
            .color(p.success())
            .motion_intent(MotionIntent::OVERSHOOT)
            .transition(pop())
    });
    row()
        .key(format!("lesson{index}"))
        .absolute(0.0, 0.0)
        .h(20.0)
        .gap(6.0)
        .center_y()
        .transition(turn())
        .child(
            text(label.to_owned())
                .key("label")
                .font(p.strong())
                .color(p.text()),
        )
        .children(tick)
}

/// The lesson's keys in their tray, each down while it is pressed or held.
fn keys(p: Paint, play: &Play, index: usize, mv: demo::Move, chord: Option<Chord>) -> El {
    let tray = match chord {
        Some(chord) => controls::tray_held(p, "tray", &play.caps(mv, chord)),
        None => controls::chords(p, &[]),
    };
    // Keyed by the chord too, so another base modifier brings its own keys in.
    let spelled = chord.map_or_else(String::new, |chord| chord.keycaps().concat());
    row()
        .key(format!("keys{index}-{spelled}"))
        .center_y()
        .transition(turn())
        .child(tray)
}

/// One mark for each lesson: filled as it is learned, a touch brighter for the one asked for.
fn progress(p: Paint, play: &Play, showing: Card) -> El {
    let c = &p.k.c;
    let learned = play.demo.learned();
    let asking = match showing {
        Card::Lesson {
            index,
            learned: false,
        } => Some(index),
        _ => None,
    };
    let mut marks = row().key("progress").h(MARK_H).gap(3.0).a11y_hidden();
    for index in 0..LESSONS.len() {
        let fill = (index < learned).then(|| {
            el().key("done")
                .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
                .radius(MARK_H / 2.0)
                .fill(p.success())
                .motion_intent(MotionIntent::OVERSHOOT)
                .transition(pop())
        });
        marks = marks.child(
            el().key(format!("mark{index}"))
                .size(MARK_W, MARK_H)
                .radius(MARK_H / 2.0)
                .fill(if asking == Some(index) {
                    c.border_strong
                } else {
                    c.border_base
                })
                .children(fill),
        );
    }
    marks
}

/// Every lesson learned: a green seal pops in where the keys were, draws its check and sends
/// one ring out.
fn seal(p: Paint, still: bool) -> El {
    let green = p.success();
    let check = Shape::new().stroke(
        vec![[8.5, 14.5], [12.5, 18.5], [19.5, 10.0]],
        2.2,
        p.k.c.bg_deep,
    );
    let mut ripple = el()
        .key("ripple")
        .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
        .radius(SEAL / 2.0)
        .border(1.5, green)
        .opacity(0.0);
    if !still {
        ripple = ripple.sequence(Sequence::new(&RIPPLE).delay(0.1));
    }
    el().key("seal")
        .size(SEAL, SEAL)
        .transition(turn())
        .a11y_hidden()
        .child(ripple)
        .child(
            el().key("disc")
                .size(SEAL, SEAL)
                .radius(SEAL / 2.0)
                .fill(green)
                .motion_intent(MotionIntent::OVERSHOOT)
                .transition(
                    Transition::scale(0.3)
                        .allow_small_scale()
                        .springs(SpringPair::new(SEAL_IN, LEAVE)),
                )
                .child(
                    el().key("check")
                        .size(SEAL, SEAL)
                        .shape(check)
                        .enter_delay(0.14)
                        .transition(Transition::draw().springs(SpringPair::new(CHECK_DRAW, LEAVE))),
                ),
        )
}
