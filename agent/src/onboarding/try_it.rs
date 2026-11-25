//! The tour's last step: a miniature desktop to try four shortcuts on. Each move plays on the
//! stage (`stage.rs`), ticks its keys the first time it is made and leaves the desktop free to
//! play with after all four. Its keys can be pressed for real or clicked. Nothing here
//! advances by itself, and nothing here moves a real window.

use std::time::{Duration, Instant};

use rune::{
    Cursor, El, LoopSpec, Mods, PhysicalInput, PhysicalKeyCode, Spring, SpringPair, Transition, Tx,
    col, el, icon, row, text,
};
use rune_kit::Checkbox;
use spaceman::{
    app::UiLaunchAtLogin,
    shortcuts::{Chord, Modifiers},
};

use super::{
    Input,
    demo::{self, Demo, Move},
    flow::View,
    input, stage,
    view::title_block,
};
use crate::{
    Ui,
    controls::{self, METRICS, Paint},
    panes,
};

/// How long a pressed key stays down.
pub const KEY_BEAT: Duration = Duration::from_millis(140);
/// How long the completion ring shows before it widens and fades.
const CHEER_BEAT: Duration = Duration::from_millis(280);
/// A tick lands with a little bounce.
const TICK: Spring = Spring::new(0.3, 0.4);
/// The next move's mark breathes, so the eye finds what to press.
const NEXT: LoopSpec = LoopSpec::opacity(1.6, 0.0, 1.0, 0.35);
const CHIP_H: f32 = 68.0;

/// The miniature desktop, and what is pressed on it right now.
#[derive(Clone, Copy, Debug, Default)]
pub struct Play {
    pub demo: Demo,
    /// The move whose keys are down, and since when.
    down: Option<(Move, Instant)>,
    /// The modifier keys held, while the stage has the keyboard.
    held: Modifiers,
    /// When the last move was made, while its ring still shows.
    cheer: Option<Instant>,
}

impl Play {
    /// Presses `mv`'s keys: the desktop makes the move, and the first time every move has
    /// been made the ring shows.
    pub fn press(&mut self, mv: Move) {
        let now = Instant::now();
        let was_complete = self.demo.complete();
        self.demo.press(mv);
        self.down = Some((mv, now));
        if !was_complete && self.demo.complete() {
            self.cheer = Some(now);
        }
    }

    /// Lets a pressed key up and the ring go once each has had its beat. Returns whether
    /// anything changed.
    pub fn settle(&mut self) -> bool {
        let key = self.down.is_some_and(|(_, at)| at.elapsed() >= KEY_BEAT);
        let ring = self.cheer.is_some_and(|at| at.elapsed() >= CHEER_BEAT);
        if key {
            self.down = None;
        }
        if ring {
            self.cheer = None;
        }
        key || ring
    }

    /// How long until the next key or ring is due to settle.
    pub fn pending(&self) -> Option<Duration> {
        let key = self
            .down
            .map(|(_, at)| KEY_BEAT.saturating_sub(at.elapsed()));
        let ring = self.cheer.map(|at| CHEER_BEAT.saturating_sub(at.elapsed()));
        match (key, ring) {
            (Some(key), Some(ring)) => Some(key.min(ring)),
            (key, ring) => key.or(ring),
        }
    }

    /// Everything at rest, as a scene draws it.
    pub fn rest(&mut self) {
        self.down = None;
        self.cheer = None;
        self.held = Modifiers::default();
    }

    pub const fn cheering(&self) -> bool {
        self.cheer.is_some()
    }

    /// The modifier keys now held. Returns whether that changed.
    pub fn hold(&mut self, held: Modifiers) -> bool {
        let changed = self.held != held;
        self.held = held;
        changed
    }

    /// `chord`'s keys for `mv`, each with whether it is down: all of them while the move's
    /// press has its beat, and a modifier for as long as it is really held.
    pub fn caps(&self, mv: Move, chord: Chord) -> Vec<(String, bool)> {
        let pressed = self.down.is_some_and(|(down, _)| down == mv);
        let modifiers = chord.modifiers.keycaps().len();
        chord
            .keycaps()
            .into_iter()
            .enumerate()
            .map(|(index, cap)| {
                let held = index < modifiers
                    && match cap.as_str() {
                        "⌃" => self.held.control,
                        "⌥" => self.held.option,
                        "⇧" => self.held.shift,
                        "⌘" => self.held.command,
                        _ => false,
                    };
                (cap, pressed || held)
            })
            .collect()
    }
}

/// The move a physical key press asks for, by the chord it spells on the active layout.
pub fn asked(ui: &Ui, key: PhysicalKeyCode, mods: Mods) -> Option<Move> {
    demo::asked(
        &demo::chords(ui),
        panes::shortcuts::pressed_chord(key, mods)?,
    )
}

/// The stage's keyboard, while it has focus: modifiers show on the keys as they are held, and
/// a move's chord makes the move. A chord the key tap also hears is left to the tap, so one
/// press never counts twice.
fn on_input(ui: &mut Ui, event: &PhysicalInput, tx: &mut Tx) -> bool {
    match *event {
        PhysicalInput::FocusLost => hold(ui, Modifiers::default()),
        PhysicalInput::Modifiers(mods) => hold(ui, panes::shortcuts::held_modifiers(mods)),
        PhysicalInput::Key {
            key,
            mods,
            pressed: true,
            repeat,
        } => {
            let Some(mv) = asked(ui, key, mods) else {
                return false;
            };
            let tapped = ui.tour.as_ref().is_some_and(|tour| tour.tap_hears(mv));
            if !repeat && !tapped {
                super::pressed(ui, mv, tx);
            }
            // Taken either way, so the chord types nothing.
            true
        }
        PhysicalInput::Key { .. } => false,
    }
}

