//! Which panes Settings lists: every pane, minus the Lab panes whose `[labs]` switch is off.

use super::*;
use rune::motion_gate::{Run, Track};

use crate::playground::reset_fixture;

fn activity_lab() -> LabsConfig {
    LabsConfig {
        activity: true,
        ..LabsConfig::default()
    }
}

#[test]
fn lab_panes_stay_out_of_the_sidebar_until_their_switch_is_on() {
    assert!(!Pane::Activity.visible(LabsConfig::default()));
    assert_eq!(
        Pane::listed(LabsConfig::default()).collect::<Vec<_>>(),
        vec![
            Pane::General,
            Pane::Layout,
            Pane::Rules,
            Pane::Workspaces,
            Pane::Shortcuts,
            Pane::Labs,
        ]
    );

    assert!(Pane::Activity.visible(activity_lab()));
    assert_eq!(
        Pane::listed(activity_lab()).collect::<Vec<_>>(),
        Pane::ALL.to_vec()
    );
    // The plain panes never depend on a switch.
    for pane in [
        Pane::General,
        Pane::Layout,
        Pane::Rules,
        Pane::Workspaces,
        Pane::Shortcuts,
        Pane::Labs,
    ] {
        assert!(pane.visible(LabsConfig::default()));
        assert!(pane.visible(activity_lab()));
    }
}

#[test]
fn command_shortcuts_skip_a_hidden_lab_pane() {
    assert_eq!(
        Pane::from_shortcut('5', LabsConfig::default()),
        Some(Pane::Shortcuts)
    );
    // Labs is always listed, last, so it takes the digit after the last pane shown.
    assert_eq!(
        Pane::from_shortcut('6', LabsConfig::default()),
        Some(Pane::Labs)
    );
    assert_eq!(Pane::from_shortcut('7', LabsConfig::default()), None);
    assert_eq!(
        Pane::from_shortcut('6', activity_lab()),
        Some(Pane::Activity)
    );
    assert_eq!(Pane::from_shortcut('7', activity_lab()), Some(Pane::Labs));
    assert_eq!(Pane::from_shortcut('8', activity_lab()), None);
}

#[test]
fn the_labs_deep_link_opens_the_labs_pane() {
    assert_eq!(
        Pane::from_deep_link(spaceman::deep_link::DeepLinkPane::Labs),
        Some(Pane::Labs)
    );
}

/// Settings drawn at rest on `pane`, stepped at 120 Hz on a virtual clock and read back as
/// presented rects, as `onboarding/motion_tests.rs` does for the tour.
fn settings_on(pane: Pane) -> Run<Ui> {
    let mut ui = crate::playground::blank_fixture();
    reset_fixture(&mut ui, pane);
    let mut rt = rune::Runtime::new(
        ui,
        crate::view::view,
        crate::controls::WINDOW.size,
        rune::Env::new(true, 2.0),
    );
    rt.on_key = Some(crate::on_key);
    let mut run = Run::new(rt, 120.0);
    run.idle(0.5);
    run.step();
    run
}

fn under<'a>(run: &'a Run<Ui>, body: &'a str) -> impl Iterator<Item = &'a Track> {
    let segment = format!("/{body}");
    run.rec
        .tracks
        .iter()
        .filter(move |track| track.path.contains(&segment))
}

/// Runs `input` on Settings open at `from`, records half a second, and checks the swap to `to`
/// landed in one frame: the old body is gone on the first frame after the input instead of
/// fading out under the new one, and every element of the new body (rows, keycap trays,
/// segmented controls, switches) is drawn whole and in its resting place on that frame, with
/// no fade, roll or slide into it.
fn assert_swaps_at_once(from: Pane, to: Pane, input: impl FnOnce(&mut rune::Runtime<Ui>)) {
    let mut run = settings_on(from);
    run.act("switch", input);
    let at = run.rec.frames - 1;
    run.play(0.5);
    let mut shown = from;
    run.rt.update_if(|ui, _| {
        shown = ui.pane;
        false
    });
    assert_eq!(shown, to, "{from:?} → {to:?}: the input selects the pane");

    let old = format!("{}-body", from.key());
    for track in under(&run, &old) {
        let lingering: Vec<_> = track
            .frames
            .iter()
            .filter(|(frame, snap)| *frame >= at && snap.op > 0.0)
            .map(|(frame, snap)| (frame - at, snap.op))
            .collect();
        assert!(
            lingering.is_empty(),
            "{from:?} → {to:?}: {} still drawn after the switch {lingering:?}",
            track.path
        );
    }

    let new = format!("{}-body", to.key());
    let mut drawn = 0;
    for track in under(&run, &new) {
        let frames: Vec<_> = track
            .frames
            .iter()
            .filter(|(frame, _)| *frame >= at)
            .collect();
        let Some(((first_frame, first), rest)) = frames.split_first().map(|(f, r)| (*f, r)) else {
            continue;
        };
        drawn += 1;
        let name = &track.path;
        assert_eq!(*first_frame, at, "{from:?} → {to:?}: {name} appears late");
        for (frame, snap) in &frames {
            let step = frame - at;
            assert!(
                (snap.op - first.op).abs() < 1e-3 && (snap.scale - 1.0).abs() < 1e-3,
                "{from:?} → {to:?}: {name} fades or scales in (+{step}: op {} scale {})",
                snap.op,
                snap.scale
            );
        }
        for (frame, snap) in rest {
            let moved = snap
                .rect
                .iter()
                .zip(first.rect)
                .any(|(a, b)| (a - b).abs() > 0.01);
            assert!(
                !moved,
                "{from:?} → {to:?}: {name} slides into place (+{}: {:?} from {:?})",
                frame - at,
                snap.rect,
                first.rect
            );
        }
    }
    assert!(drawn > 10, "{from:?} → {to:?}: the new pane is drawn");
}

/// A sidebar click swaps the pane with no cross-dissolve and none of the new pane's entrances
/// (the Shortcuts keycaps rolling up, the Workspaces segmented controls fading in).
#[test]
fn a_sidebar_click_swaps_the_pane_with_no_dissolve_or_entrance() {
    let hops = [
        (Pane::General, Pane::Shortcuts),
        (Pane::Shortcuts, Pane::Workspaces),
        (Pane::Workspaces, Pane::Shortcuts),
        (Pane::Workspaces, Pane::Labs),
        (Pane::Labs, Pane::General),
        (Pane::General, Pane::Layout),
    ];
    for (from, to) in hops {
        assert_swaps_at_once(from, to, |rt| Run::tap(rt, to.key()));
    }
}

/// A `⌘` digit swaps panes the same way a click does.
#[test]
fn a_command_digit_swaps_the_pane_with_no_dissolve_or_entrance() {
    assert_swaps_at_once(Pane::Workspaces, Pane::Shortcuts, |rt| {
        rt.key(rune::KeyEvent {
            key: rune::Key::Command('5'),
            mods: rune::Mods {
                cmd: true,
                logo: true,
                ..rune::Mods::default()
            },
        });
    });
}
