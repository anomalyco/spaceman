use rune::{Mods, PhysicalKeyCode};
use spaceman::shortcuts::parse_chord;

use super::*;
use crate::onboarding::{self, Input, Step, stage, try_it};

/// A US layout, where F is key 3.
fn us(chord: Chord) -> Option<(u32, u32)> {
    (chord.key == Key::Character('f')).then(|| (3, carbon(chord.modifiers)))
}

fn chord(source: &str) -> Chord {
    parse_chord(source).unwrap()
}

fn try_it_ui() -> Ui {
    let mut ui = crate::playground::blank_fixture();
    onboarding::tour_scene(&mut ui, Step::TryIt, true, false);
    ui
}

fn set_base_modifier(ui: &mut Ui, modifier: Modifiers) {
    ui.edit(|doc| doc.set_shortcut_modifier(modifier));
    assert_eq!(ui.error, None);
}

#[test]
fn switching_to_two_goes_there_and_counts_only_when_the_desktop_moves() {
    let mut demo = Demo::default();
    assert_eq!(demo.workspace(), 0);
    assert_eq!(demo.next(), Some(Move::There));
    // Coming back before going anywhere moves nothing and ticks nothing.
    assert!(!demo.press(Move::Back));
    assert!(!demo.done(Move::Back));

    assert!(demo.press(Move::There));
    assert_eq!(demo.workspace(), 1);
    assert!(demo.done(Move::There));
    assert_eq!(demo.next(), Some(Move::Back));
    assert!(!demo.press(Move::There), "already there");
    assert_eq!(demo.mastered(), 1);
}

#[test]
fn switching_back_to_one_counts_after_going_there() {
    let mut demo = Demo::default();
    demo.press(Move::There);
    assert!(demo.press(Move::Back));
    assert_eq!(demo.workspace(), 0);
    assert!(demo.done(Move::Back));
    assert_eq!(demo.next(), Some(Move::Float));
    assert_eq!(demo.mastered(), 2);
}

#[test]
fn floating_lifts_the_focused_window_and_the_other_takes_the_screen() {
    let mut demo = Demo::default();
    assert!(demo.press(Move::Float));
    assert_eq!(demo.mode(0), Mode::Floated);
    assert_eq!(
        demo.mode(1),
        Mode::Tiled,
        "the other workspace is untouched"
    );
    assert!(demo.done(Move::Float));

    let [lifted, other] = frames(Mode::Floated);
    assert_eq!(lifted.pose, Pose::Lifted);
    // Off the grid: inset from every edge.
    assert!(lifted.x > 0.0 && lifted.y > 0.0);
    assert!(lifted.x + lifted.w < 1.0 && lifted.y + lifted.h < 1.0);
    assert_eq!((other.x, other.y, other.w, other.h), (0.0, 0.0, 1.0, 1.0));
    assert_eq!(other.pose, Pose::Tiled);
    // On the stage the tile that stays is more than twice as wide as it was beside the other,
    // and starts at the screen's left edge.
    let [left, right] = frames(Mode::Tiled);
    let (left_x, _, _, _) = stage::rect(left, Mode::Tiled);
    let (_, _, tiled_w, _) = stage::rect(right, Mode::Tiled);
    let (wide_x, _, wide_w, _) = stage::rect(other, Mode::Floated);
    assert!(wide_w > 2.0 * tiled_w, "{wide_w} against {tiled_w}");
    assert_eq!(wide_x, left_x);

    // Pressing again puts it back on the grid; the tick stays.
    assert!(demo.press(Move::Float));
    assert_eq!(demo.mode(0), Mode::Tiled);
    assert!(demo.done(Move::Float));
}

#[test]
fn fullscreen_layout_toggles_and_keeps_the_other_window_behind() {
    let mut demo = Demo::default();
    assert!(demo.press(Move::Fullscreen));
    assert_eq!(demo.mode(0), Mode::Fullscreen);
    assert!(demo.done(Move::Fullscreen));

    let [front, behind] = frames(Mode::Fullscreen);
    assert_eq!((front.w, front.h, front.pose), (1.0, 1.0, Pose::Tiled));
    assert_eq!(behind.pose, Pose::Behind);
    // The window behind shows as a narrower tab above the one in front.
    let (front_x, front_y, front_w, _) = stage::rect(front, Mode::Fullscreen);
    let (tab_x, tab_y, tab_w, tab_h) = stage::rect(behind, Mode::Fullscreen);
    assert!(tab_y < front_y && tab_y + tab_h > front_y);
    assert!(tab_x > front_x && tab_x + tab_w < front_x + front_w);

    assert!(demo.press(Move::Fullscreen));
    assert_eq!(demo.mode(0), Mode::Tiled);

    // Float and Fullscreen take over from each other, one press each.
    demo.press(Move::Fullscreen);
    demo.press(Move::Float);
    assert_eq!(demo.mode(0), Mode::Floated);
    demo.press(Move::Fullscreen);
    assert_eq!(demo.mode(0), Mode::Fullscreen);
}

