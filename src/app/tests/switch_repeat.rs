use super::super::commands::FOCUS_SETTLE;
use super::*;

/// Press a workspace shortcut or select a workspace from the menu bar / HUD,
/// through the worker action path the keyboard and the menu really use.
fn press(runtime: &mut WorkspaceRuntime<FakeBackend>, action: WorkerAction) -> CycleStatus {
    let cancelled = AtomicBool::new(false);
    let Some(WorkerResult::Status { result, .. }) =
        runtime.execute(WorkerCommand::Action(action, outputs()), &cancelled)
    else {
        panic!("an action reports status");
    };
    result.unwrap()
}

fn focus_calls(runtime: &WorkspaceRuntime<FakeBackend>) -> Vec<u32> {
    runtime
        .backend
        .calls
        .iter()
        .filter_map(|call| match call {
            BackendCall::Focus(window) => Some(window.window_id),
            _ => None,
        })
        .collect()
}

fn warp_calls(runtime: &WorkspaceRuntime<FakeBackend>) -> Vec<(i32, i32)> {
    runtime
        .backend
        .calls
        .iter()
        .filter_map(|call| match call {
            BackendCall::WarpPointer(point) => Some(*point),
            _ => None,
        })
        .collect()
}

/// Option+2 on Space 2 again walks the tiles, in tile order, wrapping at the
/// end: A -> B -> C -> A, in a split layout and in Fullscreen Layout alike.
#[test]
fn pressing_the_active_workspace_shortcut_again_cycles_its_tiles() {
    let two = WorkspaceId::new(2).unwrap();
    for fullscreen in [false, true] {
        let first = window(10);
        let second = window(20);
        let third = window(30);
        let mut backend = FakeBackend::default();
        for (index, window) in [&first, &second, &third].into_iter().enumerate() {
            backend
                .frames
                .insert(window.clone(), frame(50 + 450 * index as i32));
        }
        let mut runtime = runtime(backend);
        for window in [&first, &second, &third] {
            bind(&mut runtime, two, window);
        }
        runtime.workspaces.set_fullscreen_layout(two, fullscreen);

        // The first press switches to Space 2 and lands on its first tile.
        let status = press(&mut runtime, WorkerAction::SwitchWorkspace(two));
        assert_eq!(
            runtime.backend.focused,
            Some(first.clone()),
            "fullscreen={fullscreen}"
        );
        assert!(status.hud.is_some(), "fullscreen={fullscreen}");

        // The presses after that walk the tiles and wrap cleanly.
        for expected in [&second, &third, &first] {
            let status = press(&mut runtime, WorkerAction::SwitchWorkspace(two));
            assert_eq!(
                runtime.backend.focused,
                Some(expected.clone()),
                "fullscreen={fullscreen}"
            );
            assert_eq!(
                runtime.workspaces.preferred_focus(two),
                Some(expected.clone()),
                "fullscreen={fullscreen}"
            );
            assert!(status.hud.is_some(), "fullscreen={fullscreen}");
        }
    }
}

/// Two quick presses inside the focus settle window: the second must step
/// from the window the first focused, not from the one Accessibility still
/// reports.
#[test]
fn quick_repeat_presses_step_on_while_accessibility_still_reports_the_old_window() {
    let two = WorkspaceId::new(2).unwrap();
    let first = window(10);
    let second = window(20);
    let third = window(30);
    let mut runtime = runtime(FakeBackend::default());
    for window in [&first, &second, &third] {
        bind(&mut runtime, two, window);
    }
    press(&mut runtime, WorkerAction::SwitchWorkspace(two));
    assert_eq!(runtime.backend.focused, Some(first.clone()));
    runtime.backend.calls.clear();

    // The target app has not activated yet: Accessibility still reports the
    // window focus left behind.
    runtime.backend.focused = Some(first.clone());
    press(&mut runtime, WorkerAction::SwitchWorkspace(two));
    runtime.backend.focused = Some(first.clone());
    press(&mut runtime, WorkerAction::SwitchWorkspace(two));
    assert_eq!(
        focus_calls(&runtime),
        vec![second.window_id, third.window_id],
        "each press steps on from the window the last one focused"
    );

    // Once the settle window has passed, the reported focus wins again.
    runtime.backend.focused = Some(first);
    runtime.pending_focus.as_mut().unwrap().at -= FOCUS_SETTLE;
    press(&mut runtime, WorkerAction::SwitchWorkspace(two));
    assert_eq!(
        focus_calls(&runtime),
        vec![second.window_id, third.window_id, second.window_id]
    );
}

