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
