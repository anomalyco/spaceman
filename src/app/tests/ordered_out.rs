//! A bound window the window server still lists but has ordered out: a
//! closed window its app keeps alive, or one set aside with ⌘H or ⌘M. A
//! confirmed close or a set-aside window reflows its sibling at once; anything
//! uncertain waits out the omission settle.
use super::reconciliation::{app_window, apps_observation, column, studio_display};
use super::*;

/// Two apps share a display. The second app disappears from the observation
/// while Core Graphics still lists its former surface off screen.
fn ordered_out_pair(
    bundle: &str,
) -> (
    WorkspaceRuntime<FakeBackend>,
    crate::PhysicalWindowId,
    crate::PhysicalWindowId,
) {
    let kept = app_window(10, 1, 100);
    let missing = app_window(20, 1, 200);
    let left = column(0, 1280);
    let right = column(1280, 1280);
    let mut runtime = runtime(FakeBackend {
        frames: [(kept.clone(), left), (missing.clone(), right)].into(),
        observation: Some(apps_observation(&[
            ("com.example.Kept", &kept, left),
            (bundle, &missing, right),
        ])),
        ..FakeBackend::default()
    });
    bind(&mut runtime, WorkspaceId::ONE, &kept);
    bind(&mut runtime, WorkspaceId::ONE, &missing);
    runtime
        .cycle(studio_display(), &AtomicBool::new(false))
        .unwrap();

    runtime.backend.observation = Some(apps_observation(&[("com.example.Kept", &kept, left)]));
    runtime.backend.frames.remove(&missing);
    runtime.backend.server_frames.insert(missing.clone(), right);
    runtime.backend.ordered_out.insert(missing.window_id);
    (runtime, kept, missing)
}

#[test]
fn confirmed_last_window_close_reflows_the_sibling_in_the_same_cycle() {
    let (mut runtime, kept, missing) = ordered_out_pair("com.example.Closing");
    runtime
        .backend
        .apps_confirming_no_windows
        .insert(missing.app.clone());
    let status = runtime
        .cycle(studio_display(), &AtomicBool::new(false))
        .unwrap();

    assert_eq!(
        runtime.backend.close_probes,
        std::slice::from_ref(&missing.app)
    );
    assert!(status.close_recheck.is_none());
    assert_eq!(runtime.workspaces.workspace_of(&missing), None);
    assert_eq!(runtime.backend.frames[&kept], column(0, 2560));
}

#[test]
fn uncertain_close_requests_one_early_retry() {
    for confirmed_on_retry in [false, true] {
        let (mut runtime, kept, missing) = ordered_out_pair("com.example.Closing");
        let status = runtime
            .cycle(studio_display(), &AtomicBool::new(false))
            .unwrap();
        let deadline = status.close_recheck.expect("one prompt follow-up");
        assert!(deadline <= Instant::now() + Duration::from_millis(75));
        assert_eq!(runtime.backend.frames[&kept], column(0, 1280));

        if confirmed_on_retry {
            runtime
                .backend
                .apps_confirming_no_windows
                .insert(missing.app.clone());
        }
        std::thread::sleep(deadline.saturating_duration_since(Instant::now()));
        let status = runtime
            .cycle(studio_display(), &AtomicBool::new(false))
            .unwrap();
        assert!(status.close_recheck.is_none());
        assert_eq!(
            runtime.backend.frames[&kept].width,
            if confirmed_on_retry { 2560 } else { 1280 }
        );
        assert_eq!(
            runtime.workspaces.workspace_of(&missing).is_none(),
            confirmed_on_retry
        );
        runtime
            .cycle(studio_display(), &AtomicBool::new(false))
            .unwrap();
        assert_eq!(
            runtime.backend.close_probes,
            [missing.app.clone(), missing.app]
        );
    }
}

#[test]
fn recheck_reflows_when_the_ordered_out_surface_disappears() {
    let (mut runtime, kept, missing) = ordered_out_pair("com.example.Closing");
    let cancelled = AtomicBool::new(false);
    let first = runtime.cycle(studio_display(), &cancelled).unwrap();
    assert!(first.close_recheck.is_some());
    assert_eq!(runtime.backend.frames[&kept], column(0, 1280));

    runtime.backend.server_frames.remove(&missing);
    let next = runtime.cycle(studio_display(), &cancelled).unwrap();
    assert!(next.close_recheck.is_none());
    assert_eq!(runtime.workspaces.workspace_of(&missing), None);
    assert_eq!(runtime.backend.frames[&kept], column(0, 2560));
    assert_eq!(runtime.backend.close_probes.len(), 1);
}

