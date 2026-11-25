//! Reopening an app whose windows lived on several workspaces: every window parked at quit is
//! rescued, and the arrangement is recalled however long the app was closed.

use crate::window_memory::{RememberedWindow, WindowMemory};

use super::*;

const SAFARI: &str = "com.apple.Safari";
const STICKIES: &str = "com.apple.Stickies";

fn workspace(value: u8) -> WorkspaceId {
    WorkspaceId::new(value).unwrap()
}

fn reopened(pid: i32, window_id: u32) -> crate::PhysicalWindowId {
    crate::PhysicalWindowId {
        app: AppInstanceId {
            pid,
            launched_at: 9,
        },
        window_id,
    }
}

fn remembered(bundle_id: &str, placements: &[(u8, crate::Rect, bool)]) -> WindowMemory {
    let mut memory = WindowMemory::default();
    assert!(
        memory.record(
            bundle_id,
            placements
                .iter()
                .map(|(value, frame, floating)| RememberedWindow {
                    workspace: workspace(*value),
                    frame: *frame,
                    floating: *floating,
                })
                .collect(),
            1,
        )
    );
    memory
}

/// A runtime long past its startup recall window, with no recent quit on record: the app was
/// closed minutes ago, not relaunched by an update.
fn reopened_minutes_later(
    bundle_id: &str,
    memory: WindowMemory,
    windows: &[(crate::PhysicalWindowId, crate::Rect)],
) -> WorkspaceRuntime<FakeBackend> {
    let mut runtime = runtime(FakeBackend {
        frames: windows.iter().cloned().collect(),
        observation: Some(observation_for_windows(bundle_id, &windows[0].0, windows)),
        ..FakeBackend::default()
    });
    runtime.restore_window_memory(memory);
    runtime.memory.started_at = Some(Instant::now() - crate::app::memory::RECALL_WINDOW);
    runtime
}

fn recalls(runtime: &WorkspaceRuntime<FakeBackend>) -> Vec<(u32, WorkspaceId)> {
    runtime
        .backend
        .journal
        .iter()
        .filter_map(|event| match event {
            crate::journal::JournalEvent::Recall {
                window, workspace, ..
            } => Some((*window, *workspace)),
            _ => None,
        })
        .collect()
}

#[test]
fn every_window_reopened_at_a_parking_spot_is_rescued_and_recalled() {
    // Safari quit with windows on workspaces 1, 2 and 3 while 2 and 3 were parked; macOS
    // reopens those two at the parking spot, and Safari is not even frontmost.
    let tile = outputs().main().frame;
    let parked = main_parked_frame(tile);
    let (first, second, third) = (reopened(7, 30), reopened(7, 31), reopened(7, 32));
    let mut runtime = reopened_minutes_later(
        SAFARI,
        remembered(
            SAFARI,
            &[(1, tile, false), (2, tile, false), (3, tile, false)],
        ),
        &[
            (first.clone(), tile),
            (second.clone(), parked),
            (third.clone(), parked),
        ],
    );

    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();

    assert_eq!(
        runtime.workspaces.workspace_of(&first),
        Some(WorkspaceId::ONE)
    );
    assert_eq!(runtime.workspaces.workspace_of(&second), Some(workspace(2)));
    assert_eq!(runtime.workspaces.workspace_of(&third), Some(workspace(3)));
    assert!(!runtime.parked.contains_key(&first));
    assert!(runtime.parked.contains_key(&second));
    assert!(runtime.parked.contains_key(&third));
    assert_eq!(
        recalls(&runtime),
        vec![
            (30, WorkspaceId::ONE),
            (31, workspace(2)),
            (32, workspace(3))
        ]
    );
}

#[test]
fn a_window_parked_by_spaceman_is_never_rescued() {
    let tile = outputs().main().frame;
    let owned = window(10);
    let mut runtime = runtime(FakeBackend {
        frames: HashMap::from([(owned.clone(), tile)]),
        observation: Some(observation(&owned, tile)),
        ..FakeBackend::default()
    });
    let cancelled = AtomicBool::new(false);
    runtime.cycle(outputs(), &cancelled).unwrap();
    runtime
        .switch(WorkspaceId::TWO, outputs(), &cancelled)
        .unwrap();
    let hidden = runtime.backend.frames[&owned];
    runtime.backend.calls.clear();

    runtime.cycle(outputs(), &cancelled).unwrap();

    assert!(runtime.parked.contains_key(&owned));
    assert_eq!(runtime.backend.frames[&owned], hidden);
    assert!(
        runtime.backend.calls.is_empty(),
        "{:?}",
        runtime.backend.calls
    );
}

#[test]
fn a_single_window_cold_launch_still_opens_on_the_active_workspace() {
    let tile = outputs().main().frame;
    let only = reopened(7, 30);
    let mut runtime = reopened_minutes_later(
        SAFARI,
        remembered(SAFARI, &[(2, tile, false), (3, tile, false)]),
        &[(only.clone(), tile)],
    );

    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();

    assert_eq!(
        runtime.workspaces.workspace_of(&only),
        Some(WorkspaceId::ONE)
    );
    assert!(recalls(&runtime).is_empty());
}

#[test]
fn reopened_notes_find_their_workspaces_by_size() {
    let sized = |x, width, height| crate::Rect {
        x,
        y: 100,
        width,
        height,
    };
    // Three notes of different sizes on workspaces 1, 2 and 3, reopened elsewhere and in a
    // different creation order.
    let memory = remembered(
        STICKIES,
        &[
            (1, sized(50, 220, 180), true),
            (2, sized(60, 300, 240), true),
            (3, sized(70, 180, 400), true),
        ],
    );
    let (tall, small, wide) = (reopened(8, 40), reopened(8, 41), reopened(8, 42));
    let mut runtime = reopened_minutes_later(
        STICKIES,
        memory,
        &[
            (tall.clone(), sized(100, 180, 400)),
            (small.clone(), sized(400, 220, 180)),
            (wide.clone(), sized(650, 300, 240)),
        ],
    );

    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();

    assert_eq!(
        runtime.workspaces.floating_workspace_of(&small),
        Some(WorkspaceId::ONE)
    );
    assert_eq!(
        runtime.workspaces.floating_workspace_of(&wide),
        Some(workspace(2))
    );
    assert_eq!(
        runtime.workspaces.floating_workspace_of(&tall),
        Some(workspace(3))
    );
    // A recalled float joins its workspace this cycle and parks with it on the next.
    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();
    assert!(!runtime.parked.contains_key(&small));
    assert!(runtime.parked.contains_key(&wide));
    assert!(runtime.parked.contains_key(&tall));
    let persisted = serde_json::to_string(&runtime.memory.memory).unwrap();
    assert!(!persisted.contains("title"), "{persisted}");
}