fn hold(ui: &mut Ui, held: Modifiers) -> bool {
    ui.tour.as_mut().is_some_and(|tour| tour.play.hold(held))
}

pub fn view(ui: &Ui, p: Paint, view: &View, play: &Play, still: bool) -> El {
    let scene = stage::Scene {
        demo: play.demo,
        cheering: play.cheering(),
        still,
    };
    let display = stage::stage(ui, p, scene)
        .focusable(true)
        .focus_ring(2.0, 3.0, p.k.c.border_focus)
        .on_physical_input(on_input);
    let mut login = Checkbox::new(view.launch_at_login).label("Launch at login");
    match ui.agent.as_ref().map(|agent| &agent.launch_at_login) {
        Some(UiLaunchAtLogin::NeedsApproval) => {
            login =
                login.description("Approve Spaceman in System Settings › General › Login Items.");
        }
        Some(UiLaunchAtLogin::Failed(error)) => login = login.description(error.clone()),
        _ => {}
    }
    col()
        .key("try-it")
        .gap(18.0)
        .center()
        .child(
            col()
                .key("heading")
                .gap(8.0)
                .center()
                .child(title_block(p, "Try it", None))
                .child(status(p, play.demo)),
        )
        .child(display)
        .child(moves(ui, p, play, still))
        .child(
            login
                .on_toggle(|ui: &mut Ui| input(ui, Input::ToggleLaunchAtLogin))
                .build(p.k)
                .key("try-login"),
        )
}

/// One line under the title: what to do, then that it is done.
fn status(p: Paint, demo: Demo) -> El {
    let line = if demo.complete() {
        row()
            .key("mastered")
            .gap(6.0)
            .center_y()
            .child(
                icon(p.k.glyph.check_small, METRICS.icon)
                    .key("check")
                    .color(p.success()),
            )
            .child(
                text("All 4 mastered — you're ready")
                    .key("label")
                    .font(p.strong())
                    .color(p.success()),
            )
    } else {
        row().key("hint").center_y().child(
            text("Press each shortcut, or click it")
                .key("label")
                .font(p.row_title())
                .color(p.muted()),
        )
    };
    row()
        .key("status")
        .h(20.0)
        .center()
        .child(line.transition(Transition::blur_replace()))
}

fn moves(ui: &Ui, p: Paint, play: &Play, still: bool) -> El {
    let next = play.demo.next();
    row().key("moves").w_full().gap(8.0).children(
        Move::ALL
            .into_iter()
            .zip(demo::chords(ui))
            .map(|(mv, chord)| chip(p, mv, chord, play, next == Some(mv), still)),
    )
}

/// A move: its keys, which sink while they are pressed, over what it does and whether it has
/// been made. Pressing the chip makes the move too.
fn chip(p: Paint, mv: Move, chord: Option<Chord>, play: &Play, next: bool, still: bool) -> El {
    let c = &p.k.c;
    let done = play.demo.done(mv);
    let keys = match chord {
        Some(chord) => controls::tray_held(p, "tray", &play.caps(mv, chord)),
        None => controls::chords(p, &[]),
    };
    let dot = METRICS.dot;
    let mark = if done {
        el().key("done")
            .size(dot, dot)
            .radius(dot / 2.0)
            .fill(p.success())
            .transition(
                Transition::scale(0.2)
                    .allow_small_scale()
                    .springs(SpringPair::new(TICK, TICK)),
            )
    } else {
        let ring = el()
            .key("todo")
            .size(dot, dot)
            .radius(dot / 2.0)
            .border(1.5, if next { p.muted() } else { c.border_base });
        if next && !still {
            ring.looping(NEXT)
        } else {
            ring
        }
    };
    let said = chord.map_or_else(String::new, |chord| chord.keycaps().join(" "));
    col()
        .key(mv.key())
        .role("button")
        .label(format!(
            "{}, {said}, {}",
            mv.title(),
            if done { "done" } else { "not done yet" }
        ))
        .grow(1.0)
        .w(0.0)
        .h(CHIP_H)
        .gap(8.0)
        .center()
        .radius(p.k.radius.lg)
        .fill(if next { c.layer_01 } else { c.bg_deep })
        .border(p.hair, if next { c.border_strong } else { c.border_weak })
        .hover(c.hover)
        .press(c.pressed)
        .cursor(Cursor::Pointer)
        .on_press_tx(move |ui: &mut Ui, tx: &mut Tx| {
            // The stage keeps the keyboard, so the next move can be pressed.
            tx.focus(stage::KEY, false);
            super::pressed(ui, mv, tx);
        })
        .child(keys)
        .child(
            row().key("caption").gap(6.0).center_y().child(mark).child(
                text(mv.title())
                    .key("title")
                    .font(p.description())
                    .color(if next { p.text() } else { p.muted() }),
            ),
        )
}