#[test]
fn stale_ax_sibling_during_space_switch_does_not_confirm_a_close() {
    let (mut runtime, kept, missing) = ordered_out_pair("com.example.Closing");
    runtime.backend.ordered_out.insert(kept.window_id);
    runtime
        .backend
        .apps_confirming_no_windows
        .insert(missing.app.clone());

    let status = runtime
        .cycle(studio_display(), &AtomicBool::new(false))
        .unwrap();

    assert!(runtime.backend.close_probes.is_empty());
    assert!(status.close_recheck.is_none());
    assert_eq!(
        runtime.workspaces.workspace_of(&missing),
        Some(WorkspaceId::ONE)
    );
}

#[test]
fn multiple_missing_slots_of_one_app_keep_the_ordinary_grace_period() {
    let (mut runtime, _, missing) = ordered_out_pair("com.example.Closing");
    let another = app_window(missing.app.pid, missing.app.launched_at, 201);
    bind(&mut runtime, WorkspaceId::ONE, &another);
    runtime
        .backend
        .apps_confirming_no_windows
        .insert(missing.app.clone());
    let status = runtime
        .cycle(studio_display(), &AtomicBool::new(false))
        .unwrap();

    assert!(runtime.backend.close_probes.is_empty());
    assert!(status.close_recheck.is_none());
    for window in [missing, another] {
        assert_eq!(
            runtime.workspaces.workspace_of(&window),
            Some(WorkspaceId::ONE)
        );
    }
}

#[test]
fn a_possible_tab_replacement_or_incomplete_snapshot_keeps_its_tile() {
    for bundle in ["com.mitchellh.ghostty", "com.example.Busy"] {
        let (mut runtime, kept, missing) = ordered_out_pair(bundle);
        if bundle == "com.example.Busy" {
            runtime
                .backend
                .observation
                .as_mut()
                .unwrap()
                .applications
                .push(crate::AppObservation::Unavailable {
                    app: missing.app.clone(),
                    error: "timeout".into(),
                });
        }
        runtime
            .backend
            .apps_confirming_no_windows
            .insert(missing.app.clone());
        let status = runtime
            .cycle(studio_display(), &AtomicBool::new(false))
            .unwrap();
        assert!(runtime.backend.close_probes.is_empty());
        assert!(status.close_recheck.is_none());
        assert_eq!(
            runtime.workspaces.workspace_of(&missing),
            Some(WorkspaceId::ONE)
        );
        assert_eq!(runtime.backend.frames[&kept], column(0, 1280));
    }
}

#[test]
fn hidden_or_minimized_window_reflows_the_sibling_in_the_same_cycle() {
    let (mut runtime, kept, missing) = ordered_out_pair("com.example.Hidden");
    runtime.backend.set_aside.insert(missing.clone());

    let status = runtime
        .cycle(studio_display(), &AtomicBool::new(false))
        .unwrap();

    assert!(status.close_recheck.is_none());
    assert!(runtime.backend.close_probes.is_empty());
    assert_eq!(runtime.workspaces.workspace_of(&missing), None);
    assert_eq!(runtime.backend.frames[&kept], column(0, 2560));
}

#[test]
fn an_app_keeps_its_tiles_while_one_of_its_windows_is_still_settling() {
    let (mut runtime, kept, missing) = ordered_out_pair("com.example.Minimized");
    let settling = app_window(missing.app.pid, missing.app.launched_at, 201);
    bind(&mut runtime, WorkspaceId::ONE, &settling);
    let right = runtime.backend.server_frames[&missing];
    runtime
        .backend
        .server_frames
        .insert(settling.clone(), right);
    runtime.backend.ordered_out.insert(settling.window_id);
    runtime.backend.set_aside.insert(missing.clone());

    runtime
        .cycle(studio_display(), &AtomicBool::new(false))
        .unwrap();

    for window in [&missing, &settling] {
        assert_eq!(
            runtime.workspaces.workspace_of(window),
            Some(WorkspaceId::ONE)
        );
    }
    assert_ne!(runtime.backend.frames[&kept], column(0, 2560));
}

#[test]
fn a_parked_window_keeps_its_tile_through_the_settle() {
    let (mut runtime, kept, missing) = ordered_out_pair("com.example.Parked");
    let right = runtime.backend.server_frames[&missing];
    runtime.parked.insert(missing.clone(), right);
    runtime
        .recently_parked
        .insert(missing.clone(), Instant::now());
    runtime.backend.set_aside.insert(missing.clone());

    runtime
        .cycle(studio_display(), &AtomicBool::new(false))
        .unwrap();

    assert_eq!(
        runtime.workspaces.workspace_of(&missing),
        Some(WorkspaceId::ONE)
    );
    assert_eq!(runtime.backend.frames[&kept], column(0, 1280));
}