/// A workspace can be up on its display while focus sits on another display's
/// workspace. The first press brings focus back; only the press after that
/// cycles.
#[test]
fn a_repeat_press_from_another_display_focuses_the_workspace_then_cycles() {
    let two = WorkspaceId::new(2).unwrap();
    let first = window(10);
    let second = window(20);
    let elsewhere = window(30);
    let mut backend = FakeBackend {
        focused: Some(first.clone()),
        ..FakeBackend::default()
    };
    backend.frames.insert(first.clone(), frame(50));
    backend.frames.insert(second.clone(), frame(500));
    backend.frames.insert(elsewhere.clone(), frame(1050));
    let mut runtime = runtime(backend);
    bind(&mut runtime, two, &first);
    bind(&mut runtime, two, &second);
    bind(&mut runtime, WorkspaceId::SIX, &elsewhere);

    press(&mut runtime, WorkerAction::SwitchWorkspace(two));
    assert_eq!(runtime.backend.focused, Some(first.clone()));

    // Focus moved to the other display's workspace while Space 2 stayed up.
    runtime.backend.focused = Some(elsewhere.clone());
    press(&mut runtime, WorkerAction::SwitchWorkspace(two));
    assert_eq!(
        runtime.backend.focused,
        Some(first.clone()),
        "the press only brings focus back to the workspace"
    );

    press(&mut runtime, WorkerAction::SwitchWorkspace(two));
    assert_eq!(
        runtime.backend.focused,
        Some(second),
        "the press after that cycles"
    );
}

/// A menu-bar or HUD click on the workspace already up re-focuses it; clicking
/// it again never jumps to another window.
#[test]
fn selecting_the_active_workspace_again_keeps_its_focused_tile() {
    let two = WorkspaceId::new(2).unwrap();
    let first = window(10);
    let second = window(20);
    let mut backend = FakeBackend::default();
    backend.frames.insert(first.clone(), frame(50));
    backend.frames.insert(second.clone(), frame(500));
    let mut runtime = runtime(backend);
    bind(&mut runtime, two, &first);
    bind(&mut runtime, two, &second);

    let status = press(&mut runtime, WorkerAction::SelectWorkspace(two));
    assert_eq!(runtime.backend.focused, Some(first.clone()));
    assert!(status.hud.is_some());

    for _ in 0..3 {
        let status = press(&mut runtime, WorkerAction::SelectWorkspace(two));
        assert_eq!(runtime.backend.focused, Some(first.clone()));
        assert!(status.hud.is_some());
    }
}

/// A workspace with a single tile has nowhere to cycle to; a repeat press
/// keeps that window cleanly focused.
#[test]
fn a_single_window_workspace_stays_put_on_a_repeat_press() {
    let two = WorkspaceId::new(2).unwrap();
    let only = window(10);
    let mut backend = FakeBackend::default();
    backend.frames.insert(only.clone(), frame(50));
    let mut runtime = runtime(backend);
    bind(&mut runtime, two, &only);

    for _ in 0..3 {
        let status = press(&mut runtime, WorkerAction::SwitchWorkspace(two));
        assert_eq!(runtime.backend.focused, Some(only.clone()));
        assert!(status.hud.is_some());
    }
}

/// A floating window of the workspace can hold focus but has no place in the
/// tile order, so a repeat press steps onto the remembered tile.
#[test]
fn a_repeat_press_from_a_float_lands_on_the_remembered_tile() {
    let two = WorkspaceId::new(2).unwrap();
    let first = window(10);
    let second = window(20);
    let floating = window(30);
    let mut backend = FakeBackend::default();
    backend.frames.insert(first.clone(), frame(50));
    backend.frames.insert(second.clone(), frame(500));
    backend.frames.insert(floating.clone(), frame(600));
    let mut runtime = runtime(backend);
    bind(&mut runtime, two, &first);
    bind(&mut runtime, two, &second);
    runtime.workspaces.own_floating(floating.clone(), two);

    press(&mut runtime, WorkerAction::SwitchWorkspace(two));
    assert_eq!(runtime.backend.focused, Some(first.clone()));

    runtime.workspaces.remember_focus(two, &second);
    runtime.backend.focused = Some(floating);
    press(&mut runtime, WorkerAction::SwitchWorkspace(two));
    assert_eq!(runtime.backend.focused, Some(second));
}

