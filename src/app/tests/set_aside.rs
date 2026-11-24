//! ⌘H and ⌘M: a hidden app's or minimized window's tile reflows at once
//! instead of waiting out the omission settle.
use super::reconciliation::{app_window, column, ordered_out_pair, studio_display};
use super::*;

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
