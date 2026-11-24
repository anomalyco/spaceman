use super::*;

fn rect(x: i32, y: i32, width: i32, height: i32) -> crate::Rect {
    crate::Rect {
        x,
        y,
        width,
        height,
    }
}

fn landscape_outputs() -> macos::DisplayOutputs {
    macos::DisplayOutputs::new(vec![macos::DisplayOutput {
        id: "main-id".into(),
        name: "Main".into(),
        frame: rect(0, 0, 1280, 800),
        is_main: true,
    }])
    .unwrap()
}

fn portrait_outputs() -> macos::DisplayOutputs {
    macos::DisplayOutputs::new(vec![macos::DisplayOutput {
        id: "portrait-id".into(),
        name: "Portrait".into(),
        frame: rect(0, 0, 1080, 1920),
        is_main: true,
    }])
    .unwrap()
}

fn columns_config() -> Config {
    Config::parse("version = 1\n[settings]\nlayout = \"columns\"\n").unwrap()
}

#[test]
fn dwindle_default_tiles_one_to_four_windows_on_landscape_and_re_expands_on_close() {
    let w1 = window(10);
    let w2 = window(11);
    let w3 = window(12);
    let w4 = window(13);
    let mut backend = FakeBackend::default();
    backend.frames.insert(w1.clone(), frame(50));
    backend.observation = Some(observation_for_windows(
        "com.example.Test",
        &w1,
        &[(w1.clone(), frame(50))],
    ));
    let mut rt = runtime(backend);
    let outputs = landscape_outputs();
    let cancelled = AtomicBool::new(false);

    // 1 window -> 100%.
    rt.cycle(outputs.clone(), &cancelled).unwrap();
    assert_eq!(rt.backend.frames[&w1], rect(0, 0, 1280, 800));

    // 2 windows -> 50% | 50%.
    rt.backend.frames.insert(w2.clone(), frame(100));
    rt.backend.observation = Some(observation_for_windows(
        "com.example.Test",
        &w2,
        &[(w1.clone(), frame(50)), (w2.clone(), frame(100))],
    ));
    rt.cycle(outputs.clone(), &cancelled).unwrap();
    assert_eq!(rt.backend.frames[&w1], rect(0, 0, 640, 800));
    assert_eq!(rt.backend.frames[&w2], rect(640, 0, 640, 800));

    // 3 windows -> [ w1 | (w2 / w3) ].
    rt.backend.frames.insert(w3.clone(), frame(150));
    rt.backend.observation = Some(observation_for_windows(
        "com.example.Test",
        &w3,
        &[
            (w1.clone(), frame(50)),
            (w2.clone(), frame(100)),
            (w3.clone(), frame(150)),
        ],
    ));
    rt.cycle(outputs.clone(), &cancelled).unwrap();
    assert_eq!(rt.backend.frames[&w1], rect(0, 0, 640, 800));
    assert_eq!(rt.backend.frames[&w2], rect(640, 0, 640, 400));
    assert_eq!(rt.backend.frames[&w3], rect(640, 400, 640, 400));

    // 4 windows -> [ w1 | (w2 / (w3 | w4)) ].
    rt.backend.frames.insert(w4.clone(), frame(200));
    rt.backend.observation = Some(observation_for_windows(
        "com.example.Test",
        &w4,
        &[
            (w1.clone(), frame(50)),
            (w2.clone(), frame(100)),
            (w3.clone(), frame(150)),
            (w4.clone(), frame(200)),
        ],
    ));
    rt.cycle(outputs.clone(), &cancelled).unwrap();
    assert_eq!(rt.backend.frames[&w1], rect(0, 0, 640, 800));
    assert_eq!(rt.backend.frames[&w2], rect(640, 0, 640, 400));
    assert_eq!(rt.backend.frames[&w3], rect(640, 400, 320, 400));
    assert_eq!(rt.backend.frames[&w4], rect(960, 400, 320, 400));

    // Close w4 -> w3 re-expands to the full bottom-right cell.
    rt.backend.frames.remove(&w4);
    rt.backend.observation = Some(observation_for_windows(
        "com.example.Test",
        &w3,
        &[
            (w1.clone(), frame(50)),
            (w2.clone(), frame(100)),
            (w3.clone(), frame(150)),
        ],
    ));
    rt.cycle(outputs.clone(), &cancelled).unwrap();
    assert_eq!(rt.backend.frames[&w3], rect(640, 400, 640, 400));

    // Close w1 -> surviving (w2 / w3) expands to full width and promotes to root.
    rt.backend.frames.remove(&w1);
    rt.backend.observation = Some(observation_for_windows(
        "com.example.Test",
        &w2,
        &[(w2.clone(), frame(100)), (w3.clone(), frame(150))],
    ));
    rt.cycle(outputs, &cancelled).unwrap();
    assert_eq!(rt.backend.frames[&w2], rect(0, 0, 1280, 400));
    assert_eq!(rt.backend.frames[&w3], rect(0, 400, 1280, 400));
    assert_eq!(
        rt.workspaces.engine(WorkspaceId::ONE).root_axis(),
        crate::Axis::Vertical
    );
}

