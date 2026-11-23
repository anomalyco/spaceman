use super::super::commands::FOCUS_SETTLE;
use super::*;

/// Three tiles side by side on the main display, the first one focused.
fn three_columns() -> (WorkspaceRuntime<FakeBackend>, [crate::PhysicalWindowId; 3]) {
    let windows = [window(10), window(20), window(30)];
    let backend = FakeBackend {
        focused: Some(windows[0].clone()),
        ..FakeBackend::default()
    };
    let mut runtime = runtime(backend);
    for window in &windows {
        bind(&mut runtime, WorkspaceId::ONE, window);
    }
    (runtime, windows)
}

fn focus(runtime: &mut WorkspaceRuntime<FakeBackend>, direction: Direction) {
    runtime
        .focus_relative(direction, outputs(), &AtomicBool::new(false))
        .unwrap();
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

#[test]
fn quick_presses_step_on_while_accessibility_still_reports_the_old_window() {
    let (mut runtime, [first, second, _]) = three_columns();

    focus(&mut runtime, Direction::Right);
    // The target app hasn't activated yet; Accessibility still reports the
    // window focus left.
    runtime.backend.focused = Some(first.clone());
    focus(&mut runtime, Direction::Right);
    assert_eq!(focus_calls(&runtime), vec![20, 30]);

    // Accessibility catching up to the middle of the run is still stale.
    runtime.backend.focused = Some(second);
    focus(&mut runtime, Direction::Right);
    assert_eq!(focus_calls(&runtime), vec![20, 30]);
    focus(&mut runtime, Direction::Left);
    assert_eq!(focus_calls(&runtime), vec![20, 30, 20]);
}

#[test]
fn a_settled_or_different_focus_report_wins_over_the_pending_target() {
    let (mut runtime, [first, _, third]) = three_columns();

    focus(&mut runtime, Direction::Right);
    runtime.backend.focused = Some(first.clone());
    runtime.pending_focus.as_mut().unwrap().at -= FOCUS_SETTLE;
    focus(&mut runtime, Direction::Right);
    assert_eq!(focus_calls(&runtime), vec![20, 20]);

    // The user clicked the third tile: its report replaces the pending one.
    runtime.backend.focused = Some(third);
    focus(&mut runtime, Direction::Left);
    assert_eq!(focus_calls(&runtime), vec![20, 20, 20]);
}

#[test]
fn directional_keys_rescue_focus_from_a_floating_window() {
    let left = window(10);
    let right = window(20);
    let dialog = window(99);
    let mut backend = FakeBackend {
        focused: Some(dialog.clone()),
        ..FakeBackend::default()
    };
    backend.frames.insert(
        dialog.clone(),
        crate::Rect {
            x: 600,
            y: 200,
            width: 300,
            height: 200,
        },
    );
    let mut runtime = runtime(backend);
    bind(&mut runtime, WorkspaceId::ONE, &left);
    bind(&mut runtime, WorkspaceId::ONE, &right);
    runtime.workspaces.own_floating(dialog, WorkspaceId::ONE);

    focus(&mut runtime, Direction::Left);

    assert_eq!(runtime.backend.focused, Some(left));
}

#[test]
fn directional_keys_focus_a_tile_when_no_window_has_focus() {
    let (mut runtime, [first, ..]) = three_columns();
    runtime.backend.focused = None;

    focus(&mut runtime, Direction::Down);

    assert_eq!(runtime.backend.focused, Some(first));
}

#[test]
fn reordering_still_requires_a_tiled_window() {
    let (mut runtime, _) = three_columns();
    runtime.backend.focused = Some(window(99));

    let result = runtime.reorder_focused(Direction::Left, outputs(), &AtomicBool::new(false));

    assert!(result.is_err());
}

/// Two workspaces on the main display, the first one showing, and the second
/// waiting on a focus restore.
fn switch_to_second_workspace(focus_outcome: bool) -> WorkspaceRuntime<FakeBackend> {
    let shown = window(10);
    let incoming = window(20);
    let mut backend = FakeBackend {
        focused: Some(shown.clone()),
        focus_outcome: Some(focus_outcome),
        ..FakeBackend::default()
    };
    backend.frames.insert(shown.clone(), frame(50));
    backend.frames.insert(incoming.clone(), frame(500));
    let mut runtime = runtime(backend);
    let target = WorkspaceId::new(2).unwrap();
    bind(&mut runtime, WorkspaceId::ONE, &shown);
    bind(&mut runtime, target, &incoming);
    runtime.pending_fullscreen_focus_restore.insert(target);

    runtime
        .switch(target, outputs(), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(focus_calls(&runtime), vec![20]);
    runtime
}

#[test]
fn a_workspace_switch_that_cannot_focus_keeps_the_focus_restore_pending() {
    let runtime = switch_to_second_workspace(false);

    assert!(
        runtime
            .pending_fullscreen_focus_restore
            .contains(&WorkspaceId::new(2).unwrap())
    );
}

#[test]
fn a_workspace_switch_that_focuses_settles_the_focus_restore() {
    let runtime = switch_to_second_workspace(true);

    assert!(runtime.pending_fullscreen_focus_restore.is_empty());
}

#[test]
fn following_focus_to_a_hidden_workspace_keeps_the_window_that_took_it() {
    // Clicking a notification activates its window on a hidden workspace; the
    // switch must not hand focus to the tile that workspace last remembered,
    // whether the window is tiled, floating, or in a Fullscreen Layout.
    for (fullscreen, floating) in [(false, false), (true, false), (false, true)] {
        let app_window = |pid, window_id| crate::PhysicalWindowId {
            app: AppInstanceId {
                pid,
                launched_at: 1,
            },
            window_id,
        };
        let current = window(10);
        let remembered = app_window(2, 20);
        let notified = app_window(3, 30);
        let target = WorkspaceId::new(2).unwrap();
        let mut backend = FakeBackend {
            focused: Some(notified.clone()),
            frontmost: Some(notified.app.clone()),
            ..FakeBackend::default()
        };
        backend.frames.insert(current.clone(), frame(50));
        let parked = [
            (remembered.clone(), frame(50)),
            (notified.clone(), frame(500)),
        ];
        for (window, normal) in &parked {
            backend
                .frames
                .insert(window.clone(), main_parked_frame(*normal));
        }
        let mut runtime = runtime_with_parked(backend, parked);
        bind(&mut runtime, WorkspaceId::ONE, &current);
        bind(&mut runtime, target, &remembered);
        if floating {
            runtime.workspaces.own_floating(notified.clone(), target);
        } else {
            bind(&mut runtime, target, &notified);
        }
        runtime.workspaces.set_fullscreen_layout(target, fullscreen);
        runtime.workspaces.remember_focus(target, &remembered);

        let status = runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();

        let case = format!("fullscreen {fullscreen}, floating {floating}");
        assert_eq!(status.active, vec![target, WorkspaceId::SIX], "{case}");
        assert_eq!(runtime.backend.focused, Some(notified), "{case}");
        assert!(
            !focus_calls(&runtime).contains(&remembered.window_id),
            "{case}"
        );
    }
}

#[test]
fn a_follow_cancelled_by_the_activation_it_answers_follows_on_the_next_cycle() {
    // Accessibility reports the clicked notification's window before AppKit's
    // activation notice reaches the main thread, which then cancels the cycle
    // already switching there. The window must not stay suppressed and hidden.
    let current = window(10);
    let notified = crate::PhysicalWindowId {
        app: AppInstanceId {
            pid: 3,
            launched_at: 1,
        },
        window_id: 30,
    };
    let target = WorkspaceId::new(2).unwrap();
    let cancelled = Arc::new(AtomicBool::new(false));
    let mut backend = FakeBackend {
        focused: Some(notified.clone()),
        frontmost: Some(notified.app.clone()),
        cancel_on_position: Some(Arc::clone(&cancelled)),
        ..FakeBackend::default()
    };
    backend.frames.insert(current.clone(), frame(50));
    backend
        .frames
        .insert(notified.clone(), main_parked_frame(frame(500)));
    let mut runtime = runtime_with_parked(backend, [(notified.clone(), frame(500))]);
    bind(&mut runtime, WorkspaceId::ONE, &current);
    bind(&mut runtime, target, &notified);

    let error = runtime.cycle(outputs(), &cancelled).err().unwrap();
    assert!(error.ends_with("cancelled"), "{error}");
    // The cancelled switch rolled back: the old workspace stays up and the
    // window keeps its recovery frame.
    assert_eq!(
        runtime.workspaces.active_for(&DisplaySelector::Main),
        Some(WorkspaceId::ONE)
    );
    assert_eq!(runtime.parked.get(&notified), Some(&frame(500)));
    assert_eq!(runtime.suppressed_focus, None);

    let status = runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();
    assert_eq!(status.active, vec![target, WorkspaceId::SIX]);
    assert_eq!(runtime.backend.focused, Some(notified));
}

#[test]
fn a_follow_that_fails_while_an_activation_cancels_stays_suppressed() {
    // Only a cancellation retries; a switch that genuinely failed keeps the
    // window suppressed even if an activation also raised the cancel flag.
    let current = window(10);
    let notified = crate::PhysicalWindowId {
        app: AppInstanceId {
            pid: 3,
            launched_at: 1,
        },
        window_id: 30,
    };
    let target = WorkspaceId::new(2).unwrap();
    let cancelled = Arc::new(AtomicBool::new(false));
    let mut backend = FakeBackend {
        focused: Some(notified.clone()),
        frontmost: Some(notified.app.clone()),
        cancel_on_position: Some(Arc::clone(&cancelled)),
        position_outcome: Some(macos::ApplyOutcome::Unavailable),
        ..FakeBackend::default()
    };
    backend.frames.insert(current.clone(), frame(50));
    backend
        .frames
        .insert(notified.clone(), main_parked_frame(frame(500)));
    let mut runtime = runtime_with_parked(backend, [(notified.clone(), frame(500))]);
    bind(&mut runtime, WorkspaceId::ONE, &current);
    bind(&mut runtime, target, &notified);

    let error = runtime.cycle(outputs(), &cancelled).err().unwrap();

    assert!(!error.ends_with("cancelled"), "{error}");
    assert!(cancelled.load(Ordering::Acquire));
    assert_eq!(runtime.suppressed_focus, Some(notified));
}
