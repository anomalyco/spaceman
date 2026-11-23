use super::*;
use crate::{labs::LabFeature, strip::StripWidth};

const LAB_ON: &str = "version = 1\n[labs]\nscrolling_strip = true\n";

fn column(x: i32, width: i32) -> crate::Rect {
    crate::Rect {
        x,
        y: 0,
        width,
        height: 800,
    }
}

fn press(
    runtime: &mut WorkspaceRuntime<FakeBackend>,
    action: WorkerAction,
) -> Result<CycleStatus, String> {
    match runtime.execute(
        WorkerCommand::Action(action, outputs()),
        &AtomicBool::new(false),
    ) {
        Some(WorkerResult::Status { result, .. }) => result,
        _ => panic!("an action returns status"),
    }
}

/// Four tiled windows on workspace 1 (think Ghostty, Zed, Safari and Linear), focus on the first.
fn four_windows(config: &str) -> (WorkspaceRuntime<FakeBackend>, Vec<crate::PhysicalWindowId>) {
    let windows: Vec<_> = (1..=4).map(|index| window(index * 10)).collect();
    let frames: Vec<_> = windows
        .iter()
        .zip(0..)
        .map(|(window, index)| (window.clone(), frame(index * 200)))
        .collect();
    let backend = FakeBackend {
        frames: frames.iter().cloned().collect(),
        focused: Some(windows[0].clone()),
        frontmost: Some(windows[0].app.clone()),
        observation: Some(observation_for_windows(
            "com.example.Test",
            &windows[0],
            &frames,
        )),
        ..FakeBackend::default()
    };
    let mut runtime = runtime_with_config(backend, Config::parse(config).unwrap());
    for window in &windows {
        bind(&mut runtime, WorkspaceId::ONE, window);
    }
    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();
    (runtime, windows)
}

/// Exactly the `visible` windows are on screen at these frames; every other one is parked,
/// durably, off every display.
fn assert_viewport(
    runtime: &WorkspaceRuntime<FakeBackend>,
    windows: &[crate::PhysicalWindowId],
    visible: &[(usize, crate::Rect)],
) {
    for (index, window) in windows.iter().enumerate() {
        let frame = runtime.backend.frames[window];
        match visible.iter().find(|(shown, _)| *shown == index) {
            Some((_, expected)) => {
                assert_eq!(frame, *expected, "window {index}");
                assert!(!runtime.parked.contains_key(window), "window {index}");
                assert!(!runtime.backend.persisted.contains_key(window));
            }
            None => {
                assert!(runtime.parked.contains_key(window), "window {index}");
                assert!(runtime.backend.persisted.contains_key(window));
                assert!(
                    !outputs()
                        .frames()
                        .any(|display| display.contains_center(frame)),
                    "window {index} at {frame:?}"
                );
            }
        }
    }
}

#[test]
fn lab_off_refuses_the_strip_and_tiles_as_usual_until_it_is_turned_on() {
    let (mut runtime, windows) = four_windows("version = 1\n");
    // A workspace switch left on while the Lab was on stays inert while it is off.
    runtime.workspaces.set_strip_layout(WorkspaceId::ONE, true);

    assert_eq!(
        press(&mut runtime, WorkerAction::ToggleStrip).err(),
        Some(LabFeature::ScrollingStrip.off_message())
    );
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();
    assert_eq!(runtime.backend.focused, Some(windows[1].clone()));
    assert_viewport(
        &runtime,
        &windows,
        &[
            (0, column(0, 250)),
            (1, column(250, 250)),
            (2, column(500, 250)),
            (3, column(750, 250)),
        ],
    );

    runtime.replace_config(Config::parse(LAB_ON).unwrap());
    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();

    assert_viewport(
        &runtime,
        &windows,
        &[(1, column(0, 660)), (2, column(660, 340))],
    );
}

#[test]
fn stepping_along_the_strip_parks_and_reveals_columns_durably() {
    let (mut runtime, windows) = four_windows(LAB_ON);

    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();
    assert!(runtime.workspaces.uses_strip_layout(WorkspaceId::ONE));
    assert_viewport(
        &runtime,
        &windows,
        &[(0, column(0, 660)), (1, column(660, 340))],
    );

    // ⌥L onto a column already on screen: the viewport stays put.
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    assert_eq!(runtime.backend.focused, Some(windows[1].clone()));
    assert_viewport(
        &runtime,
        &windows,
        &[(0, column(0, 340)), (1, column(340, 660))],
    );

    // ⌥L past the viewport: Safari is revealed, Ghostty parked at its last frame.
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    assert_eq!(runtime.backend.focused, Some(windows[2].clone()));
    assert_viewport(
        &runtime,
        &windows,
        &[(1, column(0, 340)), (2, column(340, 660))],
    );
    assert_eq!(runtime.backend.persisted[&windows[0]], column(0, 340));

    // ⌥H back inside the viewport.
    press(&mut runtime, WorkerAction::Focus(Direction::Left)).unwrap();
    assert_eq!(runtime.backend.focused, Some(windows[1].clone()));
    let settled = [(1, column(0, 660)), (2, column(660, 340))];
    assert_viewport(&runtime, &windows, &settled);

    // Observation cycles keep parked columns' slots and the viewport.
    for _ in 0..3 {
        runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();
    }
    assert_eq!(
        runtime.workspaces.engine(WorkspaceId::ONE).bound_windows(),
        windows
    );
    assert_viewport(&runtime, &windows, &settled);
    let state = runtime.backend.workspace_state.clone().unwrap();
    let mut restarted = Workspaces::default();
    restarted.restore(state);
    assert!(restarted.uses_strip_layout(WorkspaceId::ONE));
}

