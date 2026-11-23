use crate::journal::AnomalyKind;

use super::*;

fn gaps(inner: i32, outer: i32) -> Config {
    Config::parse(&format!(
        "version = 1\n[settings]\ngap_inner = {inner}\ngap_outer = {outer}\n"
    ))
    .unwrap()
}

fn rect(x: i32, y: i32, width: i32, height: i32) -> crate::Rect {
    crate::Rect {
        x,
        y,
        width,
        height,
    }
}

/// Windows 10 and 20 observed and tiled side by side on the main display.
fn pair(config: Config) -> WorkspaceRuntime<FakeBackend> {
    let windows = [(window(10), frame(50)), (window(20), frame(500))];
    let backend = FakeBackend {
        frames: windows.iter().cloned().collect(),
        focused: Some(window(10)),
        frontmost: Some(window(10).app),
        observation: Some(observation_for_windows(
            "com.example.Test",
            &window(10),
            &windows,
        )),
        ..FakeBackend::default()
    };
    let mut runtime = runtime_with_config(backend, config);
    bind(&mut runtime, WorkspaceId::ONE, &window(10));
    bind(&mut runtime, WorkspaceId::ONE, &window(20));
    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();
    runtime.backend.calls.clear();
    runtime
}

fn shown(runtime: &WorkspaceRuntime<FakeBackend>) -> [crate::Rect; 2] {
    [
        runtime.backend.frames[&window(10)],
        runtime.backend.frames[&window(20)],
    ]
}

fn applied(runtime: &WorkspaceRuntime<FakeBackend>) -> usize {
    runtime
        .backend
        .calls
        .iter()
        .filter(|call| matches!(call, BackendCall::Apply(..)))
        .count()
}

const TOUCHING: [crate::Rect; 2] = [
    crate::Rect {
        x: 0,
        y: 0,
        width: 500,
        height: 800,
    },
    crate::Rect {
        x: 500,
        y: 0,
        width: 500,
        height: 800,
    },
];

const GAPPED: [crate::Rect; 2] = [
    crate::Rect {
        x: 8,
        y: 8,
        width: 488,
        height: 784,
    },
    crate::Rect {
        x: 504,
        y: 8,
        width: 488,
        height: 784,
    },
];

#[test]
fn saving_gaps_retiles_the_visible_workspace_and_then_settles() {
    let mut runtime = pair(Config::default());
    let cancelled = AtomicBool::new(false);
    assert_eq!(shown(&runtime), TOUCHING);
    let layout = runtime.workspaces.state();

    runtime.replace_config(gaps(8, 8));
    runtime.cycle(outputs(), &cancelled).unwrap();

    assert_eq!(shown(&runtime), GAPPED);
    // Gaps are configuration, not layout: nothing about them is persisted.
    assert_eq!(runtime.workspaces.state(), layout);

    // The gapped frames are what the next observation expects to find.
    runtime.backend.calls.clear();
    runtime.cycle(outputs(), &cancelled).unwrap();
    runtime.cycle(outputs(), &cancelled).unwrap();
    assert_eq!(applied(&runtime), 0);
    assert_eq!(runtime.workspaces.state(), layout);

    // Removing the keys puts every tile back where it was.
    runtime.replace_config(Config::default());
    runtime.cycle(outputs(), &cancelled).unwrap();
    assert_eq!(shown(&runtime), TOUCHING);
}

#[test]
fn a_hidden_workspace_is_shown_with_the_gaps_saved_meanwhile() {
    let mut runtime = pair(Config::default());
    let hidden = window(30);
    let second = WorkspaceId::new(2).unwrap();
    runtime.backend.frames.insert(hidden.clone(), frame(60));
    bind(&mut runtime, second, &hidden);

    runtime.replace_config(gaps(7, 3));
    runtime
        .switch(second, outputs(), &AtomicBool::new(false))
        .unwrap();

    assert_eq!(runtime.backend.frames[&hidden], rect(3, 3, 994, 794));
}

#[test]
fn fullscreen_layout_and_floating_windows_ignore_gaps() {
    let mut runtime = pair(gaps(8, 8));
    let cancelled = AtomicBool::new(false);
    assert_eq!(shown(&runtime), GAPPED);
    let floating = window(30);
    runtime.backend.frames.insert(floating.clone(), frame(120));
    runtime
        .workspaces
        .own_floating(floating.clone(), WorkspaceId::ONE);

    runtime
        .toggle_fullscreen_layout(outputs(), &cancelled)
        .unwrap();

    let display = outputs().main().frame;
    assert_eq!(shown(&runtime), [display, display]);
    assert_eq!(runtime.backend.frames[&floating], frame(120));

    runtime
        .toggle_fullscreen_layout(outputs(), &cancelled)
        .unwrap();
    assert_eq!(shown(&runtime), GAPPED);
    assert_eq!(runtime.backend.frames[&floating], frame(120));
}

