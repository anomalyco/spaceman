use crate::window_memory::{RememberedWindow, WindowMemory};

use super::*;

const BUNDLE: &str = "com.example.Test";

fn relaunched(id: u32) -> crate::PhysicalWindowId {
    crate::PhysicalWindowId {
        app: AppInstanceId {
            pid: 2,
            launched_at: 2,
        },
        window_id: id,
    }
}

fn workspace(value: u8) -> WorkspaceId {
    WorkspaceId::new(value).unwrap()
}

fn memory(placements: &[(u8, crate::Rect)]) -> WindowMemory {
    let mut memory = WindowMemory::default();
    memory.record(
        BUNDLE,
        placements
            .iter()
            .map(|(value, frame)| RememberedWindow {
                workspace: workspace(*value),
                frame: *frame,
                floating: false,
            })
            .collect(),
        1,
    );
    memory
}

fn backend_with(windows: &[(crate::PhysicalWindowId, crate::Rect)]) -> FakeBackend {
    FakeBackend {
        frames: windows.iter().cloned().collect(),
        observation: Some(observation_for_windows(BUNDLE, &windows[0].0, windows)),
        ..FakeBackend::default()
    }
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
fn relaunched_windows_return_to_their_remembered_workspaces() {
    // Recreated in the opposite order: frames, not window IDs, decide.
    let first = relaunched(10);
    let second = relaunched(11);
    let mut runtime = runtime(backend_with(&[
        (first.clone(), frame(500)),
        (second.clone(), frame(50)),
    ]));
    runtime.restore_window_memory(memory(&[(3, frame(50)), (1, frame(500))]));

    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();

    assert_eq!(
        runtime.workspaces.workspace_of(&first),
        Some(WorkspaceId::ONE)
    );
    assert_eq!(runtime.workspaces.workspace_of(&second), Some(workspace(3)));
    assert!(runtime.parked.contains_key(&second));
    assert!(!runtime.parked.contains_key(&first));
    assert_eq!(
        recalls(&runtime),
        vec![(10, WorkspaceId::ONE), (11, workspace(3))]
    );
}

#[test]
fn an_instance_that_already_owns_windows_never_recalls() {
    let owned = window(10);
    let opened = window(11);
    let mut runtime = runtime(backend_with(&[
        (owned.clone(), frame(50)),
        (opened.clone(), frame(500)),
    ]));
    runtime.restore_window_memory(memory(&[(3, frame(50)), (4, frame(500))]));
    bind(&mut runtime, WorkspaceId::ONE, &owned);

    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();

    assert_eq!(
        runtime.workspaces.workspace_of(&owned),
        Some(WorkspaceId::ONE)
    );
    assert_eq!(
        runtime.workspaces.workspace_of(&opened),
        Some(WorkspaceId::ONE)
    );
    assert!(recalls(&runtime).is_empty());
}

#[test]
fn route_rules_take_precedence_over_memory() {
    let window = relaunched(10);
    let config = Config::parse(&format!(
        "version = 1\n[[rules]]\nbundle_id = '{BUNDLE}'\naction = 'route'\nworkspace = 2"
    ))
    .unwrap();
    let mut runtime = runtime_with_config(backend_with(&[(window.clone(), frame(50))]), config);
    runtime.restore_window_memory(memory(&[(3, frame(50))]));

    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();

    assert_eq!(runtime.workspaces.workspace_of(&window), Some(workspace(2)));
    assert!(recalls(&runtime).is_empty());
}

#[test]
fn windows_opened_after_the_recall_closes_are_adopted_normally() {
    let restored = relaunched(10);
    let later = relaunched(11);
    let mut runtime = runtime(backend_with(&[(restored.clone(), frame(50))]));
    runtime.restore_window_memory(memory(&[(3, frame(50)), (4, frame(500))]));
    let cancelled = AtomicBool::new(false);
    runtime.cycle(outputs(), &cancelled).unwrap();
    assert_eq!(
        runtime.workspaces.workspace_of(&restored),
        Some(workspace(3))
    );

    // The recall window lapses with workspace 4's placement unclaimed.
    runtime
        .memory
        .recalls
        .get_mut(&restored.app)
        .unwrap()
        .opened -= crate::app::memory::RECALL_WINDOW;
    runtime.backend.frames.insert(later.clone(), frame(500));
    runtime.backend.observation = Some(observation_for_windows(
        BUNDLE,
        &later,
        &[(restored.clone(), frame(50)), (later.clone(), frame(500))],
    ));
    runtime.cycle(outputs(), &cancelled).unwrap();

    assert_eq!(
        runtime.workspaces.workspace_of(&later),
        Some(WorkspaceId::ONE)
    );
    assert!(!runtime.memory.recalls.contains_key(&restored.app));
    assert!(runtime.backend.journal.iter().any(|event| matches!(
        event,
        crate::journal::JournalEvent::RecallExpired { unclaimed: 1, .. }
    )));
}

#[test]
fn a_recall_closes_when_its_app_quits_and_recording_resumes() {
    let restored = relaunched(10);
    let mut runtime = runtime(backend_with(&[(restored.clone(), frame(50))]));
    runtime.restore_window_memory(memory(&[(3, frame(50)), (4, frame(500))]));
    let cancelled = AtomicBool::new(false);
    runtime.cycle(outputs(), &cancelled).unwrap();
    assert!(runtime.memory.recalls.contains_key(&restored.app));

    // The relaunched instance quits inside its recall window and is never
    // observed again; another instance of the bundle keeps running.
    let survivor = window(20);
    bind(&mut runtime, WorkspaceId::ONE, &survivor);
    runtime.backend.frames.remove(&restored);
    runtime.backend.frames.insert(survivor.clone(), frame(700));
    runtime.backend.running_apps = Some(HashSet::from([survivor.app.clone()]));
    runtime.backend.observation = Some(observation_for(BUNDLE, &survivor, frame(700)));
    runtime.cycle(outputs(), &cancelled).unwrap();
    assert!(runtime.memory.recalls.is_empty());
    assert!(runtime.backend.journal.iter().any(|event| matches!(
        event,
        crate::journal::JournalEvent::RecallExpired { unclaimed: 1, .. }
    )));

    // Let the survivor's tiled frame settle before aging the pending arrangement.
    runtime.cycle(outputs(), &cancelled).unwrap();
    for (_, since) in runtime.memory.pending.values_mut() {
        *since -= crate::app::memory::MEMORY_SETTLE;
    }
    runtime.cycle(outputs(), &cancelled).unwrap();
    assert!(
        runtime
            .memory
            .memory
            .get(BUNDLE)
            .unwrap()
            .windows
            .iter()
            .any(|placement| placement.workspace == WorkspaceId::ONE)
    );
}

#[test]
fn memory_records_only_arrangements_that_settle() {
    let first = window(10);
    let second = window(11);
    let mut runtime = runtime(backend_with(&[
        (first.clone(), frame(50)),
        (second.clone(), frame(500)),
    ]));
    let cancelled = AtomicBool::new(false);
    runtime.cycle(outputs(), &cancelled).unwrap();
    runtime.cycle(outputs(), &cancelled).unwrap();
    assert!(runtime.memory.memory.is_empty(), "recorded before settling");

    for (_, since) in runtime.memory.pending.values_mut() {
        *since -= crate::app::memory::MEMORY_SETTLE;
    }
    runtime.cycle(outputs(), &cancelled).unwrap();
    let remembered = runtime.memory.memory.get(BUNDLE).unwrap();
    assert_eq!(remembered.windows.len(), 2);
    assert!(
        remembered
            .windows
            .iter()
            .all(|placement| placement.workspace == WorkspaceId::ONE)
    );
    assert_eq!(
        runtime.backend.window_memory.as_ref(),
        Some(&runtime.memory.memory)
    );

    // Quitting closes windows one at a time: the shrinking intermediate
    // arrangement must not replace the memory before the process exits.
    runtime.backend.frames.remove(&second);
    runtime.backend.observation = Some(observation_for(BUNDLE, &first, frame(50)));
    runtime.cycle(outputs(), &cancelled).unwrap();
    runtime.backend.frames.remove(&first);
    runtime.backend.running_apps = Some(HashSet::new());
    runtime.backend.observation = Some(crate::NativeObservation {
        cycle: 0,
        mouse_down: false,
        applications: vec![crate::AppObservation::Terminated {
            app: first.app.clone(),
        }],
    });
    runtime.cycle(outputs(), &cancelled).unwrap();

    assert_eq!(runtime.memory.memory.get(BUNDLE).unwrap().windows.len(), 2);
}

#[test]
fn apps_launched_after_the_startup_recall_window_open_on_the_active_workspace() {
    let window = relaunched(10);
    let mut runtime = runtime(backend_with(&[(window.clone(), frame(50))]));
    runtime.restore_window_memory(memory(&[(3, frame(50))]));
    runtime.memory.started_at = Some(Instant::now() - crate::app::memory::RECALL_WINDOW);

    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();

    assert_eq!(
        runtime.workspaces.workspace_of(&window),
        Some(WorkspaceId::ONE)
    );
    assert!(!runtime.parked.contains_key(&window));
    assert!(recalls(&runtime).is_empty());
}

fn second_instance(id: u32) -> crate::PhysicalWindowId {
    crate::PhysicalWindowId {
        app: AppInstanceId {
            pid: 3,
            launched_at: 3,
        },
        window_id: id,
    }
}

/// A runtime past its startup recall window whose app instance `first`
/// lives on workspace 3, as remembered.
fn settled_on_three(first: &crate::PhysicalWindowId) -> WorkspaceRuntime<FakeBackend> {
    let mut runtime = runtime(backend_with(&[(first.clone(), frame(50))]));
    runtime.restore_window_memory(memory(&[(3, frame(50))]));
    runtime.memory.started_at = Some(Instant::now() - crate::app::memory::RECALL_WINDOW);
    bind(&mut runtime, workspace(3), first);
    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();
    assert_eq!(runtime.workspaces.workspace_of(first), Some(workspace(3)));
    runtime
}

fn terminated(app: &AppInstanceId) -> crate::AppObservation {
    crate::AppObservation::Terminated { app: app.clone() }
}

#[test]
fn an_app_relaunched_right_after_quitting_returns_to_its_workspace() {
    // An update quits the app and opens the new build while workspace 1 is
    // active: the old process's exit and the new window arrive together.
    let first = relaunched(10);
    let second = second_instance(20);
    let mut runtime = settled_on_three(&first);
    runtime.backend.frames.remove(&first);
    runtime.backend.frames.insert(second.clone(), frame(50));
    runtime.backend.running_apps = Some(HashSet::from([second.app.clone()]));
    let mut observation = observation_for(BUNDLE, &second, frame(50));
    observation.applications.push(terminated(&first.app));
    runtime.backend.observation = Some(observation);
    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();

    assert_eq!(runtime.workspaces.workspace_of(&second), Some(workspace(3)));
    assert!(runtime.parked.contains_key(&second));
    assert_eq!(recalls(&runtime), vec![(20, workspace(3))]);
}

#[test]
fn a_new_instance_started_before_the_old_one_quits_takes_its_workspace() {
    // scripts that swap builds start the new process first.
    let first = relaunched(10);
    let second = second_instance(20);
    let mut runtime = settled_on_three(&first);
    runtime.backend.frames.insert(second.clone(), frame(50));
    let mut observation = observation_for(BUNDLE, &first, frame(50));
    observation
        .applications
        .extend(observation_for(BUNDLE, &second, frame(50)).applications);
    runtime.backend.observation = Some(observation);
    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();

    assert_eq!(runtime.workspaces.workspace_of(&second), Some(workspace(3)));
    assert_eq!(recalls(&runtime), vec![(20, workspace(3))]);
}

#[test]
fn an_app_reopened_long_after_quitting_opens_on_the_active_workspace() {
    let first = relaunched(10);
    let second = second_instance(20);
    let mut runtime = settled_on_three(&first);
    let cancelled = AtomicBool::new(false);
    runtime.backend.frames.remove(&first);
    runtime.backend.running_apps = Some(HashSet::new());
    runtime.backend.observation = Some(crate::NativeObservation {
        cycle: 0,
        mouse_down: false,
        applications: vec![terminated(&first.app)],
    });
    runtime.cycle(outputs(), &cancelled).unwrap();
    for quit in runtime.memory.departed.values_mut() {
        *quit -= crate::app::memory::RELAUNCH_WINDOW;
    }

    runtime.backend.frames.insert(second.clone(), frame(50));
    runtime.backend.running_apps = Some(HashSet::from([second.app.clone()]));
    runtime.backend.observation = Some(observation_for(BUNDLE, &second, frame(50)));
    runtime.cycle(outputs(), &cancelled).unwrap();

    assert_eq!(
        runtime.workspaces.workspace_of(&second),
        Some(WorkspaceId::ONE)
    );
    assert!(!runtime.parked.contains_key(&second));
    assert!(recalls(&runtime).is_empty());
}

#[test]
fn app_with_no_manageable_windows_does_not_open_or_expire_a_recall() {
    let helper = relaunched(10);
    let backend = FakeBackend {
        observation: Some(observation_for_windows(BUNDLE, &helper, &[])),
        ..FakeBackend::default()
    };
    let mut runtime = runtime(backend);
    runtime.restore_window_memory(memory(&[(3, frame(50))]));

    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();

    assert!(runtime.memory.recalls.is_empty());
    assert!(recalls(&runtime).is_empty());
}
