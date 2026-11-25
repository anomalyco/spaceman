//! #423: renaming a workspace's display selector to the same display keeps it in front.
//! Reproduction contributed by @coygeek.

use super::super::*;

fn execute(
    rt: &mut WorkspaceRuntime<FakeBackend>,
    action: WorkerAction,
    outputs: &macos::DisplayOutputs,
) -> Result<serde_json::Value, String> {
    let Some(WorkerResult::Status { result, debug }) = rt.execute(
        WorkerCommand::Action(action, outputs.clone()),
        &AtomicBool::new(false),
    ) else {
        panic!("missing worker status");
    };
    assert_eq!(result?.failed, 0);
    Ok(serde_json::to_value(debug).unwrap())
}

fn portrait_displays() -> macos::DisplayOutputs {
    let mut displays = three_outputs().displays().to_vec();
    for (display, (x, y, width, height)) in displays.iter_mut().zip([
        (0, 30, 2560, 1410),
        (-1440, -583, 1440, 2530),
        (2560, -539, 1440, 2530),
    ]) {
        display.frame = crate::Rect {
            x,
            y,
            width,
            height,
        };
    }
    let bounds = displays
        .iter()
        .map(|d| crate::Rect {
            y: d.frame.y - 30,
            height: d.frame.height + 30,
            ..d.frame
        })
        .collect();
    macos::DisplayOutputs::with_bounds(displays, bounds).unwrap()
}

fn configured_rig(
    config: Config,
    displays: &macos::DisplayOutputs,
    tiles: &[(u8, u32, usize)],
) -> WorkspaceRuntime<FakeBackend> {
    let mut applications = Vec::new();
    let mut backend = FakeBackend::default();
    for &(_, id, display) in tiles {
        let window = window(id);
        let output = displays.displays()[display].frame;
        let frame = crate::Rect {
            x: output.x + 50,
            y: output.y + 50,
            width: 400,
            height: 300,
        };
        backend.frames.insert(window.clone(), frame);
    }
    if !tiles.is_empty() {
        let app = crate::ObservedApplication {
            id: window(tiles[0].1).app,
            bundle_id: "com.example.Test".into(),
        };
        applications.push(crate::AppObservation::Complete {
            app,
            focused: Some(window(tiles[0].1)),
            windows: tiles
                .iter()
                .map(|&(_, id, _)| crate::ObservedWindow {
                    id: window(id),
                    frame: backend.frames[&window(id)],
                    title: None,
                    resizable: true,
                })
                .collect(),
        });
        backend.focused = Some(window(tiles[0].1));
        backend.frontmost = Some(window(tiles[0].1).app);
    }
    backend.observation = Some(crate::NativeObservation {
        cycle: 0,
        applications,
        mouse_down: false,
    });
    let mut rt = runtime_with_config(backend, config);
    for &(n, id, _) in tiles {
        bind(&mut rt, WorkspaceId::new(n).unwrap(), &window(id));
    }
    execute(&mut rt, WorkerAction::Retile, displays).unwrap();
    rt
}

fn ws(n: u8) -> WorkspaceId {
    WorkspaceId::new(n).unwrap()
}

fn alias_config(monitor: &str, layout: &str, inner: u32) -> Config {
    let mut source = format!(
        "version = 1\n[settings]\nlayout = '{layout}'\nmouse_follows_focus = 'display'\n[settings.gaps]\ninner = {inner}\n"
    );
    for (n, selector) in [
        (4, "secondary"),
        (5, monitor),
        (6, "secondary-id"),
        (7, "third-id"),
        (8, "third-id"),
        (9, "third-id"),
        (10, "third-id"),
    ] {
        source.push_str(&format!(
            "\n[[workspaces]]\nworkspace = {n}\nmonitor = '{selector}'\n"
        ));
    }
    Config::parse(&source).unwrap()
}

#[test]
fn config_research_alias() {
    let displays = portrait_displays();
    for layout in ["columns", "dwindle"] {
        for (monitor, inner) in [("secondary", 8), ("secondary-id", 0)] {
            let mut rt = configured_rig(
                alias_config("secondary", layout, 0),
                &displays,
                &[(1, 10, 0), (4, 40, 1), (5, 50, 1), (6, 60, 1), (9, 90, 2)],
            );
            for n in [9, 6, 5, 1] {
                execute(&mut rt, WorkerAction::SwitchWorkspace(ws(n)), &displays).unwrap();
            }
            assert!(rt.is_visible(ws(5), &displays));
            assert_eq!(
                displays.resolve(&DisplaySelector::Secondary).id,
                displays.resolve(&DisplaySelector::parse(monitor)).id,
            );
            rt.replace_config(alias_config(monitor, layout, inner));
            let debug = execute(&mut rt, WorkerAction::Retile, &displays).unwrap();
            assert_eq!(
                debug["active_workspaces"],
                serde_json::json!([1, 5, 9]),
                "{layout} monitor={monitor}"
            );
            assert!(!rt.parked.contains_key(&window(50)), "{layout} {monitor}");
            // Workspaces 4 and 6 on that display still switch in place.
            for n in [4, 6] {
                let shown =
                    execute(&mut rt, WorkerAction::SwitchWorkspace(ws(n)), &displays).unwrap();
                assert_eq!(shown["active_workspaces"], serde_json::json!([1, n, 9]));
            }
        }
    }
}
