//! The Center Floats Lab: Toggle Float centres the window, and `resize` sizes a focused float.

use super::*;

fn lab_runtime() -> WorkspaceRuntime<FakeBackend> {
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
    let config = Config::parse("version = 1\n[labs]\ncenter_floats = true\n").unwrap();
    let mut runtime = runtime_with_config(backend, config);
    bind(&mut runtime, WorkspaceId::ONE, &window(10));
    bind(&mut runtime, WorkspaceId::ONE, &window(20));
    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();
    runtime.backend.calls.clear();
    runtime
}

#[test]
fn floating_a_tile_centres_it_at_three_quarters_of_its_display() {
    let mut runtime = lab_runtime();
    let cancelled = AtomicBool::new(false);

    let status = runtime.toggle_float(outputs(), &cancelled).unwrap();

    assert_eq!(status.failed, 0);
    assert!(runtime.workspaces.is_manual_float(&window(10)));
    // The main display is 1000 x 800 at the origin.
    let centred = crate::Rect {
        x: 125,
        y: 100,
        width: 750,
        height: 600,
    };
    assert_eq!(runtime.backend.frames[&window(10)], centred);
    assert_eq!(runtime.backend.frames[&window(20)], outputs().main().frame);
}

#[test]
fn resize_keys_size_a_focused_float_around_its_centre() {
    let mut runtime = lab_runtime();
    let cancelled = AtomicBool::new(false);
    runtime.toggle_float(outputs(), &cancelled).unwrap();

    runtime
        .resize_focused(ResizeDirection::Grow, outputs(), &cancelled)
        .unwrap();
    assert_eq!(
        runtime.backend.frames[&window(10)],
        crate::Rect {
            x: 100,
            y: 75,
            width: 800,
            height: 650,
        }
    );

    runtime
        .resize_focused_to(crate::Axis::Horizontal, 600, outputs(), &cancelled)
        .unwrap();
    assert_eq!(
        runtime.backend.frames[&window(10)],
        crate::Rect {
            x: 200,
            y: 75,
            width: 600,
            height: 650,
        }
    );
    // The tile beside it is untouched.
    assert_eq!(runtime.backend.frames[&window(20)], outputs().main().frame);
}