#[test]
fn four_moves_complete_it_in_any_order_and_free_play_goes_on() {
    let mut demo = Demo::default();
    for mv in [Move::Fullscreen, Move::Float, Move::There] {
        demo.press(mv);
        assert!(!demo.complete());
    }
    assert_eq!(demo.next(), Some(Move::Back));
    demo.press(Move::Back);
    assert!(demo.complete());
    assert_eq!(demo.mastered(), 4);
    assert_eq!(demo.next(), None);

    // After that every key still moves the desktop, and each workspace keeps its own layout.
    assert_eq!(demo.mode(0), Mode::Floated);
    assert!(demo.press(Move::Float));
    assert_eq!(demo.mode(0), Mode::Tiled);
    assert!(demo.press(Move::There));
    assert!(demo.press(Move::Fullscreen));
    assert_eq!(
        (demo.mode(0), demo.mode(1)),
        (Mode::Tiled, Mode::Fullscreen)
    );
    assert!(demo.press(Move::Fullscreen));
    assert!(demo.press(Move::Float));
    assert!(demo.press(Move::Back));
    assert_eq!((demo.workspace(), demo.mode(1)), (0, Mode::Floated));
    assert!(demo.complete());
}

#[test]
fn the_default_chords_are_heard_by_key_code() {
    let ui = crate::playground::blank_fixture();
    let chords = chords(&ui);
    assert_eq!(
        chords,
        [
            Some(chord("alt+2")),
            Some(chord("alt+1")),
            Some(chord("alt+shift+space")),
            Some(chord("alt+shift+f")),
        ]
    );
    let presses = presses(&chords, us);
    assert!(presses.iter().all(|press| hearable(*press)));
    assert_eq!(heard(&presses, 19, OPTION), Some(Move::There));
    assert_eq!(heard(&presses, 18, OPTION), Some(Move::Back));
    assert_eq!(heard(&presses, 49, OPTION | SHIFT), Some(Move::Float));
    assert_eq!(heard(&presses, 3, OPTION | SHIFT), Some(Move::Fullscreen));
    // Option-Shift-2 moves a window; Option-Space, Option-F and Option-3 are not moves.
    assert_eq!(heard(&presses, 19, OPTION | SHIFT), None);
    assert_eq!(heard(&presses, 49, OPTION), None);
    assert_eq!(heard(&presses, 3, OPTION), None);
    assert_eq!(heard(&presses, 20, OPTION), None);

    // A layout with no key for a character leaves that one move to the window's own keys.
    let unmapped = super::presses(&chords, |_| None);
    assert_eq!(unmapped.map(hearable), [true, true, true, false]);
}

#[test]
fn a_custom_base_modifier_moves_every_chord_and_the_window_hears_what_the_tap_cannot() {
    let mut ui = try_it_ui();
    set_base_modifier(
        &mut ui,
        Modifiers {
            control: true,
            ..Modifiers::default()
        },
    );
    let chords = chords(&ui);
    assert_eq!(
        chords,
        [
            Some(chord("ctrl+2")),
            Some(chord("ctrl+1")),
            Some(chord("ctrl+shift+space")),
            Some(chord("ctrl+shift+f")),
        ]
    );
    assert_eq!(asked(&chords, chord("ctrl+2")), Some(Move::There));
    assert_eq!(
        asked(&chords, chord("ctrl+shift+f")),
        Some(Move::Fullscreen)
    );
    assert_eq!(
        asked(&chords, chord("alt+2")),
        None,
        "the old chord is gone"
    );
    // Without Option the key tap hears none of them.
    let presses = super::presses(&chords, us);
    assert_eq!(presses[0], Some((19, CONTROL)));
    assert!(presses.iter().all(|press| !hearable(*press)));
    // The stage's own keyboard does, by the chord a physical press spells.
    let control = Mods {
        ctrl: true,
        ..Mods::default()
    };
    assert_eq!(
        try_it::asked(&ui, PhysicalKeyCode::Digit2, control),
        Some(Move::There)
    );
    assert_eq!(
        try_it::asked(&ui, PhysicalKeyCode::Digit1, control),
        Some(Move::Back)
    );
    let control_shift = Mods {
        shift: true,
        ..control
    };
    assert_eq!(
        try_it::asked(&ui, PhysicalKeyCode::Space, control_shift),
        Some(Move::Float)
    );
    let option = Mods {
        alt: true,
        ..Mods::default()
    };
    assert_eq!(try_it::asked(&ui, PhysicalKeyCode::Digit2, option), None);
    // No tap is up in a scene, so the window's keys make the move.
    assert!(!ui.tour.as_ref().unwrap().tap_hears(Move::There));

    // Control-Option keeps Option, so the tap hears it again.
    set_base_modifier(
        &mut ui,
        Modifiers {
            control: true,
            option: true,
            ..Modifiers::default()
        },
    );
    let presses = super::presses(&super::chords(&ui), us);
    assert_eq!(heard(&presses, 19, CONTROL | OPTION), Some(Move::There));
    assert_eq!(
        heard(&presses, 3, CONTROL | OPTION | SHIFT),
        Some(Move::Fullscreen)
    );
    assert_eq!(heard(&presses, 19, OPTION), None);
}

