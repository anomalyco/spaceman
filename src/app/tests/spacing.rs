use super::*;

/// The main display is 1000 × 800 at the origin.
fn spaced() -> Config {
    Config::parse(
        "version = 1\n[settings]\nsingle_window_max_width = 640\n\
         [settings.gaps]\ninner = 10\nleft = 20\ntop = 30\n",
    )
    .unwrap()
}

#[test]
fn layout_applies_gaps_padding_and_the_single_window_width() {
    let (left, right) = (window(10), window(11));
    let mut backend = FakeBackend::default();
    backend.frames.insert(left.clone(), frame(0));
    backend.frames.insert(right.clone(), frame(0));
    let mut runtime = runtime_with_config(backend, spaced());
    let output = outputs().main().frame;
    let cancelled = AtomicBool::new(false);

    bind(&mut runtime, WorkspaceId::ONE, &left);
    runtime.layout_workspace(WorkspaceId::ONE, output, &cancelled);
    let lone = runtime.backend.frames[&left];
    // Usable: x 20..1000, y 30..800; 640 wide centered in its 980.
    assert_eq!(
        lone,
        crate::Rect {
            x: 20 + (980 - 640) / 2,
            y: 30,
            width: 640,
            height: 770,
        }
    );

    bind(&mut runtime, WorkspaceId::ONE, &right);
    runtime.layout_workspace(WorkspaceId::ONE, output, &cancelled);
    let (a, b) = (
        runtime.backend.frames[&left],
        runtime.backend.frames[&right],
    );
    assert_eq!((a.x, a.y, a.height), (20, 30, 770));
    assert_eq!(b.x - (a.x + a.width), 10);
    assert_eq!(b.x + b.width, 1000);
    assert!((a.width - b.width).abs() <= 1);

    runtime
        .workspaces
        .set_fullscreen_layout(WorkspaceId::ONE, true);
    runtime.layout_workspace(WorkspaceId::ONE, output, &cancelled);
    assert_eq!(runtime.backend.frames[&left], lone);
    assert_eq!(runtime.backend.frames[&right], lone);
}

#[test]
fn a_config_without_spacing_tiles_edge_to_edge() {
    let (left, right) = (window(10), window(11));
    let mut backend = FakeBackend::default();
    backend.frames.insert(left.clone(), frame(0));
    backend.frames.insert(right.clone(), frame(0));
    let mut runtime = runtime(backend);
    bind(&mut runtime, WorkspaceId::ONE, &left);
    bind(&mut runtime, WorkspaceId::ONE, &right);
    runtime.layout_workspace(
        WorkspaceId::ONE,
        outputs().main().frame,
        &AtomicBool::new(false),
    );
    let (a, b) = (
        runtime.backend.frames[&left],
        runtime.backend.frames[&right],
    );
    assert_eq!((a.x, a.width, b.x, b.width), (0, 500, 500, 500));
}
