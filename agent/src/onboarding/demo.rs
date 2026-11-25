//! The Try it step's miniature desktop as plain state: two workspaces of two windows each, the
//! four shortcuts that move it, and the six lessons the step teaches with them, one at a time.
//! It is a sandbox: nothing here touches a real window.

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

/// A shortcut the step listens for.
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
}

/// One thing to try: a shortcut, and what pressing it now does.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Lesson {
    pub mv: Move,
    pub title: &'static str,
}

/// The lessons in the order they are asked for. Float and Fullscreen Layout are each pressed
/// twice: once to make the change and once to undo it.
pub const LESSONS: [Lesson; 6] = [
    lesson(Move::There, "Switch to workspace 2"),
    lesson(Move::Back, "Switch back to workspace 1"),
    lesson(Move::Float, "Float this window"),
    lesson(Move::Float, "Tile it again"),
    lesson(Move::Fullscreen, "Fill the screen"),
    lesson(Move::Fullscreen, "Back to tiles"),
];

const fn lesson(mv: Move, title: &'static str) -> Lesson {
    Lesson { mv, title }
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
        Mode::Floated => [frame(0.2, 0.13, 0.6, 0.7, Pose::Lifted), WHOLE],
        Mode::Fullscreen => [WHOLE, frame(0.0, 0.0, 1.0, 1.0, Pose::Behind)],
    }
}

/// What a press did.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Pressed {
    /// The desktop changed.
    pub moved: bool,
    /// The press was the lesson being asked for, so the next one is up.
    pub learned: bool,
}

/// The miniature desktop and how many lessons have been learned on it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Demo {
    /// The workspace showing: 0 or 1.
    workspace: usize,
    modes: [Mode; 2],
    learned: usize,
}

impl Demo {
    pub const fn workspace(self) -> usize {
        self.workspace
    }

    pub const fn mode(self, workspace: usize) -> Mode {
        self.modes[workspace]
    }

    /// How many lessons are done: the place in `LESSONS` of the one being asked for.
    pub const fn learned(self) -> usize {
        self.learned
    }

    pub const fn complete(self) -> bool {
        self.learned == LESSONS.len()
    }

    /// The lesson being asked for.
    pub fn next(self) -> Option<Lesson> {
        LESSONS.get(self.learned).copied()
    }

    /// Makes `mv` on the desktop. A switch to the workspace already showing does nothing; Float
    /// and Fullscreen toggle, each taking over from the other. Every shortcut moves the desktop
    /// whichever lesson is up, and after the last; the lesson is learned when its own shortcut
    /// does what it asks for.
    pub fn press(&mut self, mv: Move) -> Pressed {
        let before = *self;
        match mv {
            Move::There => self.workspace = 1,
            Move::Back => self.workspace = 0,
            Move::Float => self.toggle(Mode::Floated),
            Move::Fullscreen => self.toggle(Mode::Fullscreen),
        }
        let moved = *self != before;
        let was = before.modes[before.workspace];
        let now = self.modes[self.workspace];
        let learned = moved
            && self.next().is_some_and(|lesson| lesson.mv == mv)
            && match self.learned {
                // Coming back out of each layout is its own lesson.
                3 => was == Mode::Floated && now == Mode::Tiled,
                5 => was == Mode::Fullscreen && now == Mode::Tiled,
                2 => now == Mode::Floated,
                4 => now == Mode::Fullscreen,
                _ => true,
            };
        if learned {
            self.learned += 1;
        }
        Pressed { moved, learned }
    }

    fn toggle(&mut self, to: Mode) {
        let mode = &mut self.modes[self.workspace];
        *mode = if *mode == to { Mode::Tiled } else { to };
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
