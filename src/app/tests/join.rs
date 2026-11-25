//! The Join Lab (`spaceman ctl join DIR`): off by default, and when on it nests the focused
//! tile with its neighbor instead of swapping with it.

use super::*;

/// Windows 10, 20 and 30 side by side on Workspace 1, with window 30 focused.
fn three_columns(config: Config) -> WorkspaceRuntime<FakeBackend> {
    let mut backend = FakeBackend {
        focused: Some(window(30)),
        ..FakeBackend::default()
    };
    for (id, x) in [(10, 50), (20, 400), (30, 750)] {
        backend.frames.insert(window(id), frame(x));
    }
    let mut runtime = runtime_with_config(backend, config);
    for id in [10, 20, 30] {
        bind(&mut runtime, WorkspaceId::ONE, &window(id));
    }
    runtime
}

fn frames(runtime: &WorkspaceRuntime<FakeBackend>) -> Vec<crate::Rect> {
    runtime
        .workspaces
        .engine(WorkspaceId::ONE)
        .placements(&Settings::for_output(outputs().main().frame))
        .into_iter()
        .map(|placement| placement.frame)
        .collect()
}

#[test]
fn join_is_refused_while_the_lab_is_off() {
    let mut runtime = three_columns(Config::default());
    let before = frames(&runtime);

    let Err(error) = runtime.join_focused(Direction::Left, outputs(), &AtomicBool::new(false))
    else {
        panic!("join ran with the Lab off");
    };

    assert!(error.contains("[labs] join"), "{error}");
    assert_eq!(frames(&runtime), before);
}

#[test]
fn join_left_stacks_the_focused_tile_under_its_neighbor() {
    let lab = Config::parse("version = 1\n[labs]\njoin = true\n").unwrap();
    let mut runtime = three_columns(lab);

    runtime
        .join_focused(Direction::Left, outputs(), &AtomicBool::new(false))
        .unwrap();

    let rect = |x, y, width, height| crate::Rect {
        x,
        y,
        width,
        height,
    };
    // | 10 | 20/30 |
    assert_eq!(
        frames(&runtime),
        vec![
            rect(0, 0, 500, 800),
            rect(500, 0, 500, 400),
            rect(500, 400, 500, 400),
        ]
    );
    assert_eq!(
        runtime.workspaces.engine(WorkspaceId::ONE).bound_windows(),
        vec![window(10), window(20), window(30)]
    );
}
