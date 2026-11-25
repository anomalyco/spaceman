//! Frame-by-frame checks of the Try it step: the real view, clicked and typed at as a window
//! would be, stepped at 120 Hz on a virtual clock and read back as presented rects.

use rune::{Env, Mods, PhysicalInput, PhysicalKeyCode, Runtime, motion_gate::Run};

use super::{
    Step,
    demo::{Demo, Mode},
    stage,
};
use crate::Ui;

const HZ: f64 = 120.0;

struct Tour {
    run: Run<Ui>,
}

impl Tour {
    /// The Try it step, drawn and at rest.
    fn on_try_it() -> Self {
        Self::on(Step::TryIt)
    }

    fn on(step: Step) -> Self {
        let mut ui = crate::playground::blank_fixture();
        super::tour_scene(&mut ui, step, true, false);
        let rt = Runtime::new(ui, crate::view::view, (920.0, 660.0), Env::new(true, 2.0));
        let mut run = Run::new(rt, HZ);
        run.idle(0.5);
        run.step();
        Self { run }
    }

    fn click(&mut self, key: &'static str) -> usize {
        self.run.act(key, |rt| Run::tap(rt, key));
        self.run.rec.frames - 1
    }

    fn press(&mut self, key: PhysicalKeyCode, mods: Mods) -> usize {
        self.run.act("key", |rt| {
            rt.physical_input(PhysicalInput::Key {
                key,
                mods,
                pressed: true,
                repeat: false,
            });
        });
        self.run.rec.frames - 1
    }

    fn demo(&mut self) -> Demo {
        let mut demo = Demo::default();
        self.run.rt.update_if(|ui, _| {
            demo = ui.tour.as_ref().unwrap().play.demo;
            false
        });
        demo
    }

    /// The presented rect of the element whose path ends in `path`, on each frame from `from`.
    fn rects(&self, path: &str, from: usize) -> Vec<[f32; 4]> {
        let suffix = format!("/{path}");
        let track = self
            .run
            .rec
            .tracks
            .iter()
            .rfind(|track| track.path == path || track.path.ends_with(&suffix))
            .unwrap_or_else(|| panic!("nothing drawn at {path}"));
        track
            .frames
            .iter()
            .filter(|(frame, _)| *frame >= from)
            .map(|(_, snap)| snap.rect)
            .collect()
    }

    fn findings(&self) -> Vec<String> {
        self.run.finish().iter().map(ToString::to_string).collect()
    }
}

fn option() -> Mods {
    Mods {
        alt: true,
        ..Mods::default()
    }
}

fn option_shift() -> Mods {
    Mods {
        shift: true,
        ..option()
    }
}

/// Every step between two frames is smaller than `most`, so nothing jumped.
fn glides(xs: &[f32], most: f32) -> bool {
    xs.windows(2).all(|pair| (pair[1] - pair[0]).abs() < most)
}

#[test]
fn a_click_makes_the_move_and_the_workspaces_slide_with_the_marker_stretching() {
    let mut tour = Tour::on_try_it();
    let strip_rest = tour.rects("strip", 0)[0];
    let marker_rest = tour.rects("marker", 0)[0];

    let from = tour.click("there");
    assert_eq!(tour.demo().workspace(), 1);
    tour.run.play(1.0);

    // The strip travels one screen to the left, and never more than a few points a frame.
    let strip: Vec<f32> = tour
        .rects("strip", from)
        .iter()
        .map(|rect| rect[0])
        .collect();
    let travelled = strip_rest[0] - strip[strip.len() - 1];
    assert!(travelled > 400.0, "{travelled}");
    assert!(glides(&strip, 30.0), "{strip:?}");
    // The HUD's marker goes to the second group, and is wider on the way than at either end.
    let marker = tour.rects("marker", from);
    let landed = marker[marker.len() - 1];
    assert!(landed[0] > marker_rest[0] + 30.0);
    assert!((landed[2] - marker_rest[2]).abs() < 0.5);
    let widest = marker.iter().map(|rect| rect[2]).fold(0.0, f32::max);
    assert!(
        widest > marker_rest[2] + 4.0,
        "{widest} against {}",
        marker_rest[2]
    );
    assert_eq!(tour.findings(), Vec::<String>::new());
}