#[test]
fn focus_from_outside_spaceman_slides_the_viewport_to_a_parked_column() {
    let (mut runtime, windows) = four_windows(LAB_ON);
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();

    // Later, ⌘Tab or the Dock focuses Linear while it is parked.
    runtime.recently_parked.clear();
    runtime.backend.focused = Some(windows[3].clone());
    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();

    assert_viewport(
        &runtime,
        &windows,
        &[(2, column(0, 340)), (3, column(340, 660))],
    );
}

#[test]
fn pause_and_quit_restore_every_strip_parked_window() {
    let (mut runtime, windows) = four_windows(LAB_ON);
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    let recovery = runtime.parked.clone();
    assert_eq!(recovery.len(), 2);

    let result = runtime.execute(
        WorkerCommand::RestoreAll(Some(outputs())),
        &AtomicBool::new(false),
    );

    assert!(matches!(result, Some(WorkerResult::Restored { failed: 0 })));
    assert!(runtime.parked.is_empty());
    assert!(runtime.backend.persisted.is_empty());
    for window in &windows {
        let frame = runtime.backend.frames[window];
        assert!(outputs().main().frame.contains_center(frame));
        if let Some(original) = recovery.get(window) {
            assert_eq!((frame.x, frame.y), (original.x, original.y));
        }
    }
}

#[test]
fn switching_away_parks_every_column_and_back_reveals_only_the_viewport() {
    let (mut runtime, windows) = four_windows(LAB_ON);
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();
    let two = WorkspaceId::TWO;

    press(&mut runtime, WorkerAction::SwitchWorkspace(two)).unwrap();
    assert_viewport(&runtime, &windows, &[]);

    press(
        &mut runtime,
        WorkerAction::SwitchWorkspace(WorkspaceId::ONE),
    )
    .unwrap();
    assert_viewport(
        &runtime,
        &windows,
        &[(0, column(0, 660)), (1, column(660, 340))],
    );
}

#[test]
fn reordering_and_width_cycling_follow_the_focused_column() {
    let (mut runtime, windows) = four_windows(LAB_ON);
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();

    // ⌥⇧L carries Ghostty one column right; the viewport keeps it in view.
    press(&mut runtime, WorkerAction::Reorder(Direction::Right)).unwrap();
    let order = runtime.workspaces.engine(WorkspaceId::ONE).bound_windows();
    assert_eq!(
        order,
        vec![
            windows[1].clone(),
            windows[0].clone(),
            windows[2].clone(),
            windows[3].clone()
        ]
    );
    assert_viewport(
        &runtime,
        &windows,
        &[(0, column(0, 660)), (2, column(660, 340))],
    );

    // ⌥= to 100%: Ghostty alone; unsized neighbors follow the new width.
    press(&mut runtime, WorkerAction::Resize(ResizeDirection::Grow)).unwrap();
    assert_viewport(&runtime, &windows, &[(0, outputs().main().frame)]);
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    assert_viewport(&runtime, &windows, &[(2, outputs().main().frame)]);
    press(&mut runtime, WorkerAction::Focus(Direction::Left)).unwrap();
    assert_viewport(&runtime, &windows, &[(0, outputs().main().frame)]);

    // ⌥- ⌥-: 66% then 50%.
    press(&mut runtime, WorkerAction::Resize(ResizeDirection::Shrink)).unwrap();
    assert_viewport(
        &runtime,
        &windows,
        &[(0, column(0, 660)), (2, column(660, 340))],
    );
    press(&mut runtime, WorkerAction::Resize(ResizeDirection::Shrink)).unwrap();
    assert_viewport(
        &runtime,
        &windows,
        &[(0, column(0, 500)), (2, column(500, 500))],
    );
    assert_eq!(
        runtime.workspaces.strip(WorkspaceId::ONE).width(SlotId(10)),
        StripWidth::Half
    );

    // ⌥` cycles to the next column, wrapping at the end.
    press(&mut runtime, WorkerAction::CycleFocus).unwrap();
    assert_eq!(runtime.backend.focused, Some(windows[2].clone()));
}

