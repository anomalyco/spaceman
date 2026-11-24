//! #202: moving a workspace there and back shows the display's previous
//! workspace even when two selectors (a UUID and `secondary`) name it.
use super::*;

fn nine_returns(nine: &str) -> bool {
    let ws = |n| WorkspaceId::new(n).unwrap();
    let displays = three_outputs().displays().to_vec();
    let without_secondary =
        macos::DisplayOutputs::new(vec![displays[0].clone(), displays[2].clone()]).unwrap();
    let config = Config::parse(&format!(
        "version = 1\n[[workspaces]]\nworkspace = 9\nmonitor = '{nine}'"
    ))
    .unwrap();
    let windows = [
        (1, window(10), 50),
        (6, window(60), 1050),
        (9, window(90), 2050),
    ];
    let focused = windows[0].1.clone();
    let mut backend = FakeBackend {
        focused: Some(focused.clone()),
        frontmost: Some(focused.app),
        ..FakeBackend::default()
    };
    for (_, w, x) in &windows {
        backend.frames.insert(w.clone(), frame(*x));
    }
    let mut rt = runtime_with_config(backend, config);
    for (n, w, _) in &windows {
        bind(&mut rt, ws(*n), w);
    }
    let cancelled = AtomicBool::new(false);
    for outputs in [three_outputs(), without_secondary.clone()] {
        for n in [6, 9, 1] {
            rt.switch(ws(n), outputs.clone(), &cancelled).unwrap();
        }
    }
    assert!(rt.is_visible(ws(9), &without_secondary));
    for _ in 0..2 {
        rt.move_workspace_to_display(DisplayTarget::Next, without_secondary.clone(), &cancelled)
            .unwrap();
    }
    assert!(rt.is_visible(ws(1), &without_secondary));
    rt.is_visible(ws(9), &without_secondary)
}

#[test]
fn previous_workspace_returns_when_assigned_by_uuid() {
    assert!(nine_returns("third-id"));
}

#[test]
fn control_previous_workspace_returns_when_assigned_as_secondary() {
    assert!(nine_returns("secondary"));
}