#[test]
fn dwindle_portrait_display_splits_vertically_first() {
    let w1 = window(10);
    let w2 = window(11);
    let w3 = window(12);
    let mut backend = FakeBackend::default();
    backend.frames.insert(w1.clone(), frame(50));
    backend.frames.insert(w2.clone(), frame(100));
    backend.observation = Some(observation_for_windows(
        "com.example.Test",
        &w2,
        &[(w1.clone(), frame(50)), (w2.clone(), frame(100))],
    ));
    let mut rt = runtime(backend);
    let outputs = portrait_outputs();
    let cancelled = AtomicBool::new(false);

    // 2 windows on portrait -> 50% / 50% top/bottom (#208).
    rt.cycle(outputs.clone(), &cancelled).unwrap();
    assert_eq!(rt.backend.frames[&w1], rect(0, 0, 1080, 960));
    assert_eq!(rt.backend.frames[&w2], rect(0, 960, 1080, 960));

    // 3rd window splits the bottom 1080x960 tile left/right.
    rt.backend.frames.insert(w3.clone(), frame(150));
    rt.backend.observation = Some(observation_for_windows(
        "com.example.Test",
        &w3,
        &[
            (w1.clone(), frame(50)),
            (w2.clone(), frame(100)),
            (w3.clone(), frame(150)),
        ],
    ));
    rt.cycle(outputs, &cancelled).unwrap();
    assert_eq!(rt.backend.frames[&w1], rect(0, 0, 1080, 960));
    assert_eq!(rt.backend.frames[&w2], rect(0, 960, 540, 960));
    assert_eq!(rt.backend.frames[&w3], rect(540, 960, 540, 960));
}

#[test]
fn live_config_reload_switches_between_dwindle_and_columns() {
    let w1 = window(10);
    let w2 = window(11);
    let w3 = window(12);
    let mut backend = FakeBackend::default();
    backend.frames.insert(w1.clone(), frame(50));
    backend.frames.insert(w2.clone(), frame(100));
    backend.observation = Some(observation_for_windows(
        "com.example.Test",
        &w2,
        &[(w1.clone(), frame(50)), (w2.clone(), frame(100))],
    ));
    let mut rt = runtime(backend);
    let outputs = landscape_outputs();
    let cancelled = AtomicBool::new(false);

    // Start in Dwindle with 2 windows -> [ w1 | w2 ].
    rt.cycle(outputs.clone(), &cancelled).unwrap();
    assert_eq!(rt.backend.frames[&w1], rect(0, 0, 640, 800));
    assert_eq!(rt.backend.frames[&w2], rect(640, 0, 640, 800));

    // Hot-reload to Columns without restarting -> 3rd window joins as 3 equal columns.
    rt.replace_config(columns_config());
    rt.backend.frames.insert(w3.clone(), frame(150));
    rt.backend.observation = Some(observation_for_windows(
        "com.example.Test",
        &w3,
        &[
            (w1.clone(), frame(50)),
            (w2.clone(), frame(100)),
            (w3.clone(), frame(150)),
        ],
    ));
    rt.cycle(outputs.clone(), &cancelled).unwrap();
    assert_eq!(rt.backend.frames[&w1].height, 800);
    assert_eq!(rt.backend.frames[&w2].height, 800);
    assert_eq!(rt.backend.frames[&w3].height, 800);
    assert_eq!(
        rt.backend.frames[&w1].width + rt.backend.frames[&w2].width + rt.backend.frames[&w3].width,
        1280
    );

    // Hot-reload back to Dwindle -> 4th window splits w3 vertically (427x800).
    rt.replace_config(Config::default());
    let w4 = window(13);
    rt.backend.frames.insert(w4.clone(), frame(200));
    rt.backend.observation = Some(observation_for_windows(
        "com.example.Test",
        &w4,
        &[
            (w1.clone(), frame(50)),
            (w2.clone(), frame(100)),
            (w3.clone(), frame(150)),
            (w4.clone(), frame(200)),
        ],
    ));
    rt.cycle(outputs, &cancelled).unwrap();
    assert_eq!(rt.backend.frames[&w3].height, 400);
    assert_eq!(rt.backend.frames[&w4].height, 400);
    assert_eq!(rt.backend.frames[&w4].y, 400);
}

