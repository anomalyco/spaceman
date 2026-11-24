use spaceman::{
    Rect, WorkspaceId,
    app::{UiHudDisplay, UiHudTile, UiHudWorkspace},
};

use crate::rules::Place;

use super::{
    FRESH_MS, HOLD_MS, HudFlow, HudTick, HudTransition, LEAVE_MS, RESUME_HOLD_MS, assign_panels,
    grow, icons, menu,
};

fn id(number: u8) -> WorkspaceId {
    WorkspaceId::new(number).unwrap()
}

fn workspace(number: u8, apps: &[&str]) -> UiHudWorkspace {
    let count = apps.len().max(1) as i32;
    UiHudWorkspace {
        workspace: id(number),
        fullscreen: false,
        tiles: apps
            .iter()
            .zip(0..)
            .map(|(bundle, index)| UiHudTile {
                key: u64::from(number) * 100 + index as u64,
                rect: Rect {
                    x: index * 1500 / count,
                    y: 0,
                    width: 1500 / count,
                    height: 1100,
                },
                bundle: Some((*bundle).to_owned()),
            })
            .collect(),
        floating: Vec::new(),
    }
}

/// Workspaces 1 to 3 occupied, `visible` shown, arrived at from `from`.
fn row(visible: u8, from: Option<u8>) -> UiHudDisplay {
    let mut workspaces = vec![
        workspace(1, &["a", "b"]),
        workspace(2, &["c"]),
        workspace(3, &["d", "e", "f"]),
    ];
    if visible > 3 {
        workspaces.push(workspace(visible, &[]));
    }
    UiHudDisplay {
        id: "main".to_owned(),
        visible: Some(id(visible)),
        from: from.map(id),
        workspaces,
        order: Vec::new(),
    }
}

fn shown(flow: &HudFlow) -> bool {
    flow.view(false).is_some_and(|view| view.shown)
}

fn focus(flow: &HudFlow) -> (Option<WorkspaceId>, bool) {
    let view = flow.view(false).unwrap();
    (view.focus, view.fresh)
}

/// Shown and arrived, as the overlay's first frame and poll leave it.
fn arrived(flow: &mut HudFlow, row: UiHudDisplay, now_ms: u64) {
    flow.trigger(row, now_ms);
    flow.mark_drawn();
    assert_eq!(flow.tick(now_ms + FRESH_MS), HudTick::Arrive);
}

#[test]
fn shows_on_first_trigger_and_updates_in_place_while_visible() {
    let mut flow = HudFlow::new(true);
    assert!(flow.view(false).is_none());

    arrived(&mut flow, row(1, None), 1_000);
    assert!(shown(&flow));
    assert_eq!(flow.remaining_ms(1_400), Some(HOLD_MS - 400));

    // A rapid second switch extends the hold without showing the panel again.
    assert_eq!(flow.trigger(row(2, Some(1)), 1_500), HudTransition::Update);
    assert_eq!(flow.tick(1_000 + HOLD_MS), HudTick::Idle);
    assert!(shown(&flow));

    // The hold runs out: the capsule leaves, and the panel stays up until its exit has played.
    let left = 1_500 + HOLD_MS;
    assert_eq!(flow.tick(left), HudTick::Leave);
    assert!(!shown(&flow));
    assert_eq!(flow.remaining_ms(left), Some(LEAVE_MS));
    assert_eq!(flow.tick(left + LEAVE_MS - 1), HudTick::Idle);
    // The last row stays through the exit, so the capsule never blanks on its way out.
    assert_eq!(focus(&flow).0, Some(id(2)));
    assert_eq!(flow.tick(left + LEAVE_MS), HudTick::Gone);
    assert_eq!(flow.remaining_ms(left + LEAVE_MS), None);
    assert_eq!(flow.tick(left + LEAVE_MS + 1), HudTick::Idle);
}

#[test]
fn a_switch_during_the_exit_brings_the_same_capsule_back() {
    let mut flow = HudFlow::new(true);
    arrived(&mut flow, row(1, None), 0);
    assert_eq!(flow.tick(HOLD_MS), HudTick::Leave);

    // Not a new show: the panel is still up and the marker heads straight for the new one.
    assert_eq!(
        flow.trigger(row(2, Some(1)), HOLD_MS + 60),
        HudTransition::Update
    );
    assert!(shown(&flow));
    assert_eq!(focus(&flow), (Some(id(2)), false));
    assert_eq!(flow.tick(HOLD_MS + LEAVE_MS), HudTick::Idle);
    assert_eq!(flow.tick(HOLD_MS + 60 + HOLD_MS), HudTick::Leave);
}

