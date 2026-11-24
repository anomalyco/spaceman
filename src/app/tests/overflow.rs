use super::*;

fn one_output() -> macos::DisplayOutputs {
    macos::DisplayOutputs::new(vec![outputs().displays()[0].clone()]).unwrap()
}

fn overflow_config(capacity: u8) -> Config {
    Config::parse(&format!("version = 1\n[labs]\noverflow = {capacity}\n")).unwrap()
}

fn backend_with(windows: &[(crate::PhysicalWindowId, crate::Rect)]) -> FakeBackend {
    let mut backend = FakeBackend {
        observation: Some(observation_for_windows(
            "com.example.Test",
            &windows[0].0,
            windows,
        )),
        ..FakeBackend::default()
    };
    for (window, frame) in windows {
        backend.frames.insert(window.clone(), *frame);
    }
    backend
}

#[test]
fn overflow_spreads_new_windows_across_the_only_display_workspaces() {
    let windows: Vec<_> = (11..=15).map(|id| (window(id), frame(50))).collect();
    let mut runtime = runtime_with_config(backend_with(&windows), overflow_config(2));

    runtime
        .cycle(one_output(), &AtomicBool::new(false))
        .unwrap();

    let placed: Vec<_> = windows
        .iter()
        .map(|(window, _)| runtime.workspaces.workspace_of(window))
        .collect();
    assert_eq!(
        placed,
        [
            Some(WorkspaceId::ONE),
            Some(WorkspaceId::ONE),
            Some(WorkspaceId::TWO),
            Some(WorkspaceId::TWO),
            Some(WorkspaceId::THREE),
        ]
    );
    for (window, _) in &windows[2..] {
        assert!(
            runtime.parked.contains_key(window),
            "{window:?} overflowed to a hidden workspace and is parked"
        );
    }
}

#[test]
fn overflow_counts_tiles_the_active_workspace_already_holds() {
    let existing = window(10);
    let created = window(11);
    let windows = [(existing.clone(), frame(50)), (created.clone(), frame(60))];
    let mut runtime = runtime_with_config(backend_with(&windows), overflow_config(1));
    bind(&mut runtime, WorkspaceId::ONE, &existing);

    runtime
        .cycle(one_output(), &AtomicBool::new(false))
        .unwrap();

    assert_eq!(
        runtime.workspaces.workspace_of(&existing),
        Some(WorkspaceId::ONE)
    );
    assert_eq!(
        runtime.workspaces.workspace_of(&created),
        Some(WorkspaceId::TWO)
    );
}

#[test]
fn overflow_stays_on_the_display_the_window_opened_on() {
    let existing = window(10);
    let created = window(11);
    let windows = [
        (existing.clone(), frame(1050)),
        (created.clone(), frame(1060)),
    ];
    let mut runtime = runtime_with_config(backend_with(&windows), overflow_config(1));
    bind(&mut runtime, WorkspaceId::SIX, &existing);

    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();

    assert_eq!(
        runtime.workspaces.workspace_of(&created),
        WorkspaceId::new(7).ok()
    );
}

#[test]
fn overflow_is_off_by_default() {
    let windows: Vec<_> = (11..=13).map(|id| (window(id), frame(50))).collect();
    let mut runtime = runtime(backend_with(&windows));

    runtime
        .cycle(one_output(), &AtomicBool::new(false))
        .unwrap();

    for (window, _) in &windows {
        assert_eq!(
            runtime.workspaces.workspace_of(window),
            Some(WorkspaceId::ONE)
        );
    }
}

#[test]
fn overflow_leaves_route_rules_alone() {
    let existing = window(10);
    let created = window(11);
    let windows = [(existing.clone(), frame(50)), (created.clone(), frame(60))];
    let config = Config::parse(
        "version = 1\n[labs]\noverflow = 1\n[[rules]]\nbundle_id = 'com.example.Test'\naction = 'route'\nworkspace = 4",
    )
    .unwrap();
    let mut runtime = runtime_with_config(backend_with(&windows), config);

    runtime
        .cycle(one_output(), &AtomicBool::new(false))
        .unwrap();

    assert_eq!(
        runtime.workspaces.workspace_of(&existing),
        Some(WorkspaceId::FOUR)
    );
}

#[test]
fn overflow_joins_the_active_workspace_when_every_workspace_is_full() {
    let windows: Vec<_> = (11..=13).map(|id| (window(id), frame(50))).collect();
    let mut runtime = runtime_with_config(backend_with(&windows), overflow_config(1));
    for number in 2..=10 {
        let workspace = WorkspaceId::new(number).unwrap();
        bind(&mut runtime, workspace, &window(100 + u32::from(number)));
    }

    runtime
        .cycle(one_output(), &AtomicBool::new(false))
        .unwrap();

    for (window, _) in &windows {
        assert_eq!(
            runtime.workspaces.workspace_of(window),
            Some(WorkspaceId::ONE)
        );
    }
}
