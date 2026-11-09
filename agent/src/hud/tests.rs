use spaceman::{
    Axis, Rect, WorkspaceId,
    app::{UiHudEvent, UiHudKind, UiHudLayout},
};

use super::{HOLD_MS, HudFlow, HudTick, HudTransition, LEAVE_MS, Travel, pill::labels};

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

fn switch(workspace: u8) -> UiHudEvent {
    sample_event(workspace, UiHudKind::Workspace, 1)
}

fn shown(flow: &HudFlow) -> bool {
    flow.view("test", false).is_some_and(|view| view.shown)
}

fn travel(flow: &HudFlow) -> (Travel, bool) {
    let view = flow.view("test", false).unwrap();
    (view.travel, view.fresh)
}

#[test]
fn shows_on_first_trigger_and_updates_in_place_while_visible() {
    let mut flow = HudFlow::new(true);
    assert!(flow.view("test", false).is_none());

    assert_eq!(flow.trigger(switch(1), 1_000), HudTransition::Show);
    assert!(shown(&flow));
    assert_eq!(flow.remaining_ms(1_400), Some(HOLD_MS - 400));

    // A rapid second switch extends the hold without showing the panel again.
    assert_eq!(flow.trigger(switch(2), 1_500), HudTransition::Update);
    assert_eq!(flow.tick(1_000 + HOLD_MS), HudTick::Idle);
    assert!(shown(&flow));

    // The hold runs out: the capsule leaves, and the panel stays up until its exit has played.
    let left = 1_500 + HOLD_MS;
    assert_eq!(flow.tick(left), HudTick::Leave);
    assert!(!shown(&flow));
    assert_eq!(flow.remaining_ms(left), Some(LEAVE_MS));
    assert_eq!(flow.tick(left + LEAVE_MS - 1), HudTick::Idle);
    // The last event stays through the exit, so the capsule never blanks on its way out.
    assert_eq!(
        flow.view("test", false).map(|view| view.event.workspace),
        Some(WorkspaceId::TWO)
    );
    assert_eq!(flow.tick(left + LEAVE_MS), HudTick::Gone);
    assert_eq!(flow.remaining_ms(left + LEAVE_MS), None);
    assert_eq!(flow.tick(left + LEAVE_MS + 1), HudTick::Idle);
}

#[test]
fn a_switch_during_the_exit_brings_the_same_capsule_back() {
    let mut flow = HudFlow::new(true);
    flow.trigger(switch(1), 0);
    assert_eq!(flow.tick(HOLD_MS), HudTick::Leave);

    // Not a new show: the panel is still up and the capsule turns around where it is.
    assert_eq!(flow.trigger(switch(2), HOLD_MS + 60), HudTransition::Update);
    assert!(shown(&flow));
    assert_eq!(travel(&flow), (Travel::Forward, false));
    assert_eq!(flow.tick(HOLD_MS + LEAVE_MS), HudTick::Idle);
    assert_eq!(flow.tick(HOLD_MS + 60 + HOLD_MS), HudTick::Leave);
}

#[test]
fn content_travels_only_while_the_hud_is_up() {
    let mut flow = HudFlow::new(true);
    flow.trigger(switch(2), 0);
    assert_eq!(travel(&flow), (Travel::Stay, true));

    flow.trigger(switch(4), 200);
    assert_eq!(travel(&flow), (Travel::Forward, false));
    flow.trigger(switch(1), 400);
    assert_eq!(travel(&flow), (Travel::Back, false));
    flow.trigger(
        sample_event(1, UiHudKind::Fullscreen { enabled: true }, 1),
        600,
    );
    assert_eq!(travel(&flow), (Travel::Stay, false));
    flow.trigger(
        UiHudEvent {
            workspace: WorkspaceId::named('B').unwrap(),
            ..switch(1)
        },
        800,
    );
    assert_eq!(
        travel(&flow),
        (Travel::Forward, false),
        "named follow numbered"
    );

    // Once it has gone, the next event lands at rest whatever it showed before.
    assert_eq!(flow.tick(800 + HOLD_MS), HudTick::Leave);
    assert_eq!(flow.tick(800 + HOLD_MS + LEAVE_MS), HudTick::Gone);
    flow.trigger(switch(3), 5_000);
    assert_eq!(travel(&flow), (Travel::Stay, true));
    // So does one right behind it, which arrives before the capsule has drawn.
    flow.trigger(switch(4), 5_010);
    assert_eq!(travel(&flow), (Travel::Stay, true));
    flow.trigger(switch(5), 5_200);
    assert_eq!(travel(&flow), (Travel::Forward, false));
}

#[test]
fn disabling_hud_suppresses_triggers_and_dismisses_active_pill() {
    let mut flow = HudFlow::new(true);
    assert_eq!(
        flow.trigger(sample_event(3, UiHudKind::Moved, 1), 500),
        HudTransition::Show
    );
    assert!(flow.set_enabled(false, 600));
    assert!(!shown(&flow));
    assert_eq!(
        flow.trigger(sample_event(4, UiHudKind::Workspace, 1), 650),
        HudTransition::Ignored
    );
    assert_eq!(flow.tick(600 + LEAVE_MS), HudTick::Gone);
    // Turning it off again while it's already hidden leaves nothing to dismiss.
    assert!(!flow.set_enabled(false, 2_000));
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
        labels(&UiHudEvent {
            preview: UiHudLayout::Fullscreen,
            ..sample_event(2, UiHudKind::Workspace, 3)
        }),
        ("Workspace 2".into(), "3 windows · Fullscreen".into())
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