#[test]
fn a_fresh_show_starts_on_the_workspace_the_switch_left() {
    let mut flow = HudFlow::new(true);
    assert_eq!(flow.trigger(row(3, Some(1)), 0), HudTransition::Show);
    assert_eq!(focus(&flow), (Some(id(1)), true));
    // Not until the panel has drawn it there, however long that takes.
    assert_eq!(flow.tick(FRESH_MS), HudTick::Idle);
    assert_eq!(flow.remaining_ms(FRESH_MS), Some(16));
    // Before the first frame drew, a second switch still starts from where the first left.
    flow.trigger(row(2, Some(3)), FRESH_MS + 5);
    assert_eq!(focus(&flow), (Some(id(1)), true));
    flow.mark_drawn();
    assert_eq!(flow.tick(FRESH_MS + 10), HudTick::Arrive);
    assert_eq!(focus(&flow), (Some(id(2)), false));
    assert_eq!(flow.tick(FRESH_MS + 20), HudTick::Idle);

    // Once the first frame HAS drawn, a rapid second switch leaves `fresh` immediately so it
    // springs smoothly instead of snapping with `instant_changes()`.
    let mut rapid = HudFlow::new(true);
    rapid.trigger(row(2, Some(1)), 0);
    rapid.mark_drawn();
    rapid.trigger(row(3, Some(2)), 8);
    assert_eq!(focus(&rapid), (Some(id(3)), false));

    // With nowhere to come from, it lands at rest on the visible one.
    let mut flow = HudFlow::new(true);
    flow.trigger(row(2, None), 0);
    assert_eq!(focus(&flow), (Some(id(2)), true));
}

#[test]
fn a_workspace_the_row_no_longer_lists_stays_while_the_marker_leaves_it() {
    let mut flow = HudFlow::new(true);
    // Left an empty workspace 5, which drops out of the row once you're not on it.
    flow.trigger(row(2, Some(5)), 0);
    let numbers = |flow: &HudFlow| -> Vec<WorkspaceId> {
        flow.view(false)
            .unwrap()
            .entries()
            .iter()
            .map(|entry| entry.workspace)
            .collect()
    };
    assert_eq!(numbers(&flow), [id(1), id(2), id(3), id(5)]);
    flow.mark_drawn();
    flow.tick(FRESH_MS);
    assert_eq!(numbers(&flow), [id(1), id(2), id(3)]);
}

#[test]
fn disabling_hud_suppresses_triggers_and_dismisses_active_pill() {
    let mut flow = HudFlow::new(true);
    assert_eq!(flow.trigger(row(3, None), 500), HudTransition::Show);
    assert!(flow.set_enabled(false, 600));
    assert!(!shown(&flow));
    assert_eq!(flow.trigger(row(4, Some(3)), 650), HudTransition::Ignored);
    assert_eq!(flow.tick(600 + LEAVE_MS), HudTick::Gone);
    // Turning it off again while it's already hidden leaves nothing to dismiss.
    assert!(!flow.set_enabled(false, 2_000));
}

#[test]
fn the_layouts_row_is_as_wide_as_the_prototype_s() {
    for count in 1..=10_u8 {
        let entries: Vec<_> = (1..=count)
            .map(|number| workspace(number, &["a"]))
            .collect();
        let (_, width) = grow::rest(&entries, Some(id(1)));
        let n = f32::from(count);
        // `24 + w * (1 + (n - 1) * small) + gap * (n - 1)`, inside the capsule's border.
        let expected =
            24.0 + grow::SCREEN_W * (1.0 + (n - 1.0) * grow::SMALL) + grow::GAP * (n - 1.0);
        assert!(
            (width - expected).abs() < 0.01,
            "{count}: {width} vs {expected}"
        );
    }
}

#[test]
fn icons_show_each_app_once_in_tile_order_then_floating() {
    let mut entry = workspace(1, &["term", "web", "term", "notes"]);
    entry.floating = vec!["chat".to_owned(), "web".to_owned()];
    assert_eq!(icons::apps(&entry), ["term", "web", "notes", "chat"]);
    assert!(icons::apps(&workspace(2, &[])).is_empty());
}

