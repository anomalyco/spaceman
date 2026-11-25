//! The tour's last step: three real shortcuts on the real workspaces, each ticked when the key
//! tap hears it, with Launch at login under them. Nothing here advances by itself.

use rune::{El, col, el, icon, text};
use rune_kit::Checkbox;
use spaceman::{
    WorkspaceId,
    app::UiLaunchAtLogin,
    shortcuts::{Chord, Key, Modifiers},
};

use super::{Input, flow::View, input, view::title_block};
use crate::{
    Ui,
    controls::{self, METRICS, Paint},
    panes,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Lesson {
    /// Switch to workspace 2.
    There,
    /// And back to workspace 1.
    Back,
    /// Toggle the focused window's floating.
    Float,
}

impl Lesson {
    pub const ALL: [Self; 3] = [Self::There, Self::Back, Self::Float];

    const fn index(self) -> usize {
        match self {
            Self::There => 0,
            Self::Back => 1,
            Self::Float => 2,
        }
    }

    const fn key(self) -> &'static str {
        match self {
            Self::There => "there",
            Self::Back => "back",
            Self::Float => "float",
        }
    }
}

/// Which lessons have been done.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Lessons {
    done: [bool; 3],
}

impl Lessons {
    pub const fn done(self, lesson: Lesson) -> bool {
        self.done[lesson.index()]
    }

    pub fn complete(self) -> bool {
        self.done.iter().all(|done| *done)
    }

    /// Hears `lesson`'s shortcut. Coming back counts only after going there. Returns whether
    /// it added a tick.
    pub fn press(&mut self, lesson: Lesson) -> bool {
        if self.done(lesson) || (lesson == Lesson::Back && !self.done(Lesson::There)) {
            return false;
        }
        self.done[lesson.index()] = true;
        true
    }
}

/// The chord each lesson asks for, from the shortcuts in force.
pub fn chords(ui: &Ui) -> [Option<Chord>; 3] {
    let shortcuts = ui.doc.shortcuts();
    [
        panes::shortcuts::switch_chord(&shortcuts, WorkspaceId::TWO),
        panes::shortcuts::switch_chord(&shortcuts, WorkspaceId::ONE),
        panes::shortcuts::float_chord(&shortcuts, &ui.doc.named_workspaces()),
    ]
}

// Carbon's modifier bits, as the key tap reports them.
const COMMAND: u32 = 1 << 8;
const SHIFT: u32 = 1 << 9;
const OPTION: u32 = 1 << 11;
const CONTROL: u32 = 1 << 12;
/// The number row's virtual key codes, 1 through 0.
const NUMBER_ROW: [u32; 10] = [18, 19, 20, 21, 23, 22, 26, 28, 25, 29];

fn carbon(modifiers: Modifiers) -> u32 {
    [
        (modifiers.command, COMMAND),
        (modifiers.shift, SHIFT),
        (modifiers.option, OPTION),
        (modifiers.control, CONTROL),
    ]
    .into_iter()
    .filter(|(held, _)| *held)
    .fold(0, |bits, (_, bit)| bits | bit)
}

fn key_code(key: Key) -> Option<u32> {
    match key {
        Key::Number(number) => NUMBER_ROW.get(usize::from(number).checked_sub(1)?).copied(),
        Key::Code(code) => Some(u32::from(code)),
        // A character's key depends on the keyboard layout.
        Key::Character(_) => None,
    }
}

/// Whether the key tap can hear `chord`: it reports only presses with Option held, by key code.
pub fn hearable(chord: Chord) -> bool {
    chord.modifiers.option && key_code(chord.key).is_some()
}

/// The lesson a key press completes, if it is one of `chords`.
pub fn heard(chords: &[Option<Chord>; 3], code: u32, modifiers: u32) -> Option<Lesson> {
    Lesson::ALL
        .into_iter()
        .zip(chords)
        .find_map(|(lesson, chord)| {
            let chord = (*chord)?;
            (carbon(chord.modifiers) == modifiers && key_code(chord.key) == Some(code))
                .then_some(lesson)
        })
}

pub fn view(ui: &Ui, p: Paint, view: &View, lessons: Lessons) -> El {
    let rows = Lesson::ALL
        .into_iter()
        .zip(chords(ui))
        .map(|(lesson, chord)| lesson_row(p, lesson, chord, lessons.done(lesson)))
        .collect();
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
        .gap(METRICS.section_gap)
        .center()
        .child(title_block(p, "Try it", None))
        .child(controls::card(p, "try-card", rows).w_full())
        .child(
            login
                .on_toggle(|ui: &mut Ui| input(ui, Input::ToggleLaunchAtLogin))
                .build(p.k)
                .key("try-login"),
        )
}

fn lesson_row(p: Paint, lesson: Lesson, chord: Option<Chord>, done: bool) -> El {
    let mark = if done {
        icon(p.k.glyph.check_small, METRICS.icon)
            .key("done")
            .color(p.success())
    } else if chord.is_some_and(hearable) {
        el().key("waiting")
            .size(METRICS.icon, METRICS.icon)
            .center()
            .child(controls::status_dot(p.k.c.border_strong))
    } else {
        // A chord the tap can't hear is shown, never ticked.
        el().key("unheard").size(METRICS.icon, METRICS.icon)
    };
    controls::row_frame(lesson.key(), false)
        .gap(12.0)
        .label(if done { "Done" } else { "Not done yet" })
        .child(
            text("Press")
                .key("press")
                .font(p.row_title())
                .color(if done { p.muted() } else { p.text() }),
        )
        .child(controls::chords(p, chord.as_slice()))
        .child(el().key("space").grow(1.0))
        .child(mark)
}