/// Cycling is an explicit keyboard focus move, so it warps the pointer onto the
/// window it lands on when mouse_follows_focus is on.
#[test]
fn cycling_warps_the_pointer_onto_the_next_tile() {
    let two = WorkspaceId::new(2).unwrap();
    let first = window(10);
    let second = window(20);
    let mut backend = FakeBackend {
        pointer: Some((-2000, -2000)),
        ..FakeBackend::default()
    };
    backend.frames.insert(first.clone(), frame(50));
    backend.frames.insert(second.clone(), frame(500));
    let mut runtime = WorkspaceRuntime::with_backend_and_config(
        ManagementScope::Both,
        backend,
        HashMap::new(),
        Config::parse("version = 1\n[settings]\nmouse_follows_focus = \"window\"\n").unwrap(),
    );
    bind(&mut runtime, two, &first);
    bind(&mut runtime, two, &second);

    press(&mut runtime, WorkerAction::SwitchWorkspace(two));
    assert_eq!(runtime.backend.focused, Some(first.clone()));
    press(&mut runtime, WorkerAction::SwitchWorkspace(two));
    assert_eq!(runtime.backend.focused, Some(second.clone()));

    let warps = warp_calls(&runtime);
    assert_eq!(warps.len(), 2, "one warp per focused tile: {warps:?}");
    for (point, window) in [(warps[0], &first), (warps[1], &second)] {
        let landed = runtime.backend.frames[window];
        assert!(
            landed.contains(point.0, point.1),
            "pointer warped to {point:?}, outside the tile {landed:?}"
        );
    }
}

#[test]
fn quick_repeat_after_returning_from_another_display_advances_during_activation_lag() {
    for fullscreen in [false, true] {
        let two = WorkspaceId::TWO;
        let first = window(10);
        let second = window(20);
        let mut elsewhere = window(30);
        elsewhere.app.pid = 2;
        let mut backend = FakeBackend::default();
        backend.frames.insert(first.clone(), frame(50));
        backend.frames.insert(second.clone(), frame(500));
        backend.frames.insert(elsewhere.clone(), frame(1050));
        let mut runtime = runtime(backend);
        bind(&mut runtime, two, &first);
        bind(&mut runtime, two, &second);
        bind(&mut runtime, WorkspaceId::SIX, &elsewhere);
        runtime.workspaces.set_fullscreen_layout(two, fullscreen);
        press(&mut runtime, WorkerAction::SwitchWorkspace(two));

        runtime.backend.focused = Some(elsewhere.clone());
        runtime.backend.frontmost = Some(elsewhere.app.clone());
        runtime.backend.calls.clear();

        // First press returns focus to the entry tile (10), then AX still reports the old app.
        press(&mut runtime, WorkerAction::SwitchWorkspace(two));
        assert_eq!(focus_calls(&runtime), vec![10]);
        runtime.backend.focused = Some(elsewhere.clone());
        runtime.backend.frontmost = Some(elsewhere.app.clone());

        // Second press inside the settle window advances directly to tile 20.
        press(&mut runtime, WorkerAction::SwitchWorkspace(two));
        assert_eq!(focus_calls(&runtime), vec![10, 20]);
        assert_eq!(runtime.workspaces.preferred_focus(two), Some(second));
    }
}

