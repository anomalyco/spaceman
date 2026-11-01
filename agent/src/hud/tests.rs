use spaceman::{
    Axis, Rect, WorkspaceId,
    app::{UiHudEvent, UiHudKind, UiHudLayout},
};

use super::{HOLD_MS, HudFlow, HudTransition, labels};

fn sample_event(workspace: u8, kind: UiHudKind, windows: usize) -> UiHudEvent {
    UiHudEvent {
        workspace: WorkspaceId::new(workspace).unwrap(),
        kind,
        preview: UiHudLayout::Split(vec![Rect {
            x: 0,
            y: 0,
            width: 1500,
            height: 1100,
        }]),
        windows,
    }
}

#[test]
fn shows_on_first_trigger_and_updates_in_place_while_visible() {
    let mut flow = HudFlow::new(true);
    assert!(!flow.visible());

    assert_eq!(
        flow.trigger(sample_event(1, UiHudKind::Workspace, 2), 1_000),
        HudTransition::Show
    );
    assert!(flow.visible());
    assert_eq!(flow.remaining_ms(1_400), Some(HOLD_MS - 400));

    // Rapid second switch extends the hold timer without re-triggering fade-in.
    assert_eq!(
        flow.trigger(sample_event(2, UiHudKind::Workspace, 3), 1_500),
        HudTransition::Update
    );
    assert!(!flow.tick(1_000 + HOLD_MS));
    assert!(flow.visible());

    assert!(flow.tick(1_500 + HOLD_MS));
    assert!(!flow.visible());
    // Retains last event so fade-out never blanks mid-animation.
    assert_eq!(
        flow.current().map(|event| event.workspace),
        Some(WorkspaceId::TWO)
    );
}

#[test]
fn disabling_hud_suppresses_triggers_and_dismisses_active_pill() {
    let mut flow = HudFlow::new(true);
    assert_eq!(
        flow.trigger(sample_event(3, UiHudKind::Moved, 1), 500),
        HudTransition::Show
    );
    assert!(flow.set_enabled(false));
    assert!(!flow.visible());
    assert_eq!(
        flow.trigger(sample_event(4, UiHudKind::Workspace, 1), 600),
        HudTransition::Hidden
    );
}

#[test]
fn formats_title_free_labels_for_every_hud_event_kind() {
    assert_eq!(
        labels(&sample_event(2, UiHudKind::Workspace, 0)),
        ("Workspace 2".into(), "Empty".into())
    );
    assert_eq!(
        labels(&sample_event(2, UiHudKind::Workspace, 3)),
        ("Workspace 2".into(), "3 windows".into())
    );
    assert_eq!(
        labels(&sample_event(4, UiHudKind::Moved, 1)),
        ("Moved to Workspace 4".into(), "1 window".into())
    );
    assert_eq!(
        labels(&sample_event(
            1,
            UiHudKind::Layout {
                axis: Axis::Vertical
            },
            2
        )),
        ("Workspace 1".into(), "Vertical split".into())
    );
    assert_eq!(
        labels(&UiHudEvent {
            workspace: WorkspaceId::ONE,
            kind: UiHudKind::Fullscreen { enabled: true },
            preview: UiHudLayout::Fullscreen,
            windows: 2,
        }),
        ("Workspace 1".into(), "Fullscreen layout".into())
    );
}
