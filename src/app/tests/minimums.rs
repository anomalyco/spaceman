use super::*;

fn tiled(windows: &[(crate::PhysicalWindowId, i32)]) -> WorkspaceRuntime<FakeBackend> {
    let mut backend = FakeBackend::default();
    for (window, minimum) in windows {
        backend.frames.insert(window.clone(), frame(0));
        if *minimum > 0 {
            backend.minimum_widths.insert(window.clone(), *minimum);
        }
    }
    let mut runtime = runtime(backend);
    for (window, _) in windows {
        bind(&mut runtime, WorkspaceId::ONE, window);
    }
    runtime
}

fn tile_frames(runtime: &WorkspaceRuntime<FakeBackend>) -> Vec<crate::Rect> {
    runtime
        .workspaces
        .engine(WorkspaceId::ONE)
        .placements(&Settings::for_output(outputs().main().frame))
        .into_iter()
        .map(|placement| placement.frame)
        .collect()
}

#[test]
fn a_window_that_refuses_its_tile_width_gets_a_wider_tile_from_its_neighbor() {
    let (left, middle, right) = (window(10), window(11), window(12));
    let mut runtime = tiled(&[(left.clone(), 0), (middle.clone(), 400), (right.clone(), 0)]);
    let output = outputs().main().frame;

    runtime.layout_workspace(WorkspaceId::ONE, output, &AtomicBool::new(false));

    let frames = tile_frames(&runtime);
    assert_eq!(frames[1].width, 400);
    assert_eq!(frames[0].width, 333);
    assert_eq!(frames[1].x, frames[0].x + frames[0].width);
    assert_eq!(frames[2].x, frames[1].x + frames[1].width);
    assert_eq!(frames[2].x + frames[2].width, output.width);
    assert_eq!(runtime.backend.frames[&middle], frames[1]);
    assert!(!runtime.constrained[WorkspaceId::ONE.index()].contains_key(&middle));
}

#[test]
fn an_observation_cycle_fits_a_minimum_width_in_the_same_pass() {
    let (left, right) = (window(10), window(11));
    let mut backend = FakeBackend {
        observation: Some(observation_for_windows(
            "com.example.Test",
            &left,
            &[(left.clone(), frame(0)), (right.clone(), frame(500))],
        )),
        ..FakeBackend::default()
    };
    backend.frames.insert(left.clone(), frame(0));
    backend.frames.insert(right.clone(), frame(500));
    backend.minimum_widths.insert(left.clone(), 640);
    let mut runtime = runtime(backend);

    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();

    let frames = tile_frames(&runtime);
    assert_eq!(frames[0].width, 640);
    assert_eq!(runtime.backend.frames[&right], frames[1]);
    assert_eq!(frames[1].x, 640);
}

#[test]
fn the_last_tile_grows_toward_its_leading_neighbor() {
    let (left, right) = (window(10), window(11));
    let mut runtime = tiled(&[(left.clone(), 0), (right.clone(), 700)]);
    let output = outputs().main().frame;

    runtime.layout_workspace(WorkspaceId::ONE, output, &AtomicBool::new(false));

    let frames = tile_frames(&runtime);
    assert_eq!(frames[0].width, 300);
    assert_eq!(frames[1].x, 300);
    assert_eq!(frames[1].width, 700);
    assert_eq!(runtime.backend.frames[&right], frames[1]);
}

#[test]
fn neighbors_whose_minimums_cannot_both_fit_do_not_trade_space_forever() {
    let (left, right) = (window(10), window(11));
    let mut runtime = tiled(&[(left.clone(), 600), (right.clone(), 600)]);
    let output = outputs().main().frame;
    let cancelled = AtomicBool::new(false);

    runtime.layout_workspace(WorkspaceId::ONE, output, &cancelled);
    runtime.layout_workspace(WorkspaceId::ONE, output, &cancelled);
    let settled = tile_frames(&runtime);
    runtime.layout_workspace(WorkspaceId::ONE, output, &cancelled);
    runtime.layout_workspace(WorkspaceId::ONE, output, &cancelled);

    assert_eq!(tile_frames(&runtime), settled);
    assert_eq!(settled[0].width, 600);
}

#[test]
fn fullscreen_layout_ignores_minimum_widths() {
    let (left, right) = (window(10), window(11));
    let mut runtime = tiled(&[(left.clone(), 0), (right.clone(), 600)]);
    runtime
        .workspaces
        .set_fullscreen_layout(WorkspaceId::ONE, true);
    let before = tile_frames(&runtime);

    runtime.layout_workspace(
        WorkspaceId::ONE,
        outputs().main().frame,
        &AtomicBool::new(false),
    );

    assert_eq!(tile_frames(&runtime), before);
}