#[test]
fn stale_focus_report_does_not_lose_selected_tile_on_switch_move_or_cycle() {
    let setup = |fullscreen: bool| {
        let first = window(10);
        let second = window(20);
        let other = window(30);
        let backend = FakeBackend {
            focused: Some(first.clone()),
            frontmost: Some(first.app.clone()),
            frames: HashMap::from([
                (first.clone(), frame(50)),
                (second.clone(), frame(500)),
                (other.clone(), main_parked_frame(frame(200))),
            ]),
            ..FakeBackend::default()
        };
        let mut rt = runtime_with_parked(backend, [(other.clone(), frame(200))]);
        bind(&mut rt, WorkspaceId::ONE, &first);
        bind(&mut rt, WorkspaceId::ONE, &second);
        bind(&mut rt, WorkspaceId::TWO, &other);
        if fullscreen {
            press(&mut rt, WorkerAction::ToggleFullscreenLayout);
        }
        press(&mut rt, WorkerAction::Focus(Direction::Right));
        assert_eq!(rt.backend.focused, Some(second));
        assert!(rt.pending_focus.is_some());
        rt
    };

    // 1. Switch away and back during stale focus report (both split and Fullscreen Layout).
    for fullscreen in [false, true] {
        let mut rt = setup(fullscreen);
        rt.backend.focused = Some(window(10));
        press(&mut rt, WorkerAction::SwitchWorkspace(WorkspaceId::TWO));
        assert_eq!(rt.backend.focused, Some(window(30)));
        press(&mut rt, WorkerAction::SwitchWorkspace(WorkspaceId::ONE));
        assert_eq!(
            rt.workspaces.preferred_focus(WorkspaceId::ONE),
            Some(window(20))
        );
        assert_eq!(rt.backend.focused, Some(window(20)));
    }

    // 2. Move workspace to next display during stale focus report.
    let mut rt = setup(false);
    rt.backend.focused = Some(window(10));
    press(
        &mut rt,
        WorkerAction::MoveWorkspaceToDisplay(DisplayTarget::Next),
    );
    assert_eq!(
        rt.workspaces.preferred_focus(WorkspaceId::ONE),
        Some(window(20))
    );

    // 3. Observation cycle in Fullscreen Layout during stale focus report.
    let mut rt = setup(true);
    let reported = window(10);
    rt.backend.focused = Some(reported.clone());
    rt.backend.frontmost = Some(reported.app.clone());
    rt.backend.observation = Some(observation_for_windows(
        "com.example.Test",
        &reported,
        &[(window(10), frame(50)), (window(20), frame(500))],
    ));
    press(&mut rt, WorkerAction::Retile);
    assert_eq!(
        rt.workspaces.preferred_focus(WorkspaceId::ONE),
        Some(window(20))
    );
}

#[test]
fn cross_display_focus_then_move_workspace_targets_pending_workspace_during_lag() {
    for stale in [true, false] {
        let first = window(10);
        let mut second = window(30);
        second.app.pid = 2;
        let backend = FakeBackend {
            focused: Some(first.clone()),
            frontmost: Some(first.app.clone()),
            frames: HashMap::from([(first.clone(), frame(50)), (second.clone(), frame(1050))]),
            ..FakeBackend::default()
        };
        let mut rt = runtime(backend);
        bind(&mut rt, WorkspaceId::ONE, &first);
        bind(&mut rt, WorkspaceId::SIX, &second);
        press(&mut rt, WorkerAction::Focus(Direction::Right));
        assert_eq!(rt.backend.focused, Some(second.clone()));
        assert_eq!(rt.pending_focus.as_ref().unwrap().target, second);

        if stale {
            rt.backend.focused = Some(first.clone());
            rt.backend.frontmost = Some(first.app.clone());
        }
        press(
            &mut rt,
            WorkerAction::MoveWorkspaceToDisplay(DisplayTarget::Next),
        );
        assert_eq!(
            rt.backend.persisted_monitors.first().map(|entry| entry.0),
            Some(WorkspaceId::SIX)
        );
        assert_eq!(
            rt.workspaces.active_for(&DisplaySelector::Main),
            Some(WorkspaceId::SIX)
        );
        assert_eq!(
            rt.workspaces.active_for(&DisplaySelector::Secondary),
            Some(WorkspaceId::new(7).unwrap())
        );
        assert!(!rt.parked.contains_key(&second));
        if stale {
            assert_eq!(rt.suppressed_focus, Some(first));
            assert_eq!(
                rt.pending_focus.as_ref().map(|p| p.target.clone()),
                Some(second)
            );
        }
    }
}
