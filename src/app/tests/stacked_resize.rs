//! #197: an automatic stack is temporary, so a keyboard resize made while it
//! shows must not save the stack as the requested split.
use super::*;

fn stacked_pair() -> (WorkspaceRuntime<FakeBackend>, [crate::PhysicalWindowId; 2]) {
    let windows = [window(10), window(11)];
    let mut backend = FakeBackend {
        focused: Some(windows[0].clone()),
        ..FakeBackend::default()
    };
    for w in &windows {
        backend.frames.insert(w.clone(), frame(0));
        backend.minimum_widths.insert(w.clone(), 600);
    }
    let mut rt = runtime(backend);
    for w in &windows {
        bind(&mut rt, WorkspaceId::ONE, w);
    }
    for _ in 0..2 {
        rt.layout_workspace(
            WorkspaceId::ONE,
            outputs().main().frame,
            &AtomicBool::new(false),
        );
    }
    (rt, windows)
}

fn side_by_side_when_wider(resize: bool) -> bool {
    let (mut rt, [a, b]) = stacked_pair();
    assert_eq!(
        rt.backend.frames[&a].x, rt.backend.frames[&b].x,
        "starts stacked"
    );
    let before = rt.workspaces.state();
    if resize {
        let stacked = (rt.backend.frames[&a], rt.backend.frames[&b]);
        rt.resize_focused(ResizeDirection::Grow, outputs(), &AtomicBool::new(false))
            .unwrap();
        assert_eq!(
            rt.workspaces.state(),
            before,
            "the stack never reaches saved state"
        );
        assert_ne!(
            (rt.backend.frames[&a], rt.backend.frames[&b]),
            stacked,
            "the resize still changes the stack shown"
        );
    }
    let wide = crate::Rect {
        width: 1400,
        ..outputs().main().frame
    };
    rt.layout_workspace(WorkspaceId::ONE, wide, &AtomicBool::new(false));
    rt.backend.frames[&a].x != rt.backend.frames[&b].x
}

#[test]
fn resizing_a_temporary_stack_keeps_the_requested_row() {
    assert!(side_by_side_when_wider(true));
}

#[test]
fn control_the_row_returns_without_a_resize() {
    assert!(side_by_side_when_wider(false));
}

#[test]
fn flipping_there_and_back_after_a_resize_keeps_the_row() {
    // Flipping the split there and back after a resize still lands on the row.
    let (mut rt, [a, b]) = stacked_pair();
    rt.resize_focused(ResizeDirection::Grow, outputs(), &AtomicBool::new(false))
        .unwrap();
    rt.toggle_layout(outputs(), &AtomicBool::new(false))
        .unwrap();
    rt.toggle_layout(outputs(), &AtomicBool::new(false))
        .unwrap();
    let wide = crate::Rect {
        width: 1400,
        ..outputs().main().frame
    };
    rt.layout_workspace(WorkspaceId::ONE, wide, &AtomicBool::new(false));
    assert_ne!(rt.backend.frames[&a].x, rt.backend.frames[&b].x);
}
