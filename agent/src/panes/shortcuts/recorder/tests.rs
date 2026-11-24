use rune::{Mods, PhysicalKeyCode as K};
use spaceman::shortcuts::parse_chord;

use super::*;

const ALT: Mods = Mods {
    shift: false,
    cmd: false,
    alt: true,
    ctrl: false,
    logo: false,
};

fn mods(alt: bool, ctrl: bool, shift: bool, cmd: bool) -> Mods {
    Mods {
        shift,
        cmd,
        alt,
        ctrl,
        logo: cmd,
    }
}

fn us(key: K, held: Mods) -> Press {
    press(key, held, us_character)
}

fn chord(source: &str) -> Press {
    Press::Chord(parse_chord(source).unwrap())
}

#[test]
fn a_press_records_the_held_modifiers_and_its_key() {
    assert_eq!(us(K::KeyL, ALT), chord("alt+l"));
    assert_eq!(
        us(K::KeyL, mods(true, true, true, true)),
        chord("ctrl+alt+shift+cmd+l")
    );
    assert_eq!(us(K::Digit0, ALT), chord("alt+0"));
    assert_eq!(
        us(K::ArrowLeft, mods(false, true, false, false)),
        chord("ctrl+left")
    );
    assert_eq!(
        us(K::Space, mods(true, false, true, false)),
        chord("alt+shift+space")
    );
    assert_eq!(us(K::Slash, ALT), chord("alt+/"));
    assert_eq!(us(K::F5, mods(false, false, false, true)), chord("cmd+f5"));
    assert_eq!(us(K::Backspace, ALT), chord("alt+backspace"));
    // The active layout names a character key: AZERTY types `a` on the US Q key.
    let azerty = |key| (key == K::KeyQ).then_some('A');
    assert_eq!(press(K::KeyQ, ALT, azerty), chord("alt+a"));
}

#[test]
fn bare_and_shift_only_keys_are_refused() {
    assert_eq!(us(K::KeyL, Mods::default()), Press::Refused(NEEDS_MODIFIER));
    assert_eq!(
        us(K::KeyL, mods(false, false, true, false)),
        Press::Refused(SHIFT_ALONE)
    );
    assert_eq!(
        us(K::ShiftLeft, mods(false, false, true, false)),
        Press::Ignore
    );
    assert_eq!(us(K::AltRight, ALT), Press::Ignore);
}

#[test]
fn escape_cancels_and_a_bare_delete_unbinds() {
    assert_eq!(us(K::Escape, Mods::default()), Press::Cancel);
    assert_eq!(us(K::Escape, ALT), Press::Cancel);
    assert_eq!(us(K::Backspace, Mods::default()), Press::Unbind);
    assert_eq!(
        us(K::Delete, mods(false, false, true, false)),
        Press::Unbind
    );
}

fn listening(action: &'static str) -> Ui {
    let mut ui = crate::playground::blank_fixture();
    begin(&mut ui, action);
    ui
}

fn message(ui: &Ui) -> Option<&str> {
    match &ui.shortcut_capture {
        Capture::Listening { message, .. } => message.as_deref(),
        _ => None,
    }
}

#[test]
fn recording_saves_the_chord_and_moves_the_built_in_off_its_old_one() {
    let mut ui = listening("focus-left");
    record(
        &mut ui,
        "focus-left",
        us(K::KeyH, mods(true, true, false, false)),
    );
    assert_eq!(
        ui.shortcut_capture,
        Capture::Saved {
            action: "focus-left"
        }
    );
    let shortcuts = ui.doc.shortcuts();
    let focus_left = catalog::find("focus-left").unwrap();
    assert_eq!(
        focus_left.chords(&shortcuts),
        [parse_chord("ctrl+alt+h").unwrap()]
    );
    assert_eq!(shortcuts.binding(parse_chord("alt+h").unwrap()), Some(None));
}

#[test]
fn a_chord_another_shortcut_uses_is_refused_by_name() {
    let mut ui = listening("focus-left");
    record(&mut ui, "focus-left", us(K::KeyL, ALT));
    assert_eq!(message(&ui), Some("Used by Focus right"));
    record(
        &mut ui,
        "focus-left",
        us(K::Digit3, mods(true, false, true, false)),
    );
    assert_eq!(message(&ui), Some("Used by Move window to workspace 3"));
    assert_eq!(ui.doc.shortcuts(), Shortcuts::default());
    // Its own chord is no conflict: recording it keeps the built-in.
    record(&mut ui, "focus-left", us(K::KeyH, ALT));
    assert_eq!(
        ui.shortcut_capture,
        Capture::Saved {
            action: "focus-left"
        }
    );
    assert!(!ui.doc.has_custom_shortcuts());
}

#[test]
fn a_refusal_keeps_listening_and_escape_leaves_everything_as_it_was() {
    let mut ui = listening("float");
    record(&mut ui, "float", us(K::KeyV, Mods::default()));
    assert_eq!(message(&ui), Some(NEEDS_MODIFIER));
    record(&mut ui, "float", us(K::Escape, Mods::default()));
    assert_eq!(ui.shortcut_capture, Capture::Idle);
    assert!(!ui.doc.has_custom_shortcuts());
}

#[test]
fn delete_unbinds_to_none() {
    let mut ui = listening("float");
    record(&mut ui, "float", us(K::Backspace, Mods::default()));
    let shortcuts = ui.doc.shortcuts();
    assert!(
        catalog::find("float")
            .unwrap()
            .chords(&shortcuts)
            .is_empty()
    );
    assert_eq!(
        shortcuts.binding(parse_chord("alt+shift+space").unwrap()),
        Some(None)
    );
}

#[test]
fn pressing_the_listening_shortcut_again_cancels() {
    let mut ui = listening("grow");
    begin(&mut ui, "grow");
    assert_eq!(ui.shortcut_capture, Capture::Idle);
    begin(&mut ui, "grow");
    begin(&mut ui, "shrink");
    assert_eq!(ui.shortcut_capture.listening(), Some("shrink"));
}

#[test]
fn held_modifiers_show_and_clear_a_refusal() {
    let mut ui = listening("grow");
    record(&mut ui, "grow", us(K::KeyG, Mods::default()));
    assert!(on_input(&mut ui, &PhysicalInput::Modifiers(ALT)));
    assert_eq!(
        ui.shortcut_capture,
        Capture::Listening {
            action: "grow",
            held: Modifiers::OPTION,
            message: None,
        }
    );
    assert!(on_input(&mut ui, &PhysicalInput::FocusLost));
    assert_eq!(ui.shortcut_capture, Capture::Idle);
    assert!(!on_input(&mut ui, &PhysicalInput::Modifiers(ALT)));
}
