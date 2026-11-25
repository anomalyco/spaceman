//! #526: Option+Tab from a focused pin counts the workspace its display shows,
//! never the pin's hidden home. Reported by @coygeek.

use super::super::*;

fn ws(n: u8) -> WorkspaceId {
    WorkspaceId::new(n).unwrap()
}

fn win(n: u32) -> crate::PhysicalWindowId {
    let mut window = window(n);
    window.app.pid = n as i32;
    window
}

fn rect(x: i32, y: i32, width: i32, height: i32) -> crate::Rect {
    crate::Rect {
        x,
        y,
        width,
        height,
    }
}

fn side_by_side() -> macos::DisplayOutputs {
    let frames = [
        rect(0, 30, 2560, 1410),
        rect(-1440, -583, 1440, 2530),
        rect(2560, -539, 1440, 2530),
    ];
    let mut displays = three_outputs().displays().to_vec();
    for (display, frame) in displays.iter_mut().zip(frames) {
        display.frame = frame;
    }
    let bounds = frames.map(|frame| rect(frame.x, frame.y - 30, frame.width, frame.height + 30));
    macos::DisplayOutputs::with_bounds(displays, bounds.to_vec()).unwrap()
}

fn execute(
    rt: &mut WorkspaceRuntime<FakeBackend>,
    displays: &macos::DisplayOutputs,
    action: WorkerAction,
) {
    let Some(WorkerResult::Status { result, .. }) = rt.execute(
        WorkerCommand::Action(action, displays.clone()),
        &AtomicBool::new(false),
    ) else {
        panic!("missing worker status");
    };
    assert_eq!(result.unwrap().failed, 0);
}

fn report(rt: &mut WorkspaceRuntime<FakeBackend>, id: u32) {
    rt.backend.focused = Some(win(id));
    rt.backend.frontmost = Some(win(id).app);
}

fn visible(rt: &WorkspaceRuntime<FakeBackend>, displays: &macos::DisplayOutputs) -> Vec<u8> {
    let mut shown: Vec<u8> = rt
        .visible_workspaces(displays)
        .iter()
        .filter_map(|resolved| resolved.workspace.number())
        .collect();
    shown.sort_unstable();
    shown
}

#[test]
fn back_and_forth_from_a_pin_skips_its_hidden_home() {
    let displays = side_by_side();
    let config = Config::parse(
        "version = 1\n[settings]\nlayout = 'columns'\n[[workspaces]]\nworkspace = 9\nmonitor = 'third-id'\n",
    )
    .unwrap();
    let mut rt = runtime_with_config(FakeBackend::default(), config);
    let mut applications = Vec::new();
    // Pin 70 lives on workspace 7; workspaces 6 and 7 share the left display.
    for (id, workspace) in [(10, 1), (60, 6), (61, 6), (90, 9), (70, 7)] {
        let window = win(id);
        let display = match workspace {
            6 | 7 => 1,
            9 => 2,
            _ => 0,
        };
        let output = displays.displays()[display].frame;
        let frame = rect(output.x + 200, output.y + 800, 500, 340);
        rt.backend.frames.insert(window.clone(), frame);
        if id == 70 {
            rt.workspaces
                .float(window.clone(), ws(7), crate::workspace::FloatCause::Manual);
            assert!(rt.workspaces.pin(window.clone()));
        } else {
            bind(&mut rt, ws(workspace), &window);
        }
        applications
            .extend(observation_for(&format!("com.example.Test{id}"), &window, frame).applications);
    }
    rt.backend.observation = Some(crate::NativeObservation {
        cycle: 0,
        mouse_down: false,
        applications,
    });
    report(&mut rt, 10);
    execute(&mut rt, &displays, WorkerAction::Retile);

    for workspace in [7, 1, 6] {
        execute(&mut rt, &displays, WorkerAction::SwitchWorkspace(ws(workspace)));
    }
    assert_eq!(visible(&rt, &displays), [1, 6, 9]);

    // The pin, focused on its display while 6 shows there.
    report(&mut rt, 70);
    execute(&mut rt, &displays, WorkerAction::Retile);
    assert_eq!(visible(&rt, &displays), [1, 6, 9]);

    execute(&mut rt, &displays, WorkerAction::WorkspaceBackAndForth);
    assert_eq!(rt.backend.focused, Some(win(10)), "first press goes to 1");
    assert_eq!(visible(&rt, &displays), [1, 6, 9]);

    execute(&mut rt, &displays, WorkerAction::WorkspaceBackAndForth);
    let focused = rt.backend.focused.as_ref().map(|window| window.window_id);
    assert!(
        matches!(focused, Some(60 | 61)),
        "second press returns to 6, got {focused:?}"
    );
    assert_eq!(visible(&rt, &displays), [1, 6, 9], "7 stays hidden");
    assert!(!rt.parked.contains_key(&win(70)), "the pin stays on screen");
}
