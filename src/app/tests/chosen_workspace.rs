//! #362, Labs › `open_on_chosen_workspace`: after an empty workspace is chosen on another
//! display, the window the user opens next joins it, even when macOS opens it on the display
//! focus is still on. Off by default, and only for that one window.

use super::three_display::{CENTER, LEFT, RIGHT, act, rig_with, seen, win, ws};
use super::*;

const LAB: &str = "[labs]\nopen_on_chosen_workspace = true\n";

/// Where macOS opens the new window: on the center display, which still has focus.
const OPENED: crate::Rect = crate::Rect {
    x: 400,
    y: 200,
    width: 900,
    height: 700,
};

/// Center (workspace 1) has the focused window 10; left shows 6 with 60; right shows 9 with 90,
/// and empty workspace 10 is assigned there.
fn rig(extra: &str) -> WorkspaceRuntime<FakeBackend> {
    rig_with(&[(10, 1, CENTER), (60, 6, LEFT), (90, 9, RIGHT)], extra)
}

/// The app of window 10 opens window 11 (Command+N). `focus` is what the system-wide focus
/// reports; `app_focus` is what the app's own snapshot reports.
fn open_second_window(
    rt: &mut WorkspaceRuntime<FakeBackend>,
    focus: bool,
    app_focus: bool,
) -> crate::PhysicalWindowId {
    let window = crate::PhysicalWindowId {
        app: win(10).app,
        window_id: 11,
    };
    rt.backend.frames.insert(window.clone(), OPENED);
    let observation = rt.backend.observation.as_mut().unwrap();
    for app in &mut observation.applications {
        if let crate::AppObservation::Complete {
            app,
            focused,
            windows,
        } = app
            && app.id == window.app
        {
            windows.push(crate::ObservedWindow {
                id: window.clone(),
                frame: OPENED,
                resizable: true,
                title: None,
            });
            if app_focus {
                *focused = Some(window.clone());
            }
        }
    }
    if focus {
        rt.backend.focused = Some(window.clone());
        rt.backend.frontmost = Some(window.app.clone());
    }
    act(rt, WorkerAction::Retile);
    window
}

#[test]
fn the_next_window_joins_the_empty_workspace_chosen_on_another_display() {
    let mut rt = rig(LAB);
    act(&mut rt, WorkerAction::SwitchWorkspace(ws(10)));
    let center_tile = rt.backend.frames[&win(10)];

    let opened = open_second_window(&mut rt, true, true);

    assert_eq!(rt.workspaces.workspace_of(&opened), Some(ws(10)));
    assert!(RIGHT.contains_center(rt.backend.frames[&opened]));
    assert_eq!(
        rt.backend.frames[&win(10)],
        center_tile,
        "the center's tiles never make room for it"
    );
    assert!(rt.chosen_workspace.is_none(), "only the next window");
}

/// App `id` launches with one window where macOS opens it, focused.
fn launch(rt: &mut WorkspaceRuntime<FakeBackend>, id: u32) -> crate::PhysicalWindowId {
    launch_with(rt, id, true)
}

fn launch_with(
    rt: &mut WorkspaceRuntime<FakeBackend>,
    id: u32,
    resizable: bool,
) -> crate::PhysicalWindowId {
    let launched = win(id);
    rt.backend.frames.insert(launched.clone(), OPENED);
    let mut app = seen(id, OPENED);
    if let crate::AppObservation::Complete { windows, .. } = &mut app {
        windows[0].resizable = resizable;
    }
    rt.backend
        .observation
        .as_mut()
        .unwrap()
        .applications
        .push(app);
    rt.backend.focused = Some(launched.clone());
    rt.backend.frontmost = Some(launched.app.clone());
    act(rt, WorkerAction::Retile);
    launched
}

#[test]
fn a_launched_apps_first_window_joins_it_too() {
    let mut rt = rig(LAB);
    act(&mut rt, WorkerAction::SelectWorkspace(ws(10)));
    let launched = launch(&mut rt, 12);
    assert_eq!(rt.workspaces.workspace_of(&launched), Some(ws(10)));
}