#[test]
fn icons_marker_is_concentric_and_stays_circular_when_empty() {
    use super::pill::{INNER, Icons};
    use crate::controls::Paint;

    let entries = vec![workspace(1, &["term", "web"]), workspace(4, &[])];
    let paint = Paint::new(true, 2.0);

    let first = icons::track(&entries, Some(id(1)), &Icons::new(), paint);
    let [mx, my, _, mh] = first.marker.unwrap();
    assert_eq!(first.marker_radius, icons::MARKER_RADIUS);
    // Horizontal inset at the capsule end matches the vertical inset for a concentric pill.
    assert!((mx - my).abs() < 0.01, "x={mx} vs y={my}");
    assert!((my - (INNER - mh) / 2.0).abs() < 0.01);

    let empty = icons::track(&entries, Some(id(4)), &Icons::new(), paint);
    let [_, _, empty_w, empty_h] = empty.marker.unwrap();
    assert!(
        empty_w >= empty_h,
        "empty marker should be at least circular: {empty_w}x{empty_h}"
    );
}

#[test]
fn displays_keep_their_panels_as_others_come_and_go() {
    let some = |id: &str| Some(id.to_owned());
    let none = || None::<String>;
    let start = assign_panels(&[none(), none(), none(), none()], &["main", "side"]);
    assert_eq!(start, [some("main"), some("side"), none(), none()]);
    // The main display goes away: its panel is freed, the other keeps its own.
    let unplugged = assign_panels(&start, &["side"]);
    assert_eq!(unplugged, [none(), some("side"), none(), none()]);
    // A display plugged in takes the free panel, never one in use.
    let plugged = assign_panels(&unplugged, &["side", "new"]);
    assert_eq!(plugged, [some("new"), some("side"), none(), none()]);
    // More displays than panels: the extras go without.
    let crowded = assign_panels(&plugged, &["a", "b", "c", "d", "e"]);
    assert_eq!(crowded, [some("a"), some("b"), some("c"), some("d")]);
}

#[test]
fn hovering_holds_the_hud_open_and_resumes_dismiss_on_leave() {
    let mut flow = HudFlow::new(true);
    arrived(&mut flow, row(1, None), 0);

    assert!(flow.set_hovered(true, 400));
    assert!(flow.is_held());
    assert_eq!(flow.remaining_ms(400), None);
    // Past the original hold deadline: stays shown while hovered.
    assert_eq!(flow.tick(HOLD_MS * 3), HudTick::Idle);
    assert!(shown(&flow));

    // Pointer leaves: resumes auto-dismiss after RESUME_HOLD_MS.
    let leave_hover = HOLD_MS * 3;
    assert!(flow.set_hovered(false, leave_hover));
    assert!(!flow.is_held());
    assert_eq!(flow.remaining_ms(leave_hover), Some(RESUME_HOLD_MS));
    assert_eq!(flow.tick(leave_hover + RESUME_HOLD_MS - 1), HudTick::Idle);
    assert_eq!(flow.tick(leave_hover + RESUME_HOLD_MS), HudTick::Leave);

    // Catching the pill mid-exit with hover restores it cleanly.
    let caught = leave_hover + RESUME_HOLD_MS + 40;
    assert!(flow.set_hovered(true, caught));
    assert!(shown(&flow));
    assert_eq!(flow.tick(caught + LEAVE_MS + 100), HudTick::Idle);

    // An open context menu keeps the pill up even after the pointer leaves the capsule.
    flow.set_menu_open(true, caught + 200);
    flow.set_hovered(false, caught + 250);
    assert!(flow.is_held());
    assert_eq!(flow.tick(caught + HOLD_MS * 2), HudTick::Idle);
    let closed = caught + HOLD_MS * 2;
    flow.set_menu_open(false, closed);
    assert!(!flow.is_held());
    assert_eq!(flow.tick(closed + RESUME_HOLD_MS), HudTick::Leave);
    assert_eq!(flow.tick(closed + RESUME_HOLD_MS + LEAVE_MS), HudTick::Gone);
}

#[test]
fn visited_empty_workspaces_stay_in_the_row_until_hud_hides() {
    let mut flow = HudFlow::new(true);
    arrived(&mut flow, row(1, None), 0);
    let numbers = |flow: &HudFlow| -> Vec<WorkspaceId> {
        flow.view(false)
            .unwrap()
            .entries()
            .iter()
            .map(|entry| entry.workspace)
            .collect()
    };
    assert_eq!(numbers(&flow), [id(1), id(2), id(3)]);

    flow.trigger(row(4, Some(1)), 80);
    assert_eq!(numbers(&flow), [id(1), id(2), id(3), id(4)]);
    // Flick to another empty workspace (5): 4 stays in the row so width doesn't jitter.
    flow.trigger(row(5, Some(4)), 160);
    assert_eq!(numbers(&flow), [id(1), id(2), id(3), id(4), id(5)]);
    // Back to occupied workspace 2: 4 and 5 stay for the remainder of this show.
    flow.trigger(row(2, Some(5)), 240);
    assert_eq!(numbers(&flow), [id(1), id(2), id(3), id(4), id(5)]);

    // Once the HUD hides and opens again, empty workspaces are cleared.
    assert_eq!(flow.tick(240 + HOLD_MS), HudTick::Leave);
    assert_eq!(flow.tick(240 + HOLD_MS + LEAVE_MS), HudTick::Gone);
    arrived(&mut flow, row(2, None), 240 + HOLD_MS + LEAVE_MS + 50);
    assert_eq!(numbers(&flow), [id(1), id(2), id(3)]);
}