#[test]
fn keyboard_resize_and_balance_keep_the_gap() {
    let mut runtime = pair(gaps(8, 8));
    let cancelled = AtomicBool::new(false);

    runtime
        .resize_focused(ResizeDirection::Grow, outputs(), &cancelled)
        .unwrap();
    assert_eq!(
        shown(&runtime),
        [rect(8, 8, 538, 784), rect(554, 8, 438, 784)]
    );

    runtime.balance_focused(outputs(), &cancelled).unwrap();
    assert_eq!(shown(&runtime), GAPPED);
}

#[test]
fn a_pointer_resize_follows_the_dragged_edge_and_a_click_changes_nothing() {
    let mut runtime = pair(gaps(8, 8));
    let cancelled = AtomicBool::new(false);
    let layout = runtime.workspaces.state();

    // Released where the layout put it: no weight may move by the gap.
    for preview in [true, false] {
        runtime
            .resize_window(window(10), GAPPED[0], preview, outputs(), &cancelled)
            .unwrap();
    }
    assert_eq!(runtime.workspaces.state(), layout);
    assert_eq!(applied(&runtime), 0);

    let dragged = rect(8, 8, 588, 784);
    runtime.backend.frames.insert(window(10), dragged);
    runtime
        .resize_window(window(10), dragged, false, outputs(), &cancelled)
        .unwrap();
    assert_eq!(shown(&runtime), [dragged, rect(604, 8, 388, 784)]);

    // The window is already where its tile is: the next cycle leaves it.
    runtime.backend.calls.clear();
    runtime.cycle(outputs(), &cancelled).unwrap();
    assert_eq!(applied(&runtime), 0);
    assert_eq!(shown(&runtime), [dragged, rect(604, 8, 388, 784)]);
}

#[test]
fn drops_swap_and_nest_gapped_tiles_and_a_drop_in_the_gap_does_neither() {
    let mut runtime = pair(gaps(8, 8));
    let cancelled = AtomicBool::new(false);
    let order = |runtime: &WorkspaceRuntime<FakeBackend>| {
        runtime.workspaces.engine(WorkspaceId::ONE).bound_windows()
    };

    // x = 500 lies between the tiles, which end at 496 and start at 504.
    runtime
        .drop_window(window(10), (500, 400), false, outputs(), &cancelled)
        .unwrap();
    assert_eq!(order(&runtime), [window(10), window(20)]);
    assert_eq!(shown(&runtime), GAPPED);

    runtime
        .drop_window(window(10), (750, 400), false, outputs(), &cancelled)
        .unwrap();
    assert_eq!(order(&runtime), [window(20), window(10)]);
    assert_eq!(shown(&runtime), [GAPPED[1], GAPPED[0]]);

    // The top quarter of a tile nests the dropped window above it.
    runtime
        .drop_window(window(10), (250, 20), false, outputs(), &cancelled)
        .unwrap();
    assert_eq!(order(&runtime), [window(10), window(20)]);
    assert_eq!(
        shown(&runtime),
        [rect(8, 8, 984, 388), rect(8, 404, 984, 388)]
    );
}

#[test]
fn a_refused_tile_width_grows_the_tile_and_keeps_the_gaps() {
    let ids = [window(10), window(11), window(12)];
    let mut backend = FakeBackend::default();
    for id in &ids {
        backend.frames.insert(id.clone(), frame(0));
    }
    backend.minimum_widths.insert(ids[1].clone(), 400);
    let mut runtime = runtime_with_config(backend, gaps(8, 8));
    for id in &ids {
        bind(&mut runtime, WorkspaceId::ONE, id);
    }
    let output = outputs().main().frame;
    let cancelled = AtomicBool::new(false);

    runtime.layout_workspace(WorkspaceId::ONE, output, &cancelled);

    let [left, middle, right] = ids.each_ref().map(|id| runtime.backend.frames[id]);
    assert_eq!(middle.width, 400);
    assert_eq!(left.x, 8);
    assert_eq!(middle.x, left.x + left.width + 8);
    assert_eq!(right.x, middle.x + middle.width + 8);
    assert_eq!(right.x + right.width, 992);
    assert!(runtime.constrained[WorkspaceId::ONE.index()].is_empty());

    // Converged: the same frames are asked for again and nothing is refused.
    let settled = runtime.backend.frames.clone();
    runtime.layout_workspace(WorkspaceId::ONE, output, &cancelled);
    assert_eq!(runtime.backend.frames, settled);
    assert!(runtime.constrained[WorkspaceId::ONE.index()].is_empty());
}

