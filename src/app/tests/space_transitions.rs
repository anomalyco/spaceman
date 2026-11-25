//! macOS Space slides: a parked window on another desktop keeps its recovery
//! frame (#328), Cmd+Tab out of a native fullscreen app lands on the target
//! workspace (#329), and the outgoing desktop's tile keeps its size while the
//! slide runs (#334).

use super::*;

fn app_window(pid: i32, window_id: u32) -> crate::PhysicalWindowId {
    crate::PhysicalWindowId {
        app: AppInstanceId {
            pid,
            launched_at: 1,
        },
        window_id,
    }
}

/// One complete application per window; `focused` is its app's focus.
fn observe_apps(
    focused: Option<&crate::PhysicalWindowId>,
    windows: &[(crate::PhysicalWindowId, crate::Rect)],
) -> crate::NativeObservation {
    crate::NativeObservation {
        cycle: 0,
        mouse_down: false,
        applications: windows
            .iter()
            .map(|(window, frame)| crate::AppObservation::Complete {
                app: crate::ObservedApplication {
                    id: window.app.clone(),
                    bundle_id: format!("com.example.App{}", window.app.pid),
                },
                focused: (focused == Some(window)).then(|| window.clone()),
                windows: vec![crate::ObservedWindow {
                    id: window.clone(),
                    frame: *frame,
                    resizable: true,
                    title: None,
                }],
            })
            .collect(),
    }
}

/// A Space change long enough ago that its slide has settled.
fn settled_space_change() -> Option<Instant> {
    Instant::now().checked_sub(Duration::from_secs(5))
}

fn main_workspace(runtime: &WorkspaceRuntime<FakeBackend>) -> Option<WorkspaceId> {
    runtime.workspaces.active_for(&DisplaySelector::Main)
}

fn applied_to(runtime: &WorkspaceRuntime<FakeBackend>, window: &crate::PhysicalWindowId) -> bool {
    runtime.backend.calls.iter().any(|call| {
        matches!(call, BackendCall::Apply(target, _) | BackendCall::SetPosition(target, _)
            if target == window)
    })
}

/// #328: Accessibility cannot reach a parked window on another macOS desktop
/// while the window server still lists it. Pause and quit keep its recovery
/// entry however long that lasts, and restore it once its desktop is shown;
/// a window that really closed still lets them finish.
#[test]
fn restore_keeps_recovery_for_a_parked_window_on_another_desktop() {
    let elsewhere = window(10);
    let closed = window(11);
    let normal = frame(50);
    let parked_at = main_parked_frame(normal);
    let mut backend = FakeBackend::default();
    // Listed by the window server, unreachable through Accessibility.
    backend.server_frames.insert(elsewhere.clone(), parked_at);
    let mut runtime = runtime_with_parked(
        backend,
        [(elsewhere.clone(), normal), (closed.clone(), frame(100))],
    );
    let two = WorkspaceId::new(2).unwrap();
    bind(&mut runtime, two, &elsewhere);
    bind(&mut runtime, two, &closed);

    assert_eq!(runtime.restore_all(), 1);
    assert_eq!(runtime.parked.get(&elsewhere), Some(&normal));
    assert!(!runtime.parked.contains_key(&closed));

    // Long past the Accessibility grace that retires unparked slots.
    if let Some(since) = runtime.inaccessible_since.get_mut(&elsewhere) {
        *since -= crate::app::parking::INACCESSIBLE_GRACE * 2;
    }
    assert_eq!(runtime.restore_all(), 1);
    assert_eq!(runtime.parked.get(&elsewhere), Some(&normal));
    assert_eq!(runtime.backend.persisted.get(&elsewhere), Some(&normal));

    // The user shows that desktop: Accessibility reaches the window again.
    runtime.backend.frames.insert(elsewhere.clone(), parked_at);
    assert_eq!(runtime.restore_all(), 0);
    assert!(runtime.parked.is_empty());
    assert_eq!(runtime.backend.frames[&elsewhere].x, normal.x);
    assert!(!runtime.inaccessible_since.contains_key(&elsewhere));
}

/// #334: activating a window on another desktop slides it in while the
/// outgoing desktop's tile is still on screen. Accessibility already reports
/// the arriving window, but laying both out together would shrink the
/// outgoing tile mid-slide. Layout waits for the slide to settle.
#[test]
fn a_space_slide_holds_layout_until_it_settles() {
    let outgoing = app_window(1, 10);
    let arriving = app_window(2, 20);
    let mut backend = FakeBackend {
        focused: Some(outgoing.clone()),
        frontmost: Some(outgoing.app.clone()),
        observation: Some(observe_apps(
            Some(&outgoing),
            &[(outgoing.clone(), frame(50))],
        )),
        ..FakeBackend::default()
    };
    backend.frames.insert(outgoing.clone(), frame(50));
    let mut runtime = runtime(backend);
    let cancelled = AtomicBool::new(false);
    runtime.cycle(outputs(), &cancelled).unwrap();
    assert_eq!(
        runtime.workspaces.workspace_of(&outgoing),
        Some(WorkspaceId::ONE)
    );
    let tiled = runtime.backend.frames[&outgoing];

    // The slide begins: the arriving window is reported alongside it.
    runtime.backend.space_changed_at = Some(Instant::now());
    runtime.backend.focused = Some(arriving.clone());
    runtime.backend.frontmost = Some(arriving.app.clone());
    runtime.backend.observation = Some(observe_apps(
        Some(&arriving),
        &[(outgoing.clone(), tiled), (arriving.clone(), frame(300))],
    ));
    runtime.backend.frames.insert(arriving.clone(), frame(300));
    runtime.backend.calls.clear();
    let held = runtime.cycle(outputs(), &cancelled).unwrap();

    assert!(held.close_recheck.is_some(), "no retile after the slide");
    assert!(
        !applied_to(&runtime, &outgoing),
        "outgoing tile resized mid-slide"
    );
    assert_eq!(runtime.backend.frames[&outgoing], tiled);
    assert_eq!(runtime.workspaces.workspace_of(&arriving), None);

    // Once it settles, the arriving window is adopted and tiled as before.
    runtime.backend.space_changed_at = settled_space_change();
    runtime.cycle(outputs(), &cancelled).unwrap();
    assert_eq!(
        runtime.workspaces.workspace_of(&arriving),
        Some(WorkspaceId::ONE)
    );
    assert!(applied_to(&runtime, &arriving));
}