#[test]
fn turning_the_strip_off_reveals_every_column_in_the_split_layout() {
    let (mut runtime, windows) = four_windows(LAB_ON);
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();

    assert!(!runtime.workspaces.uses_strip_layout(WorkspaceId::ONE));
    assert_viewport(
        &runtime,
        &windows,
        &[
            (0, column(0, 250)),
            (1, column(250, 250)),
            (2, column(500, 250)),
            (3, column(750, 250)),
        ],
    );

    // Fullscreen Layout and the strip exclude each other.
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();
    press(&mut runtime, WorkerAction::ToggleFullscreenLayout).unwrap();
    assert!(runtime.workspaces.uses_fullscreen_layout(WorkspaceId::ONE));
    assert!(!runtime.workspaces.uses_strip_layout(WorkspaceId::ONE));
    assert!(runtime.parked.is_empty());
}

#[test]
fn ghostty_native_tab_replacement_keeps_the_strip_column_and_width() {
    let previous = window(10);
    let other = window(20);
    let replacement = window(11);
    let windows = [(previous.clone(), frame(50)), (other.clone(), frame(500))];
    let backend = FakeBackend {
        frames: windows.iter().cloned().collect(),
        focused: Some(previous.clone()),
        frontmost: Some(previous.app.clone()),
        observation: Some(observation_for_windows(
            "com.mitchellh.ghostty",
            &previous,
            &windows,
        )),
        ..FakeBackend::default()
    };
    let mut runtime = runtime_with_config(backend, Config::parse(LAB_ON).unwrap());
    bind(&mut runtime, WorkspaceId::ONE, &previous);
    bind(&mut runtime, WorkspaceId::ONE, &other);
    let cancelled = AtomicBool::new(false);
    runtime.cycle(outputs(), &cancelled).unwrap();
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();
    press(&mut runtime, WorkerAction::Resize(ResizeDirection::Grow)).unwrap();
    runtime.cycle(outputs(), &cancelled).unwrap();
    let full = outputs().main().frame;
    assert_eq!(runtime.backend.frames[&previous], full);

    runtime.backend.frames.remove(&previous);
    runtime.backend.frames.insert(replacement.clone(), full);
    runtime.backend.focused = Some(replacement.clone());
    runtime.backend.observation = Some(observation_for_windows(
        "com.mitchellh.ghostty",
        &replacement,
        &[(replacement.clone(), full), (other.clone(), frame(500))],
    ));
    runtime.cycle(outputs(), &cancelled).unwrap();

    let engine = runtime.workspaces.engine(WorkspaceId::ONE);
    assert_eq!(engine.slot_for_window(&replacement), Some(SlotId(10)));
    assert_eq!(
        engine.bound_windows(),
        vec![replacement.clone(), other.clone()]
    );
    assert_eq!(
        runtime.workspaces.strip(WorkspaceId::ONE).width(SlotId(10)),
        StripWidth::Full
    );
    assert_eq!(runtime.backend.frames[&replacement], full);
    assert!(runtime.parked.contains_key(&other));
}

/// An app that drags a parked strip column back on screen in the background is pushed back
/// off, like a hidden workspace's window.
#[test]
fn a_strip_column_that_drifts_back_on_screen_is_re_parked() {
    let (mut runtime, windows) = four_windows(LAB_ON);
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    let ghostty = &windows[0];
    let recovery = runtime.parked[ghostty];
    runtime.backend.frames.insert(ghostty.clone(), frame(40));
    runtime.backend.frontmost = None;
    runtime.recently_parked.clear();

    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();

    let at = runtime.backend.frames[ghostty];
    assert!(
        !outputs()
            .frames()
            .any(|display| display.contains_center(at)),
        "still on screen at {at:?}"
    );
    assert_eq!(runtime.parked.get(ghostty), Some(&recovery));
    assert_eq!(
        runtime.workspaces.workspace_of(ghostty),
        Some(WorkspaceId::ONE)
    );
}

/// Docking a display can uncover columns parked against the laptop's edge. They are re-parked
/// against the new displays at once, and focus follow cannot re-home them meanwhile.
#[test]
fn docking_a_display_re_parks_hidden_strip_columns() {
    let (mut runtime, windows) = four_windows(LAB_ON);
    let laptop = macos::DisplayOutputs::new(vec![outputs().displays()[0].clone()]).unwrap();
    let cancelled = AtomicBool::new(false);
    runtime.cycle(laptop.clone(), &cancelled).unwrap();
    runtime
        .execute(
            WorkerCommand::Action(WorkerAction::ToggleStrip, laptop),
            &cancelled,
        )
        .unwrap();
    let hidden: Vec<_> = windows
        .iter()
        .filter(|window| runtime.parked.contains_key(*window))
        .cloned()
        .collect();
    assert_eq!(hidden.len(), 2);
    runtime.backend.frontmost = None;

    runtime.cycle(outputs(), &cancelled).unwrap();

    for window in &hidden {
        assert!(runtime.parked.contains_key(window));
        assert_eq!(
            runtime.workspaces.workspace_of(window),
            Some(WorkspaceId::ONE)
        );
        let at = runtime.backend.frames[window];
        assert!(
            !outputs()
                .frames()
                .any(|display| display.contains_center(at)),
            "{window:?} shows at {at:?}"
        );
    }
}