/// A launcher's panel takes focus first; no observation lists it, and the app it launches still
/// gets the chosen workspace.
#[test]
fn a_launcher_panel_on_the_way_keeps_the_choice() {
    let mut rt = rig(LAB);
    act(&mut rt, WorkerAction::SwitchWorkspace(ws(10)));
    let panel = win(50);
    rt.backend.focused = Some(panel.clone());
    rt.backend.frontmost = Some(panel.app.clone());
    act(&mut rt, WorkerAction::Retile);
    assert!(rt.chosen_workspace.is_some());

    let launched = launch(&mut rt, 12);
    assert_eq!(rt.workspaces.workspace_of(&launched), Some(ws(10)));
}

/// A launched app's non-resizable window floats on the chosen workspace, fully on its display.
#[test]
fn a_window_floating_by_its_traits_floats_on_the_chosen_display() {
    let mut rt = rig(LAB);
    act(&mut rt, WorkerAction::SwitchWorkspace(ws(10)));
    let launched = launch_with(&mut rt, 13, false);

    assert_eq!(rt.workspaces.floating_workspace_of(&launched), Some(ws(10)));
    let frame = rt.backend.frames[&launched];
    assert!(
        frame.x >= RIGHT.x
            && frame.y >= RIGHT.y
            && frame.x + frame.width <= RIGHT.x + RIGHT.width
            && frame.y + frame.height <= RIGHT.y + RIGHT.height,
        "{frame:?} is fully on the right display"
    );
}

/// A float rule decides as before, and the app's window does not use up the choice.
#[test]
fn a_float_rule_keeps_the_window_where_it_opened() {
    let mut rt = rig(&format!(
        "{LAB}[[rules]]\nbundle_id = \"com.example.A14\"\naction = \"float\"\n"
    ));
    act(&mut rt, WorkerAction::SwitchWorkspace(ws(10)));
    let floated = launch(&mut rt, 14);
    assert_eq!(rt.workspaces.floating_workspace_of(&floated), Some(ws(1)));
    assert_eq!(rt.backend.frames[&floated], OPENED);
    assert!(rt.chosen_workspace.is_some(), "passed over, not used up");
}

/// An app's window an ignore rule matches is never placed, so it must not count as one of the
/// app's new windows: the app's real window still gets the choice.
#[test]
fn an_ignored_window_does_not_count_as_a_new_window() {
    let mut rt = rig(&format!(
        "{LAB}[[rules]]\nbundle_id = \"com.example.A15\"\ntitle_contains = \"Helper\"\naction = \"ignore\"\n"
    ));
    act(&mut rt, WorkerAction::SwitchWorkspace(ws(10)));
    let main = win(15);
    let helper = crate::PhysicalWindowId {
        app: main.app.clone(),
        window_id: 16,
    };
    let helper_frame = crate::Rect {
        x: 100,
        y: 100,
        width: 300,
        height: 200,
    };
    rt.backend.frames.insert(helper.clone(), helper_frame);
    let mut app = seen(15, OPENED);
    if let crate::AppObservation::Complete { windows, .. } = &mut app {
        windows.push(crate::ObservedWindow {
            id: helper.clone(),
            frame: helper_frame,
            resizable: true,
            title: Some("Helper".into()),
        });
    }
    rt.backend
        .observation
        .as_mut()
        .unwrap()
        .applications
        .push(app);
    // The helper shows first, in the background; the app's window then takes focus.
    act(&mut rt, WorkerAction::Retile);
    assert!(rt.workspaces.workspace_of_any(&helper).is_none());
    rt.backend.frames.insert(main.clone(), OPENED);
    rt.backend.focused = Some(main.clone());
    rt.backend.frontmost = Some(main.app.clone());
    act(&mut rt, WorkerAction::Retile);

    assert_eq!(rt.workspaces.workspace_of(&main), Some(ws(10)));
    assert!(rt.workspaces.workspace_of_any(&helper).is_none());
}

