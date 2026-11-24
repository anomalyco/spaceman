//! Moving windows to a workspace on another display (Option+Shift+number).
use super::*;

/// Where a tiled window on Main's only tile lands.
fn main_tile() -> crate::Rect {
    outputs().main().frame
}

fn at(x: i32) -> crate::Rect {
    crate::Rect { x, ..frame(0) }
}

#[test]
fn moving_a_window_to_the_workspace_on_screen_on_another_display_tiles_it_there() {
    let window = window(20);
    let mut backend = FakeBackend {
        focused: Some(window.clone()),
        ..FakeBackend::default()
    };
    backend.frames.insert(window.clone(), frame(1050));
    let mut runtime = runtime(backend);
    bind(&mut runtime, WorkspaceId::SIX, &window);
    runtime.record_focused_workspace(WorkspaceId::SIX);

    runtime
        .move_focused(WorkspaceId::ONE, outputs(), &AtomicBool::new(false))
        .unwrap();

    assert_eq!(
        runtime.workspaces.workspace_of(&window),
        Some(WorkspaceId::ONE)
    );
    assert_eq!(runtime.backend.frames[&window], main_tile());
    assert!(!runtime.parked.contains_key(&window));
    assert_eq!(runtime.backend.focused, Some(window));
    // Focus followed the window to Main, so Option+Tab goes back to Secondary.
    assert_eq!(runtime.last_focused_workspace, Some(WorkspaceId::ONE));
    assert_eq!(runtime.previous_focused_workspace, Some(WorkspaceId::SIX));
}

#[test]
fn moving_a_window_to_a_hidden_workspace_on_another_display_parks_it_where_it_is() {
    let window = window(20);
    let mut backend = FakeBackend {
        focused: Some(window.clone()),
        // macOS clamps a write off Main's edge back onto Secondary, where
        // the window is. Moving it used to fail outright.
        refuses_position: Some(|target| target.x < 0),
        ..FakeBackend::default()
    };
    backend.frames.insert(window.clone(), frame(1050));
    let mut runtime = runtime(backend);
    bind(&mut runtime, WorkspaceId::SIX, &window);
    let target = WorkspaceId::new(2).unwrap();
    let cancelled = AtomicBool::new(false);

    runtime.move_focused(target, outputs(), &cancelled).unwrap();

    assert_eq!(runtime.workspaces.workspace_of(&window), Some(target));
    let displays = outputs().frames().collect::<Vec<_>>();
    let secondary = outputs().displays()[1].frame;
    assert_eq!(
        runtime.backend.frames[&window],
        crate::geometry::parked_frame(secondary, &displays, frame(1050))
    );
    // Recovery brings it back on Main, where its workspace lives.
    assert_eq!(runtime.parked[&window], frame(50));
    assert_eq!(runtime.backend.persisted[&window], frame(50));

    runtime.switch(target, outputs(), &cancelled).unwrap();

    assert_eq!(runtime.backend.frames[&window], main_tile());
    assert!(!runtime.parked.contains_key(&window));
    assert!(runtime.backend.persisted.is_empty());
}

#[test]
fn a_cross_display_move_that_cannot_park_leaves_the_window_where_it_was() {
    let window = window(20);
    let mut backend = FakeBackend {
        focused: Some(window.clone()),
        refuses_position: Some(|target| target.x != 1050),
        ..FakeBackend::default()
    };
    backend.frames.insert(window.clone(), frame(1050));
    let mut runtime = runtime(backend);
    bind(&mut runtime, WorkspaceId::SIX, &window);

    let Err(error) = runtime.move_focused(
        WorkspaceId::new(2).unwrap(),
        outputs(),
        &AtomicBool::new(false),
    ) else {
        panic!("a window that could not be parked was moved");
    };

    assert_eq!(error, "Window could not be parked in the target workspace");
    assert_eq!(
        runtime.workspaces.workspace_of(&window),
        Some(WorkspaceId::SIX)
    );
    assert_eq!(runtime.backend.frames[&window], frame(1050));
    assert!(runtime.parked.is_empty());
    assert!(runtime.backend.persisted.is_empty());
}

#[test]
fn moving_a_floating_window_to_the_workspace_on_screen_on_another_display_carries_it_there() {
    let floating = window(20);
    let mut backend = FakeBackend {
        focused: Some(floating.clone()),
        ..FakeBackend::default()
    };
    backend.frames.insert(floating.clone(), at(1100));
    let mut runtime = runtime(backend);
    runtime
        .workspaces
        .own_floating(floating.clone(), WorkspaceId::SIX);

    let status = runtime
        .move_focused(WorkspaceId::ONE, outputs(), &AtomicBool::new(false))
        .unwrap();

    assert_eq!(status.failed, 0);
    assert_eq!(
        runtime.workspaces.floating_workspace_of(&floating),
        Some(WorkspaceId::ONE)
    );
    // Same size, same place relative to the display, now on Main.
    assert_eq!(runtime.backend.frames[&floating], at(100));
}

#[test]
fn a_floating_window_moved_to_a_hidden_workspace_on_another_display_shows_up_there() {
    let floating = window(20);
    let mut backend = FakeBackend {
        focused: Some(floating.clone()),
        ..FakeBackend::default()
    };
    backend.frames.insert(floating.clone(), at(1100));
    let mut runtime = runtime(backend);
    runtime
        .workspaces
        .own_floating(floating.clone(), WorkspaceId::SIX);
    let target = WorkspaceId::new(2).unwrap();
    let cancelled = AtomicBool::new(false);

    runtime.move_focused(target, outputs(), &cancelled).unwrap();
    assert!(runtime.parked.contains_key(&floating));

    runtime.switch(target, outputs(), &cancelled).unwrap();

    assert_eq!(runtime.backend.frames[&floating], at(100));
    assert!(!runtime.parked.contains_key(&floating));
}