#[test]
fn a_press_sinks_its_keys_for_a_beat_and_a_held_modifier_for_as_long_as_it_is_held() {
    let mut play = try_it::Play::default();
    let there = chord("alt+2");
    let float = chord("alt+shift+space");
    let down = |play: &try_it::Play, mv, chord| -> Vec<bool> {
        play.caps(mv, chord)
            .into_iter()
            .map(|(_, down)| down)
            .collect()
    };
    assert_eq!(down(&play, Move::There, there), [false, false]);

    // Holding Option sinks every Option key, and nothing else.
    assert!(play.hold(Modifiers::OPTION));
    assert!(!play.hold(Modifiers::OPTION), "no change, no redraw");
    assert_eq!(down(&play, Move::There, there), [true, false]);
    assert_eq!(down(&play, Move::Float, float), [true, false, false]);

    // The press sinks the whole chord, on that move only, and makes the move.
    play.press(Move::There);
    assert_eq!(play.demo.workspace(), 1);
    assert_eq!(down(&play, Move::There, there), [true, true]);
    assert_eq!(down(&play, Move::Float, float), [true, false, false]);
    assert!(play.pending().is_some());

    std::thread::sleep(try_it::KEY_BEAT + std::time::Duration::from_millis(20));
    assert!(play.settle());
    assert!(!play.settle(), "settled once");
    assert_eq!(play.pending(), None);
    assert_eq!(down(&play, Move::There, there), [true, false]);
    assert!(play.hold(Modifiers::default()));
    assert_eq!(down(&play, Move::There, there), [false, false]);
}

#[test]
fn the_ring_shows_once_when_the_last_move_is_made() {
    let mut play = try_it::Play::default();
    for mv in [Move::There, Move::Back, Move::Float] {
        play.press(mv);
        assert!(!play.cheering());
    }
    play.press(Move::Fullscreen);
    assert!(play.demo.complete());
    assert!(play.cheering());
    play.rest();
    assert!(!play.cheering());
    assert_eq!(play.pending(), None);
    // Free play never brings it back.
    play.press(Move::Fullscreen);
    play.press(Move::Fullscreen);
    assert!(!play.cheering());
}

#[test]
fn a_press_counts_only_on_try_it_and_undo_starts_the_desktop_over() {
    let mut ui = crate::playground::blank_fixture();
    assert!(!onboarding::press(&mut ui, Move::There), "no tour");
    onboarding::tour_scene(&mut ui, Step::Arrange, true, false);
    assert!(
        !onboarding::press(&mut ui, Move::There),
        "not on Try it yet"
    );

    let mut ui = try_it_ui();
    // Even a press that moves nothing shows on its keys.
    assert!(onboarding::press(&mut ui, Move::Back));
    assert_eq!(ui.tour.as_ref().unwrap().play.demo, Demo::default());
    assert!(onboarding::press(&mut ui, Move::There));
    assert!(onboarding::press(&mut ui, Move::Float));
    let demo = ui.tour.as_ref().unwrap().play.demo;
    assert_eq!((demo.workspace(), demo.mode(1)), (1, Mode::Floated));

    onboarding::input(&mut ui, Input::Undo);
    let tour = ui.tour.as_ref().unwrap();
    assert_eq!(tour.flow.step(), Step::Arrange);
    assert_eq!(tour.play.demo, Demo::default());
}