#[test]
fn visited_empty_workspaces_keep_the_configured_workspace_order() {
    let named = |letter| WorkspaceId::named(letter).unwrap();
    let keyboard_row = [id(1), named('Q'), named('W'), named('E')];
    let order: Vec<WorkspaceId> = keyboard_row
        .into_iter()
        .chain(WorkspaceId::all().filter(|workspace| !keyboard_row.contains(workspace)))
        .collect();
    // As the runtime sends it: occupied 1, then the visible (empty) one, in `order`.
    let row = |visible: char, from: Option<char>| UiHudDisplay {
        id: "main".to_owned(),
        visible: Some(named(visible)),
        from: from.map(named),
        workspaces: vec![
            workspace(1, &["a"]),
            UiHudWorkspace {
                workspace: named(visible),
                fullscreen: false,
                tiles: Vec::new(),
                floating: Vec::new(),
            },
        ],
        order: order.clone(),
    };
    let mut flow = HudFlow::new(true);
    arrived(&mut flow, row('Q', None), 0);

    // Flick across the empty Q, W and E: the ones left stay where `order` puts them.
    flow.trigger(row('W', Some('Q')), 80);
    flow.trigger(row('E', Some('W')), 160);
    let entries: Vec<_> = flow
        .view(false)
        .unwrap()
        .entries()
        .iter()
        .map(|entry| entry.workspace)
        .collect();
    assert_eq!(entries, keyboard_row);
}

#[test]
fn app_context_menu_reflects_float_tile_and_pin_state() {
    let ws2 = id(2);
    let ws4 = id(4);

    // Default (tiled, unpinned): offers Float and Pin to Workspace 2.
    let default_menu = menu::build_app_menu("Safari", ws2, None);
    assert_eq!(
        default_menu.find(menu::ID_TOGGLE_FLOAT).unwrap().title(),
        "Float Safari"
    );
    assert_eq!(
        default_menu.find(menu::ID_TOGGLE_PIN).unwrap().title(),
        "Pin Safari to Workspace 2"
    );
    assert_eq!(
        menu::resolve_app_menu_pick(menu::ID_TOGGLE_FLOAT, ws2, None),
        Some(Some(Place::Float))
    );
    assert_eq!(
        menu::resolve_app_menu_pick(menu::ID_TOGGLE_PIN, ws2, None),
        Some(Some(Place::Workspace(ws2)))
    );

    // Currently floated: offers Tile and Pin to Workspace 2.
    let float_menu = menu::build_app_menu("Safari", ws2, Some(Place::Float));
    assert_eq!(
        float_menu.find(menu::ID_TOGGLE_FLOAT).unwrap().title(),
        "Tile Safari"
    );
    assert_eq!(
        menu::resolve_app_menu_pick(menu::ID_TOGGLE_FLOAT, ws2, Some(Place::Float)),
        Some(None)
    );

    // Pinned to current workspace: offers Float and Unpin.
    let pinned_menu = menu::build_app_menu("Safari", ws2, Some(Place::Workspace(ws2)));
    assert_eq!(
        pinned_menu.find(menu::ID_TOGGLE_PIN).unwrap().title(),
        "Unpin Safari from Workspace 2"
    );
    assert_eq!(
        menu::resolve_app_menu_pick(menu::ID_TOGGLE_PIN, ws2, Some(Place::Workspace(ws2))),
        Some(None)
    );

    // Pinned to another workspace: offers Float, Pin here, and Unpin from the other workspace.
    let other_menu = menu::build_app_menu("Safari", ws2, Some(Place::Workspace(ws4)));
    assert_eq!(
        other_menu.find(menu::ID_TOGGLE_PIN).unwrap().title(),
        "Pin Safari to Workspace 2"
    );
    assert_eq!(
        other_menu.find(menu::ID_UNPIN_OTHER).unwrap().title(),
        "Unpin Safari from Workspace 4"
    );
    assert_eq!(
        menu::resolve_app_menu_pick(menu::ID_UNPIN_OTHER, ws2, Some(Place::Workspace(ws4))),
        Some(None)
    );
}