#[test]
fn neighbors_too_wide_for_a_gapped_row_stack_with_the_gap_between_them() {
    let ids = [window(10), window(11)];
    let mut backend = FakeBackend::default();
    for id in &ids {
        backend.frames.insert(id.clone(), frame(0));
        // The pair fits 1000 points only while nothing separates the tiles.
        backend.minimum_widths.insert(id.clone(), 495);
    }
    let mut runtime = runtime_with_config(backend, gaps(8, 8));
    for id in &ids {
        bind(&mut runtime, WorkspaceId::ONE, id);
    }
    let saved = runtime.workspaces.state();
    let cancelled = AtomicBool::new(false);

    for _ in 0..2 {
        runtime.layout_workspace(WorkspaceId::ONE, outputs().main().frame, &cancelled);
    }

    let [top, bottom] = ids.each_ref().map(|id| runtime.backend.frames[id]);
    assert_eq!(top, rect(8, 8, 984, 388));
    assert_eq!(bottom, rect(8, 404, 984, 388));
    assert_eq!(runtime.workspaces.state(), saved);
}

/// What the arrangement check would report: kind, window and expected frame.
fn anomalies(
    runtime: &mut WorkspaceRuntime<FakeBackend>,
) -> Vec<(AnomalyKind, u32, Option<crate::Rect>)> {
    let outputs = outputs();
    let visible = runtime.visible_workspaces(&outputs);
    let server = runtime.backend.window_server_frames().unwrap();
    let windows = runtime.arranged_windows(&server);
    runtime
        .anomalies(&outputs, &visible, &windows, &server)
        .into_iter()
        .map(|anomaly| (anomaly.kind, anomaly.window, anomaly.expected))
        .collect()
}

#[test]
fn the_journal_expects_tiles_at_their_gapped_frames() {
    // Wider than the journal's misplacement tolerance.
    let mut runtime = pair(gaps(40, 40));
    assert_eq!(anomalies(&mut runtime), []);

    // A window left where it would sit without gaps is the misplaced one.
    runtime.backend.frames.insert(window(10), TOUCHING[0]);
    assert_eq!(
        anomalies(&mut runtime),
        [(AnomalyKind::Misplaced, 10, Some(rect(40, 40, 440, 720)))]
    );
}

#[test]
fn a_constrained_window_overlaps_its_neighbor_only_past_the_gap() {
    let mut runtime = pair(gaps(8, 8));
    let keeps = |runtime: &mut WorkspaceRuntime<FakeBackend>, width| {
        let kept = crate::Rect { width, ..GAPPED[0] };
        runtime.backend.frames.insert(window(10), kept);
        runtime.constrained[WorkspaceId::ONE.index()].insert(window(10), (GAPPED[0], kept));
        anomalies(runtime)
    };

    // Six points wider than its tile, the window still ends inside the gap.
    assert_eq!(keeps(&mut runtime, 494), []);
    assert_eq!(
        keeps(&mut runtime, 520),
        [(AnomalyKind::ConstrainedOverlap, 10, Some(GAPPED[0]))]
    );
}

#[test]
fn directional_focus_and_reorder_cross_wide_gaps() {
    let mut runtime = pair(gaps(120, 60));
    let cancelled = AtomicBool::new(false);
    let beyond = window(30);
    runtime.backend.frames.insert(beyond.clone(), frame(1050));
    bind(&mut runtime, WorkspaceId::SIX, &beyond);

    for expected in [window(20), beyond.clone()] {
        runtime
            .focus_relative(Direction::Right, outputs(), &cancelled)
            .unwrap();
        assert_eq!(runtime.backend.focused, Some(expected));
    }
    for expected in [window(20), window(10)] {
        runtime
            .focus_relative(Direction::Left, outputs(), &cancelled)
            .unwrap();
        assert_eq!(runtime.backend.focused, Some(expected));
    }

    runtime
        .reorder_focused(Direction::Right, outputs(), &cancelled)
        .unwrap();
    assert_eq!(
        runtime.workspaces.engine(WorkspaceId::ONE).bound_windows(),
        [window(20), window(10)]
    );
}

#[test]
fn workspace_previews_look_the_same_with_and_without_gaps() {
    let previews = |config| {
        let mut runtime = pair(config);
        let status = runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();
        status
            .layouts
            .into_iter()
            .map(|layout| layout.mode)
            .collect::<Vec<_>>()
    };

    let gapped = previews(gaps(64, 64));

    assert_eq!(gapped, previews(Config::default()));
    assert!(matches!(&gapped[0], UiHudLayout::Split(tiles) if tiles.len() == 2));
}