/// A tiled Terminal on workspace 1 and TextEdit hidden on workspace 2.
fn fullscreen_cmd_tab_runtime() -> (
    WorkspaceRuntime<FakeBackend>,
    crate::PhysicalWindowId,
    crate::PhysicalWindowId,
) {
    let terminal = app_window(1, 10);
    let text_edit = app_window(2, 20);
    let normal = frame(100);
    let mut backend = FakeBackend {
        focused: Some(terminal.clone()),
        frontmost: Some(terminal.app.clone()),
        observation: Some(observe_apps(
            Some(&terminal),
            &[(terminal.clone(), frame(50)), (text_edit.clone(), normal)],
        )),
        ..FakeBackend::default()
    };
    backend.frames.insert(terminal.clone(), frame(50));
    backend
        .frames
        .insert(text_edit.clone(), main_parked_frame(normal));
    let mut runtime = runtime_with_parked(backend, [(text_edit.clone(), normal)]);
    bind(&mut runtime, WorkspaceId::ONE, &terminal);
    bind(&mut runtime, WorkspaceId::new(2).unwrap(), &text_edit);
    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();
    (runtime, terminal, text_edit)
}

/// #329: Cmd+Tab from a native fullscreen app to a window on a hidden
/// workspace slides macOS back to the desktop. Following focus mid-slide
/// could not lay the workspace out, rolled back, and heal then re-homed the
/// window onto the workspace on screen. Focus follow now waits for the slide.
#[test]
fn cmd_tab_out_of_native_fullscreen_lands_on_the_target_workspace() {
    let (mut runtime, _, text_edit) = fullscreen_cmd_tab_runtime();
    let two = WorkspaceId::new(2).unwrap();
    let cancelled = AtomicBool::new(false);

    runtime.backend.space_changed_at = Some(Instant::now());
    // Mid-slide, writes to the incoming desktop's windows fail.
    runtime.backend.position_outcome = Some(macos::ApplyOutcome::Unavailable);
    runtime.backend.focused = Some(text_edit.clone());
    runtime.backend.frontmost = Some(text_edit.app.clone());
    runtime.cycle(outputs(), &cancelled).unwrap();
    assert_eq!(main_workspace(&runtime), Some(WorkspaceId::ONE));
    assert_eq!(runtime.workspaces.workspace_of(&text_edit), Some(two));
    assert!(runtime.parked.contains_key(&text_edit));

    runtime.backend.space_changed_at = settled_space_change();
    runtime.backend.position_outcome = None;
    runtime.cycle(outputs(), &cancelled).unwrap();
    assert_eq!(main_workspace(&runtime), Some(two));
    assert_eq!(runtime.workspaces.workspace_of(&text_edit), Some(two));
    assert!(!runtime.parked.contains_key(&text_edit));
}

/// #329: when the slide's notification arrives only while the follow is
/// already restoring the window, the failed follow is retried once the slide
/// settles instead of staying suppressed (which left the window to heal).
#[test]
fn a_focus_follow_a_space_slide_interrupts_is_retried() {
    let (mut runtime, _, text_edit) = fullscreen_cmd_tab_runtime();
    let two = WorkspaceId::new(2).unwrap();
    let cancelled = AtomicBool::new(false);

    runtime.backend.space_change_on_position = true;
    runtime.backend.position_outcome = Some(macos::ApplyOutcome::Unavailable);
    runtime.backend.focused = Some(text_edit.clone());
    runtime.backend.frontmost = Some(text_edit.app.clone());
    let held = runtime.cycle(outputs(), &cancelled).unwrap();
    assert!(held.close_recheck.is_some(), "no retile after the slide");
    assert_eq!(runtime.workspaces.workspace_of(&text_edit), Some(two));
    assert!(runtime.parked.contains_key(&text_edit));
    assert_eq!(runtime.suppressed_focus, None);

    runtime.backend.space_changed_at = settled_space_change();
    runtime.backend.position_outcome = None;
    runtime.cycle(outputs(), &cancelled).unwrap();
    assert_eq!(main_workspace(&runtime), Some(two));
    assert_eq!(runtime.workspaces.workspace_of(&text_edit), Some(two));
}

/// Control for #329: with no Space change, a follow that fails keeps focus
/// suppressed, so it cannot loop.
#[test]
fn a_failed_focus_follow_without_a_space_change_stays_suppressed() {
    let (mut runtime, _, text_edit) = fullscreen_cmd_tab_runtime();
    let cancelled = AtomicBool::new(false);

    runtime.backend.position_outcome = Some(macos::ApplyOutcome::Unavailable);
    runtime.backend.focused = Some(text_edit.clone());
    runtime.backend.frontmost = Some(text_edit.app.clone());
    assert!(runtime.cycle(outputs(), &cancelled).is_err());
    assert_eq!(runtime.suppressed_focus, Some(text_edit));
}
