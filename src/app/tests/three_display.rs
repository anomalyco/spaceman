use super::*;

const LEFT: crate::Rect = crate::Rect {
    x: -1440,
    y: -583,
    width: 1440,
    height: 2530,
};
const CENTER: crate::Rect = crate::Rect {
    x: 0,
    y: 30,
    width: 2560,
    height: 1410,
};
const RIGHT: crate::Rect = crate::Rect {
    x: 2560,
    y: -539,
    width: 1440,
    height: 2530,
};

fn trio() -> macos::DisplayOutputs {
    let display = |id: &str, frame, is_main| macos::DisplayOutput {
        id: id.into(),
        name: id.into(),
        frame,
        is_main,
    };
    let full = |r: crate::Rect| crate::Rect {
        y: r.y - 30,
        height: r.height + 30,
        ..r
    };
    macos::DisplayOutputs::with_bounds(
        vec![
            display("center", CENTER, true),
            display("left", LEFT, false),
            display("right", RIGHT, false),
        ],
        vec![full(CENTER), full(LEFT), full(RIGHT)],
    )
    .unwrap()
}

fn ws(n: u8) -> WorkspaceId {
    WorkspaceId::new(n).unwrap()
}

fn win(id: u32) -> crate::PhysicalWindowId {
    crate::PhysicalWindowId {
        app: AppInstanceId {
            pid: id as i32,
            launched_at: 1,
        },
        window_id: id,
    }
}

fn seen(id: u32, frame: crate::Rect) -> crate::AppObservation {
    crate::AppObservation::Complete {
        app: crate::ObservedApplication {
            id: win(id).app,
            bundle_id: format!("com.example.A{id}"),
        },
        focused: Some(win(id)),
        windows: vec![crate::ObservedWindow {
            id: win(id),
            frame,
            resizable: true,
            title: None,
        }],
    }
}

fn rig(windows: &[(u32, u8, crate::Rect)]) -> WorkspaceRuntime<FakeBackend> {
    let mut text = "version = 1
[settings]
layout = 'columns'
"
    .to_owned();
    for (n, id) in [
        (6, "left"),
        (7, "left"),
        (8, "left"),
        (9, "right"),
        (10, "right"),
    ] {
        text.push_str(&format!(
            "[[workspaces]]
workspace = {n}
monitor = '{id}'
"
        ));
    }
    let backend = FakeBackend {
        focused: Some(win(windows[0].0)),
        frontmost: Some(win(windows[0].0).app),
        frames: windows
            .iter()
            .map(|(id, _, frame)| (win(*id), *frame))
            .collect(),
        observation: Some(crate::NativeObservation {
            cycle: 0,
            mouse_down: false,
            applications: windows
                .iter()
                .map(|(id, _, frame)| seen(*id, *frame))
                .collect(),
        }),
        ..FakeBackend::default()
    };
    let mut rt = runtime_with_config(backend, Config::parse(&text).unwrap());
    for (id, n, _) in windows {
        bind(&mut rt, ws(*n), &win(*id));
    }
    for n in [6, 9, 1] {
        rt.workspaces.switch(ws(n));
    }
    act(&mut rt, WorkerAction::Retile);
    rt
}

fn act(rt: &mut WorkspaceRuntime<FakeBackend>, action: WorkerAction) {
    let Some(WorkerResult::Status { result, .. }) = rt.execute(
        WorkerCommand::Action(action, trio()),
        &AtomicBool::new(false),
    ) else {
        panic!("expected status");
    };
    result.unwrap();
    rt.focus.age_request(super::super::focus::FOCUS_SETTLE);
}

fn focused(rt: &WorkspaceRuntime<FakeBackend>) -> Option<u32> {
    rt.backend.focused.as_ref().map(|window| window.window_id)
}

/// #355: horizontal focus from an empty workspace always moves in the pressed
/// direction to the nearest display, whether macOS still reports the parked
/// window (at the corner or held up by its title bar) or reports no focus.
#[test]
fn focus_from_an_empty_workspace_picks_the_nearest_display_in_that_direction() {
    let cases = [
        ("center", 10, 2, Direction::Right, 90),
        ("center", 10, 2, Direction::Left, 60),
        ("left", 60, 7, Direction::Right, 10),
        ("left", 60, 7, Direction::Left, 10),
        ("right", 90, 10, Direction::Left, 10),
        ("right", 90, 10, Direction::Right, 10),
    ];
    for (name, id, empty, direction, expected) in cases {
        let mut windows = vec![(10, 1, CENTER), (60, 6, LEFT), (90, 9, RIGHT)];
        windows.sort_by_key(|window| window.0 != id);
        for (held, report) in [(0, true), (32, true), (66, true), (0, false)] {
            let mut rt = rig(&windows);
            act(&mut rt, WorkerAction::SwitchWorkspace(ws(empty)));
            rt.backend.frames.get_mut(&win(id)).unwrap().y -= held;
            if !report {
                rt.backend.focused = None;
                rt.backend.frontmost = None;
            }
            act(&mut rt, WorkerAction::Focus(direction));
            assert_eq!(
                focused(&rt),
                Some(expected),
                "{name} empty, direction {direction}, held={held}, report={report}"
            );
        }
    }
}

