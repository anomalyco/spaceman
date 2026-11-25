//! The Try it step's miniature desktop as plain state: two workspaces of two windows each, and
//! the four moves the step teaches. It is a sandbox: nothing here touches a real window.

#[cfg(test)]
mod tests;

use spaceman::{
    WorkspaceId,
    shortcuts::{Chord, Key, Modifiers},
};

use crate::{Ui, panes};

/// The windows on the two workspaces, by the app each one is. The first of a pair has focus.
pub const APPS: [[&str; 2]; 2] = [
    ["com.apple.Safari", "com.apple.Terminal"],
    ["com.apple.Notes", "com.apple.mail"],
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Move {
    /// Switch to workspace 2.
    There,
    /// And back to workspace 1.
    Back,
    /// Toggle the focused window's floating.
    Float,
    /// Toggle Fullscreen Layout.
    Fullscreen,
}

impl Move {
    pub const ALL: [Self; 4] = [Self::There, Self::Back, Self::Float, Self::Fullscreen];

    pub const fn index(self) -> usize {
        match self {
            Self::There => 0,
            Self::Back => 1,
            Self::Float => 2,
            Self::Fullscreen => 3,
        }
    }

    pub const fn key(self) -> &'static str {
        match self {
            Self::There => "there",
            Self::Back => "back",
            Self::Float => "float",
            Self::Fullscreen => "fullscreen",
        }
    }

    /// What the move does, under its keys.
    pub const fn title(self) -> &'static str {
        match self {
            Self::There => "Switch to 2",
            Self::Back => "Back to 1",
            Self::Float => "Float",
            Self::Fullscreen => "Fullscreen",
        }
    }
}

/// How a workspace's focused window sits.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Mode {
    /// Both windows share the screen.
    #[default]
    Tiled,
    /// The focused window floats over the other, which takes the whole screen.
    Floated,
    /// Fullscreen Layout: the focused window fills the screen with the other behind it.
    Fullscreen,
}

/// How a window sits in its frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Pose {
    Tiled,
    /// Floating above the tiles.
    Lifted,
    /// Behind the window that fills the screen.
    Behind,
}

/// A window's place on its workspace, in unit space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub pose: Pose,
}

const fn frame(x: f32, y: f32, w: f32, h: f32, pose: Pose) -> Frame {
    Frame { x, y, w, h, pose }
}

/// Where a workspace's two windows sit in `mode`, the focused one first.
pub const fn frames(mode: Mode) -> [Frame; 2] {
    const WHOLE: Frame = frame(0.0, 0.0, 1.0, 1.0, Pose::Tiled);
    match mode {
        Mode::Tiled => [
            frame(0.0, 0.0, 0.5, 1.0, Pose::Tiled),
            frame(0.5, 0.0, 0.5, 1.0, Pose::Tiled),
        ],
        Mode::Floated => [frame(0.19, 0.15, 0.62, 0.7, Pose::Lifted), WHOLE],
        Mode::Fullscreen => [WHOLE, frame(0.0, 0.0, 1.0, 1.0, Pose::Behind)],
    }
}

/// The miniature desktop and which moves have been made on it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Demo {
    /// The workspace showing: 0 or 1.
    workspace: usize,
    modes: [Mode; 2],
    done: [bool; 4],
}

impl Demo {
    pub const fn workspace(self) -> usize {
        self.workspace
    }

    pub const fn mode(self, workspace: usize) -> Mode {
        self.modes[workspace]
    }

    pub const fn done(self, mv: Move) -> bool {
        self.done[mv.index()]
    }

    pub fn mastered(self) -> usize {
        self.done.iter().filter(|done| **done).count()
    }

    pub fn complete(self) -> bool {
        self.done.iter().all(|done| *done)
    }

    /// The move to try next: the first one not made yet.
    pub fn next(self) -> Option<Move> {
        Move::ALL.into_iter().find(|mv| !self.done(*mv))
    }

    /// Makes `mv` on the desktop. A switch to the workspace already showing does nothing; Float
    /// and Fullscreen toggle, each taking over from the other. A move counts the first time it
    /// changes something: going there, coming back, floating, filling the screen. Returns
    /// whether the desktop changed.
    pub fn press(&mut self, mv: Move) -> bool {
        let before = *self;
        let mode = &mut self.modes[self.workspace];
        let counts = match mv {
            Move::There => std::mem::replace(&mut self.workspace, 1) == 0,
            Move::Back => std::mem::replace(&mut self.workspace, 0) == 1,
            Move::Float => {
                *mode = toggled(*mode, Mode::Floated);
                *mode == Mode::Floated
            }
            Move::Fullscreen => {
                *mode = toggled(*mode, Mode::Fullscreen);
                *mode == Mode::Fullscreen
            }
        };
        self.done[mv.index()] |= counts;
        *self != before
    }
}

const fn toggled(mode: Mode, to: Mode) -> Mode {
    match (mode, to) {
        (Mode::Floated, Mode::Floated) | (Mode::Fullscreen, Mode::Fullscreen) => Mode::Tiled,
        _ => to,
    }
}

/// The chord each move asks for, from the shortcuts in force.
pub type Chords = [Option<Chord>; 4];

pub fn chords(ui: &Ui) -> Chords {
    let shortcuts = ui.doc.shortcuts();
    let named = ui.doc.named_workspaces();
    [
        panes::shortcuts::switch_chord(&shortcuts, WorkspaceId::TWO),
        panes::shortcuts::switch_chord(&shortcuts, WorkspaceId::ONE),
        panes::shortcuts::action_chord(&shortcuts, &named, "float"),
        panes::shortcuts::action_chord(&shortcuts, &named, "fullscreen"),
    ]
}

/// The move `chord` asks for, if it is one of `chords`.
pub fn asked(chords: &Chords, chord: Chord) -> Option<Move> {
    Move::ALL
        .into_iter()
        .zip(chords)
        .find_map(|(mv, wanted)| (*wanted == Some(chord)).then_some(mv))
}

/// Each move's chord as the key tap reports a press: a virtual key and Carbon modifiers.
pub type Presses = [Option<(u32, u32)>; 4];

// Carbon's modifier bits.
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

/// How each of `chords` is pressed. Number-row and named keys have fixed key codes; a
/// character's key depends on the keyboard layout, which `on_layout` answers.
pub fn presses(chords: &Chords, on_layout: impl Fn(Chord) -> Option<(u32, u32)>) -> Presses {
    chords.map(|chord| {
        let chord = chord?;
        let code = match chord.key {
            Key::Number(number) => *NUMBER_ROW.get(usize::from(number).checked_sub(1)?)?,
            Key::Code(code) => u32::from(code),
            Key::Character(_) => return on_layout(chord),
        };
        Some((code, carbon(chord.modifiers)))
    })
}

/// Whether the key tap can hear a press: it reports only presses with Option held.
pub fn hearable(press: Option<(u32, u32)>) -> bool {
    press.is_some_and(|(_, modifiers)| modifiers & OPTION != 0)
}

/// The move a press the key tap heard asks for, if it is one of `presses`.
pub fn heard(presses: &Presses, code: u32, modifiers: u32) -> Option<Move> {
    Move::ALL
        .into_iter()
        .zip(presses)
        .find_map(|(mv, press)| (*press == Some((code, modifiers))).then_some(mv))
}
