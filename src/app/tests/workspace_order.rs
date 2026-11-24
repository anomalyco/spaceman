//! `settings.workspace_order`: the HUD, the menu bar list and three-finger swipes list a
//! display's workspaces in the configured order, then any others in the default order.

use super::*;

fn named(letter: char) -> WorkspaceId {
    WorkspaceId::named(letter).unwrap()
}

fn config(order: &str) -> Config {
    Config::parse(&format!(
        r#"
version = 1
[settings]
{order}
[[workspaces]]
workspace = "Q"
monitor = "main"
[[workspaces]]
workspace = "W"
monitor = "main"
[[workspaces]]
workspace = "E"
monitor = "main"
"#
    ))
    .unwrap()
}

/// Workspaces 1, Q, W and E on the main display, each holding one window; 1 is visible.
fn keyboard_row(order: &str) -> WorkspaceRuntime<FakeBackend> {
    let mut runtime = runtime_with_config(FakeBackend::default(), config(order));
    for (id, workspace) in [
        (10, WorkspaceId::ONE),
        (11, named('Q')),
        (12, named('W')),
        (13, named('E')),
    ] {
        let window = window(id);
        runtime.backend.frames.insert(window.clone(), frame(50));
        bind(&mut runtime, workspace, &window);
    }
    runtime
}

fn listed(runtime: &WorkspaceRuntime<FakeBackend>) -> Vec<WorkspaceId> {
    runtime.listed_workspaces("main-id", Some(WorkspaceId::ONE), &outputs())
}

#[test]
fn without_an_order_named_workspaces_list_alphabetically_after_the_numbered_ones() {
    let runtime = keyboard_row("");

    assert_eq!(
        listed(&runtime),
        [WorkspaceId::ONE, named('E'), named('Q'), named('W')]
    );
}

#[test]
fn listed_workspaces_come_first_in_the_configured_order_and_the_rest_follow() {
    let runtime = keyboard_row(r#"workspace_order = ["Q", "W", "E"]"#);

    assert_eq!(
        listed(&runtime),
        [named('Q'), named('W'), named('E'), WorkspaceId::ONE]
    );
}

#[test]
fn an_order_entry_neither_enables_a_workspace_nor_lists_an_empty_one() {
    let runtime = keyboard_row(r#"workspace_order = ["P", "Q", 4, "W", "E", 1]"#);

    assert!(!runtime.workspaces.is_enabled(named('P')));
    assert_eq!(
        listed(&runtime),
        [named('Q'), named('W'), named('E'), WorkspaceId::ONE]
    );
}

#[test]
fn the_hud_row_follows_the_configured_order() {
    let mut runtime = keyboard_row(r#"workspace_order = [1, "Q", "W", "E"]"#);

    let hud = runtime
        .switch(named('W'), outputs(), &AtomicBool::new(false))
        .unwrap()
        .hud
        .unwrap();
    let main = hud
        .displays
        .iter()
        .find(|display| display.id == "main-id")
        .unwrap();

    assert_eq!(main.visible, Some(named('W')));
    assert_eq!(
        main.workspaces
            .iter()
            .map(|entry| entry.workspace)
            .collect::<Vec<_>>(),
        [WorkspaceId::ONE, named('Q'), named('W'), named('E')]
    );
}

#[test]
fn the_menu_bar_list_follows_the_configured_order() {
    let runtime = keyboard_row(r#"workspace_order = [1, "Q", "W", "E"]"#);

    let menu = runtime.status(0, &outputs()).menu;
    let main = menu
        .iter()
        .find(|display| display.display == "Main")
        .unwrap();

    assert_eq!(
        main.workspaces
            .iter()
            .map(|entry| entry.workspace)
            .collect::<Vec<_>>(),
        [WorkspaceId::ONE, named('Q'), named('W'), named('E')]
    );
}

#[test]
fn swipes_step_through_workspaces_in_the_configured_order() {
    let mut runtime = keyboard_row(r#"workspace_order = [1, "Q", "W", "E"]"#);
    let mut swipe = |swipe| {
        runtime
            .swipe(swipe, (500, 400), outputs(), &AtomicBool::new(false))
            .unwrap()
            .active[0]
    };

    assert_eq!(swipe(Swipe::Left), named('Q'));
    assert_eq!(swipe(Swipe::Left), named('W'));
    assert_eq!(swipe(Swipe::Left), named('E'));
    assert_eq!(swipe(Swipe::Left), named('E'));
    assert_eq!(swipe(Swipe::Right), named('W'));
}

#[test]
fn a_config_reload_applies_and_clears_the_order() {
    let mut runtime = keyboard_row("");

    runtime.replace_config(config(r#"workspace_order = ["W", "Q", "E"]"#));
    assert_eq!(
        listed(&runtime),
        [named('W'), named('Q'), named('E'), WorkspaceId::ONE]
    );

    runtime.replace_config(config(""));
    assert_eq!(
        listed(&runtime),
        [WorkspaceId::ONE, named('E'), named('Q'), named('W')]
    );
}