/// #355: a floating window or dialog still rescues focus onto tiles cleanly,
/// including from near the top of a tall portrait side display.
#[test]
fn floating_window_on_tall_side_display_rescues_to_center_not_far_display() {
    let mut rt = rig(&[(10, 1, CENTER), (60, 6, LEFT), (90, 9, RIGHT)]);
    let dialog = win(77);
    let dialog_frame = crate::Rect {
        x: -1100,
        y: -500,
        width: 500,
        height: 300,
    };
    rt.backend.frames.insert(dialog.clone(), dialog_frame);
    rt.backend.focused = Some(dialog.clone());
    rt.backend.frontmost = Some(dialog.app);

    act(&mut rt, WorkerAction::Focus(Direction::Right));
    assert_eq!(focused(&rt), Some(10));
}

/// #356: pressing Option+L from the top or bottom row of a tall portrait side
/// display focuses the neighboring center display instead of skipping to right.
#[test]
fn focus_and_reorder_from_tall_side_display_never_skip_the_middle_display() {
    for count in 3..=6_u32 {
        for (id, label) in [(60, "top"), (60 + count - 1, "bottom")] {
            let mut windows: Vec<_> = (0..count).map(|row| (60 + row, 6, LEFT)).collect();
            windows.extend([(10, 1, CENTER), (90, 9, RIGHT)]);
            let mut rt = rig(&windows);
            rt.workspaces.engine_mut(ws(6)).toggle_axis();
            act(&mut rt, WorkerAction::Retile);
            rt.backend.focused = Some(win(id));
            rt.backend.frontmost = Some(win(id).app);

            act(&mut rt, WorkerAction::Focus(Direction::Right));
            assert_eq!(
                focused(&rt),
                Some(10),
                "{count} rows on left, {label} row -> right"
            );
            // A second Option+L continues onto the right display.
            act(&mut rt, WorkerAction::Focus(Direction::Right));
            assert_eq!(focused(&rt), Some(90), "{count} rows on left, second right");
        }
    }

    // Symmetrically from the right display's top/bottom rows pressing Left.
    let mut windows: Vec<_> = (0..5_u32).map(|row| (90 + row, 9, RIGHT)).collect();
    windows.extend([(10, 1, CENTER), (60, 6, LEFT)]);
    let mut rt = rig(&windows);
    rt.workspaces.engine_mut(ws(9)).toggle_axis();
    act(&mut rt, WorkerAction::Retile);
    for id in [90, 94] {
        rt.backend.focused = Some(win(id));
        rt.backend.frontmost = Some(win(id).app);
        act(&mut rt, WorkerAction::Focus(Direction::Left));
        assert_eq!(focused(&rt), Some(10), "right row {id} -> left");
    }
}

/// #358: dropping a dragged tile over another display's menu bar moves it to
/// that display's active workspace; dropping in its own display's menu bar
/// keeps it on its current workspace.
#[test]
fn drop_over_another_displays_menu_bar_moves_the_tile_there() {
    for (label, point, expected) in [
        ("left menu bar", (-720, -590), 6),
        ("left tiled area", (-720, -583), 6),
        ("right menu bar", (3200, -560), 9),
        ("right tiled area", (3200, -539), 9),
        ("own center menu bar", (1280, 10), 1),
    ] {
        let mut rt = rig(&[
            (10, 1, CENTER),
            (11, 1, CENTER),
            (60, 6, LEFT),
            (90, 9, RIGHT),
        ]);
        rt.drop_window(win(10), point, false, trio(), &AtomicBool::new(false))
            .unwrap();
        assert_eq!(
            rt.workspaces.workspace_of(&win(10)),
            Some(ws(expected)),
            "dropped at {point:?} ({label})"
        );
    }
}

/// #428: macOS fits the size write of a window moved from the center display
/// onto a taller portrait display to the bottom of the display it left. That
/// clamp is no refusal by the app, so the next cycle (the periodic retile, or
/// `ctl retile`) writes the full portrait frame again instead of freezing the
/// short one, for every way of moving the window there.
#[test]
fn tall_portrait_move_reaches_full_requested_height() {
    for (workspace, direction, portrait) in
        [(6, Direction::Left, LEFT), (9, Direction::Right, RIGHT)]
    {
        for action in [
            WorkerAction::MoveFocused(ws(workspace)),
            WorkerAction::MoveFocusedFollow(ws(workspace)),
            WorkerAction::Reorder(direction),
        ] {
            let mut rt = rig(&[(10, 1, CENTER)]);
            rt.backend.departed_display_clamp = vec![CENTER, LEFT, RIGHT];
            act(&mut rt, action.clone());
            assert_eq!(rt.workspaces.workspace_of(&win(10)), Some(ws(workspace)));
            let landed = rt.backend.frames[&win(10)];
            assert_eq!(
                (landed.x, landed.y, landed.y + landed.height),
                (portrait.x, portrait.y, CENTER.y + CENTER.height),
                "{action:?}: macOS clamps the move to the center display's bottom"
            );

            act(&mut rt, WorkerAction::Retile);
            assert_eq!(
                rt.backend.frames.get(&win(10)),
                Some(&portrait),
                "{action:?}"
            );
        }
    }
}

/// #428 control: a window that keeps a shorter frame on the display it is
/// already on (a maximum height) is still frozen after the bounded retries.
#[test]
fn a_window_short_of_its_tile_on_its_own_display_still_freezes() {
    let mut rt = rig(&[(10, 1, CENTER)]);
    let capped = crate::Rect {
        height: 900,
        ..CENTER
    };
    rt.backend.frames.insert(win(10), capped);
    rt.backend.departed_display_clamp = vec![CENTER, LEFT, RIGHT];
    rt.backend.managed_outcome = Some(macos::ApplyOutcome::Constrained);
    act(&mut rt, WorkerAction::Retile);
    assert_eq!(
        rt.constrained[ws(1).index()].get(&win(10)),
        Some(&(CENTER, capped))
    );
}