#[test]
fn a_click_gives_the_stage_the_keyboard_and_real_keys_make_the_other_moves() {
    let mut tour = Tour::on_try_it();
    // Before anything has focus, a chord goes nowhere.
    tour.press(PhysicalKeyCode::Digit2, option());
    assert_eq!(tour.demo().workspace(), 0);

    tour.click("there");
    tour.run.play(0.6);
    assert_eq!(tour.demo().workspace(), 1);
    // The click handed the keyboard to the stage: Option-1 comes back.
    tour.press(PhysicalKeyCode::Digit1, option());
    assert_eq!(tour.demo().workspace(), 0);
    tour.run.play(0.6);
    // A held key's repeats and its release make no second move.
    tour.press(PhysicalKeyCode::Space, option_shift());
    assert_eq!(tour.demo().mode(0), Mode::Floated);
    tour.run.act("repeat", |rt| {
        rt.physical_input(PhysicalInput::Key {
            key: PhysicalKeyCode::Space,
            mods: option_shift(),
            pressed: true,
            repeat: true,
        });
        rt.physical_input(PhysicalInput::Key {
            key: PhysicalKeyCode::Space,
            mods: option_shift(),
            pressed: false,
            repeat: false,
        });
    });
    assert_eq!(tour.demo().mode(0), Mode::Floated);
    // Option-Shift-2 moves a window in the real thing; it is not one of the four.
    tour.press(PhysicalKeyCode::Digit2, option_shift());
    assert_eq!(tour.demo().workspace(), 0);
    // Clicking the display itself takes the keyboard too.
    tour.click("float");
    tour.click(stage::KEY);
    tour.press(PhysicalKeyCode::Digit2, option());
    assert_eq!(tour.demo().workspace(), 1);
    assert_eq!(tour.demo().mastered(), 3);
}

#[test]
fn arriving_from_the_arrangement_gives_the_stage_the_keyboard() {
    let mut tour = Tour::on(Step::Arrange);
    tour.click("tour-continue");
    tour.run.play(0.8);
    let mut step = Step::Welcome;
    tour.run.rt.update_if(|ui, _| {
        step = ui.tour.as_ref().unwrap().flow.step();
        false
    });
    assert_eq!(step, Step::TryIt);
    // No click on the step yet, and its shortcut already lands.
    tour.press(PhysicalKeyCode::Digit2, option());
    assert_eq!(tour.demo().workspace(), 1);
}

#[test]
fn floating_lifts_the_window_off_the_grid_while_the_other_springs_wide() {
    let mut tour = Tour::on_try_it();
    let safari_rest = tour.rects("w-com.apple.Safari", 0)[0];
    let terminal_rest = tour.rects("w-com.apple.Terminal", 0)[0];

    let from = tour.click("float");
    tour.run.play(1.0);
    let safari = tour.rects("w-com.apple.Safari", from);
    let terminal = tour.rects("w-com.apple.Terminal", from);
    let lifted = safari[safari.len() - 1];
    let wide = terminal[terminal.len() - 1];
    // Off the grid: inset from where the two tiles reached, and off the top.
    assert!(lifted[0] > safari_rest[0] + 20.0 && lifted[1] > safari_rest[1] + 20.0);
    assert!(lifted[0] + lifted[2] < terminal_rest[0] + terminal_rest[2] - 20.0);
    // The other takes both tiles' room, starting where the first one did.
    assert!((wide[0] - safari_rest[0]).abs() < 0.5, "{wide:?}");
    assert!(wide[2] > 2.0 * terminal_rest[2], "{wide:?}");
    let widths: Vec<f32> = terminal.iter().map(|rect| rect[2]).collect();
    assert!(glides(&widths, 30.0), "{widths:?}");

    // Again, and both are back on the grid.
    let from = tour.click("float");
    tour.run.play(1.0);
    let safari = tour.rects("w-com.apple.Safari", from);
    let terminal = tour.rects("w-com.apple.Terminal", from);
    let settled = |rect: [f32; 4], rest: [f32; 4]| {
        rect.iter()
            .zip(rest)
            .all(|(now, rest)| (now - rest).abs() < 0.5)
    };
    assert!(settled(safari[safari.len() - 1], safari_rest));
    assert!(settled(terminal[terminal.len() - 1], terminal_rest));
    assert_eq!(tour.findings(), Vec::<String>::new());
}

#[test]
fn fullscreen_fills_the_screen_and_leaves_a_tab_of_the_window_behind() {
    let mut tour = Tour::on_try_it();
    let safari_rest = tour.rects("w-com.apple.Safari", 0)[0];
    let terminal_rest = tour.rects("w-com.apple.Terminal", 0)[0];

    let from = tour.click("fullscreen");
    tour.run.play(1.0);
    let front = *tour.rects("w-com.apple.Safari", from).last().unwrap();
    let tab = *tour.rects("w-com.apple.Terminal", from).last().unwrap();
    assert!(front[2] > 2.0 * safari_rest[2], "{front:?}");
    assert!(tab[1] < front[1] && tab[3] < 20.0, "{tab:?} over {front:?}");
    assert!(tab[0] > front[0] && tab[0] + tab[2] < front[0] + front[2]);

    let from = tour.click("fullscreen");
    tour.run.play(1.0);
    let back = *tour.rects("w-com.apple.Terminal", from).last().unwrap();
    assert!((back[2] - terminal_rest[2]).abs() < 0.5 && (back[3] - terminal_rest[3]).abs() < 0.5);
    assert_eq!(tour.findings(), Vec::<String>::new());
}