/// A launcher that floats by a rule opens on the chosen display itself (where the pointer is)
/// and floats on the chosen workspace; the app it launches still gets the choice.
#[test]
fn a_launcher_floating_on_the_chosen_display_keeps_the_choice() {
    let mut rt = rig(&format!(
        "{LAB}[[rules]]\nbundle_id = \"com.example.A14\"\naction = \"float\"\n"
    ));
    act(&mut rt, WorkerAction::SwitchWorkspace(ws(10)));
    let on_right = crate::Rect {
        x: RIGHT.x + 200,
        y: RIGHT.y + 300,
        width: 700,
        height: 100,
    };
    let launcher = win(14);
    rt.backend.frames.insert(launcher.clone(), on_right);
    rt.backend
        .observation
        .as_mut()
        .unwrap()
        .applications
        .push(seen(14, on_right));
    rt.backend.focused = Some(launcher.clone());
    rt.backend.frontmost = Some(launcher.app.clone());
    act(&mut rt, WorkerAction::Retile);
    assert_eq!(rt.workspaces.floating_workspace_of(&launcher), Some(ws(10)));
    act(&mut rt, WorkerAction::Retile);
    assert!(
        rt.chosen_workspace.is_some(),
        "the launcher is not the next window"
    );

    let launched = launch(&mut rt, 12);
    assert_eq!(rt.workspaces.workspace_of(&launched), Some(ws(10)));
}

#[test]
fn a_route_rule_still_decides_over_the_chosen_workspace() {
    let mut rt = rig(&format!(
        "{LAB}[[rules]]\nbundle_id = \"com.example.A12\"\naction = \"route\"\nworkspace = 6\n"
    ));
    act(&mut rt, WorkerAction::SwitchWorkspace(ws(10)));
    let launched = launch(&mut rt, 12);
    assert_eq!(rt.workspaces.workspace_of(&launched), Some(ws(6)));
}

/// The app reports its new window focused in the snapshot one cycle before the system-wide
/// focus catches up with it.
#[test]
fn a_focus_report_that_lags_the_new_window_still_claims_it() {
    let mut rt = rig(LAB);
    act(&mut rt, WorkerAction::SwitchWorkspace(ws(10)));
    let opened = open_second_window(&mut rt, false, true);
    assert_eq!(rt.workspaces.workspace_of(&opened), Some(ws(10)));
    assert!(RIGHT.contains_center(rt.backend.frames[&opened]));

    rt.backend.focused = Some(opened.clone());
    act(&mut rt, WorkerAction::Retile);
    assert_eq!(rt.workspaces.workspace_of(&opened), Some(ws(10)));
    assert!(rt.chosen_workspace.is_none(), "only the next window");
}

#[test]
fn with_the_lab_off_the_window_tiles_where_it_opened() {
    let mut rt = rig("");
    act(&mut rt, WorkerAction::SwitchWorkspace(ws(10)));
    let opened = open_second_window(&mut rt, true, true);
    assert_eq!(rt.workspaces.workspace_of(&opened), Some(ws(1)));
}

#[test]
fn focusing_another_window_first_returns_to_the_usual_placement() {
    let mut rt = rig(LAB);
    act(&mut rt, WorkerAction::SwitchWorkspace(ws(10)));
    rt.backend.focused = Some(win(60));
    rt.backend.frontmost = Some(win(60).app);
    act(&mut rt, WorkerAction::Retile);
    assert!(rt.chosen_workspace.is_none());

    rt.backend.focused = Some(win(10));
    rt.backend.frontmost = Some(win(10).app);
    let opened = open_second_window(&mut rt, true, true);
    assert_eq!(rt.workspaces.workspace_of(&opened), Some(ws(1)));
}

#[test]
fn a_window_opened_in_the_background_is_not_moved() {
    let mut rt = rig(LAB);
    act(&mut rt, WorkerAction::SwitchWorkspace(ws(10)));
    let opened = open_second_window(&mut rt, false, false);
    assert_eq!(rt.workspaces.workspace_of(&opened), Some(ws(1)));
}

#[test]
fn a_workspace_with_windows_claims_nothing() {
    let mut rt = rig(LAB);
    act(&mut rt, WorkerAction::SwitchWorkspace(ws(9)));
    assert!(rt.chosen_workspace.is_none());
}