#[test]
fn moving_windows_to_another_workspace_follows_dwindle_layout() {
    let w1 = window(10);
    let w2 = window(11);
    let w3 = window(12);
    let mut backend = FakeBackend::default();
    backend.frames.insert(w1.clone(), frame(50));
    backend.frames.insert(w2.clone(), frame(100));
    backend.frames.insert(w3.clone(), frame(150));
    let mut rt = runtime(backend);
    let outputs = landscape_outputs();
    let cancelled = AtomicBool::new(false);

    // Put w1 and w2 on workspace 2, w3 on workspace 1.
    rt.workspaces
        .engine_mut(WorkspaceId::TWO)
        .insert_slot(Slot {
            id: SlotId(1),
            weight: 1,
            binding: BindingState::Bound(w1.clone()),
        });
    rt.workspaces
        .engine_mut(WorkspaceId::TWO)
        .insert_slot(Slot {
            id: SlotId(2),
            weight: 1,
            binding: BindingState::Bound(w2.clone()),
        });
    rt.workspaces
        .engine_mut(WorkspaceId::ONE)
        .insert_slot(Slot {
            id: SlotId(3),
            weight: 1,
            binding: BindingState::Bound(w3.clone()),
        });

    rt.move_window(w3.clone(), WorkspaceId::TWO, outputs.clone(), &cancelled)
        .unwrap();
    let placements = rt
        .workspaces
        .engine(WorkspaceId::TWO)
        .placements(&Settings::for_output(outputs.main().frame));
    assert_eq!(placements.len(), 3);
    assert_eq!(placements[0].frame, rect(0, 0, 640, 800));
    assert_eq!(placements[1].frame, rect(640, 0, 640, 400));
    assert_eq!(placements[2].frame, rect(640, 400, 640, 400));
}

#[test]
fn dwindle_supports_axis_toggle_and_fullscreen_layout() {
    let w1 = window(10);
    let w2 = window(11);
    let w3 = window(12);
    let mut backend = FakeBackend {
        focused: Some(w2.clone()),
        ..FakeBackend::default()
    };
    backend.frames.insert(w1.clone(), frame(50));
    backend.frames.insert(w2.clone(), frame(100));
    backend.frames.insert(w3.clone(), frame(150));
    backend.observation = Some(observation_for_windows(
        "com.example.Test",
        &w2,
        &[
            (w1.clone(), frame(50)),
            (w2.clone(), frame(100)),
            (w3.clone(), frame(150)),
        ],
    ));
    let mut rt = runtime(backend);
    let outputs = landscape_outputs();
    let cancelled = AtomicBool::new(false);

    rt.cycle(outputs.clone(), &cancelled).unwrap();
    assert_eq!(rt.backend.frames[&w1], rect(0, 0, 640, 800));
    assert_eq!(rt.backend.frames[&w2], rect(640, 0, 640, 400));
    assert_eq!(rt.backend.frames[&w3], rect(640, 400, 640, 400));

    // Toggle root axis flips [ w1 | (w2 / w3) ] to [ w1 / (w2 / w3) ].
    rt.toggle_layout(outputs.clone(), &cancelled).unwrap();
    assert_eq!(
        rt.workspaces.engine(WorkspaceId::ONE).root_axis(),
        crate::Axis::Vertical
    );
    assert_eq!(rt.backend.frames[&w1], rect(0, 0, 1280, 400));

    // Fullscreen Layout still fills the display and restores the layout on toggle off.
    rt.toggle_fullscreen_layout(outputs.clone(), &cancelled)
        .unwrap();
    assert!(rt.workspaces.uses_fullscreen_layout(WorkspaceId::ONE));
    assert_eq!(rt.backend.frames[&w2], rect(0, 0, 1280, 800));

    rt.toggle_fullscreen_layout(outputs, &cancelled).unwrap();
    assert!(!rt.workspaces.uses_fullscreen_layout(WorkspaceId::ONE));
    assert_eq!(rt.backend.frames[&w1], rect(0, 0, 1280, 400));
}
