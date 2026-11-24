//! #201: after switching to an empty workspace on a display other than main,
//! moving the focused workspace moves that empty workspace, not main's.
use super::*;

fn moved_first(occupied: u8, empty: u8, x: i32) -> Option<WorkspaceId> {
    let ws = |n| WorkspaceId::new(n).unwrap();
    let (main, other) = (window(10), window(60));
    let mut backend = FakeBackend {
        focused: Some(other.clone()),
        frontmost: Some(other.app.clone()),
        ..FakeBackend::default()
    };
    backend.frames.insert(main.clone(), frame(50));
    backend.frames.insert(other.clone(), frame(x));
    let mut rt = runtime(backend);
    bind(&mut rt, ws(1), &main);
    bind(&mut rt, ws(occupied), &other);
    let cancelled = AtomicBool::new(false);
    for n in [occupied, empty] {
        rt.switch(ws(n), three_outputs(), &cancelled).unwrap();
    }
    assert_eq!(rt.last_focused_workspace, Some(ws(empty)));
    rt.move_workspace_to_display(DisplayTarget::Next, three_outputs(), &cancelled)
        .unwrap();
    rt.backend.persisted_monitors.first().map(|moved| moved.0)
}

#[test]
fn moves_the_empty_workspace_on_secondary() {
    assert_eq!(moved_first(6, 7, 1050), Some(WorkspaceId::new(7).unwrap()));
}

#[test]
fn control_moves_the_empty_workspace_on_main() {
    assert_eq!(moved_first(2, 3, 60), Some(WorkspaceId::new(3).unwrap()));
}

#[test]
fn a_focused_window_on_a_visible_workspace_still_wins() {
    let ws = |n| WorkspaceId::new(n).unwrap();
    let (main, other) = (window(10), window(60));
    let mut backend = FakeBackend {
        focused: Some(main.clone()),
        frontmost: Some(main.app.clone()),
        ..FakeBackend::default()
    };
    backend.frames.insert(main.clone(), frame(50));
    backend.frames.insert(other.clone(), frame(1050));
    let mut rt = runtime(backend);
    bind(&mut rt, ws(1), &main);
    bind(&mut rt, ws(6), &other);
    let cancelled = AtomicBool::new(false);
    rt.switch(ws(6), three_outputs(), &cancelled).unwrap();
    rt.backend.focused = Some(main.clone());
    rt.move_workspace_to_display(DisplayTarget::Next, three_outputs(), &cancelled)
        .unwrap();
    assert_eq!(
        rt.backend.persisted_monitors.first().map(|moved| moved.0),
        Some(ws(1))
    );
}