/// #121: move-app moves every managed window of the app, and leaves other
/// apps' windows where they are.
#[test]
fn move_app_moves_every_managed_window_of_the_app() {
    let first = window(10);
    let second = window(11);
    let other = crate::PhysicalWindowId {
        app: AppInstanceId {
            pid: 3,
            launched_at: 3,
        },
        window_id: 20,
    };
    let mut backend = FakeBackend {
        focused: Some(first.clone()),
        ..FakeBackend::default()
    };
    backend.frames.insert(first.clone(), frame(50));
    backend.frames.insert(second.clone(), frame(300));
    backend.frames.insert(other.clone(), frame(500));
    let mut runtime = runtime(backend);
    bind(&mut runtime, WorkspaceId::ONE, &first);
    bind(&mut runtime, WorkspaceId::ONE, &second);
    bind(&mut runtime, WorkspaceId::ONE, &other);
    runtime
        .memory
        .bundles
        .insert(first.app.clone(), "com.example.Test".into());
    runtime
        .memory
        .bundles
        .insert(other.app.clone(), "com.example.Other".into());

    let status = runtime
        .move_app(
            "com.example.Test",
            WorkspaceId::SIX,
            outputs(),
            &AtomicBool::new(false),
        )
        .unwrap();

    assert_eq!(status.failed, 0);
    assert_eq!(
        runtime.workspaces.workspace_of(&first),
        Some(WorkspaceId::SIX)
    );
    assert_eq!(
        runtime.workspaces.workspace_of(&second),
        Some(WorkspaceId::SIX)
    );
    assert_eq!(
        runtime.workspaces.workspace_of(&other),
        Some(WorkspaceId::ONE)
    );
}

/// #136: move-app takes the app's floating windows along with its tiles, and
/// finds an app whose only window floats.
#[test]
fn move_app_moves_floating_windows_too() {
    let (tile, floating) = (window(10), window(11));
    let mut backend = FakeBackend::default();
    backend.frames.insert(tile.clone(), frame(50));
    backend.frames.insert(floating.clone(), frame(500));
    let mut runtime = runtime(backend);
    bind(&mut runtime, WorkspaceId::ONE, &tile);
    runtime
        .workspaces
        .own_floating(floating.clone(), WorkspaceId::ONE);
    runtime
        .memory
        .bundles
        .insert(tile.app.clone(), "com.example.Test".into());
    let cancelled = AtomicBool::new(false);

    runtime
        .move_app("com.example.Test", WorkspaceId::SIX, outputs(), &cancelled)
        .unwrap();
    assert_eq!(
        runtime.workspaces.workspace_of(&tile),
        Some(WorkspaceId::SIX)
    );
    assert_eq!(
        runtime.workspaces.floating_workspace_of(&floating),
        Some(WorkspaceId::SIX)
    );

    // Only the float left: it is still the app's window to move.
    runtime.forget_window(WorkspaceId::SIX, &tile);
    runtime
        .move_app("com.example.Test", WorkspaceId::ONE, outputs(), &cancelled)
        .unwrap();
    assert_eq!(
        runtime.workspaces.floating_workspace_of(&floating),
        Some(WorkspaceId::ONE)
    );
}

#[test]
fn cross_display_route_and_recall_park_off_source_display_when_target_edge_is_refused() {
    use crate::window_memory::{RememberedWindow, WindowMemory};

    let target = WorkspaceId::new(2).unwrap();
    for recall in [false, true] {
        let id = window(20);
        let backend = FakeBackend {
            frames: [(id.clone(), frame(1050))].into_iter().collect(),
            observation: Some(observation(&id, frame(1050))),
            focused: Some(id.clone()),
            refuses_position: Some(|rect: crate::Rect| rect.x < 0),
            ..FakeBackend::default()
        };
        let config = if recall {
            Config::default()
        } else {
            Config::parse(
                "version = 1
[[rules]]
bundle_id = 'com.example.Test'
action = 'route'
workspace = 2",
            )
            .unwrap()
        };
        let mut rt = runtime_with_config(backend, config);
        if recall {
            let mut memory = WindowMemory::default();
            memory.record(
                "com.example.Test",
                vec![RememberedWindow {
                    workspace: target,
                    frame: frame(1050),
                    floating: false,
                }],
                1,
            );
            rt.restore_window_memory(memory);
        }

        let status = rt.cycle(outputs(), &AtomicBool::new(false)).unwrap();
        assert_eq!(
            status.failed, 0,
            "recall={recall}: parking should succeed off source display"
        );
        assert_eq!(
            rt.workspaces.workspace_of(&id),
            Some(target),
            "recall={recall}: window should transfer to hidden target workspace"
        );
        assert!(
            rt.parked.contains_key(&id),
            "recall={recall}: recovery frame must be persisted"
        );
        assert!(
            !crate::geometry::shows_on(
                rt.backend.frames[&id],
                &outputs().frames().collect::<Vec<_>>(),
            ),
            "recall={recall}: window should be parked off-screen"
        );
    }
}
