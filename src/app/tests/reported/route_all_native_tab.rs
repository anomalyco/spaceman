//! #509: with `[labs] route_all_windows` on, a `windows = "all"` route rule sent an app's incoming
//! native tab to the rule's workspace, so the tile it replaced vanished from the workspace the
//! user had moved it to. Reported by @coygeek.

use super::super::*;

const RULE: &str = "version = 1\n[labs]\nroute_all_windows = true\n[[rules]]\nbundle_id = 'com.example.Test'\naction = 'route'\nworkspace = 6\nwindows = 'all'";

/// A runtime whose workspace 1 tiles `tab` after one settled cycle, as if moved there by hand.
fn tiled_on_one(tab: &crate::PhysicalWindowId) -> (WorkspaceRuntime<FakeBackend>, crate::Rect) {
    let mut backend = FakeBackend {
        observation: Some(observation(tab, frame(50))),
        focused: Some(tab.clone()),
        ..FakeBackend::default()
    };
    backend.frames.insert(tab.clone(), frame(50));
    let mut runtime = runtime_with_config(backend, Config::parse(RULE).unwrap());
    bind(&mut runtime, WorkspaceId::ONE, tab);
    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();
    let accepted = runtime.backend.frames[tab];
    runtime.backend.observation = Some(observation(tab, accepted));
    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();
    assert_eq!(runtime.workspaces.workspace_of(tab), Some(WorkspaceId::ONE));
    (runtime, accepted)
}

#[test]
fn an_incoming_native_tab_keeps_its_slot_under_a_route_all_rule() {
    let (previous, replacement) = (window(10), window(11));
    let (mut runtime, accepted) = tiled_on_one(&previous);
    let slot = runtime
        .workspaces
        .engine(WorkspaceId::ONE)
        .slot_for_window(&previous);
    runtime.backend.frames.remove(&previous);
    runtime.backend.frames.insert(replacement.clone(), accepted);
    runtime.backend.focused = Some(replacement.clone());
    runtime.backend.observation = Some(observation(&replacement, accepted));

    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();

    assert_eq!(
        runtime.workspaces.workspace_of(&replacement),
        Some(WorkspaceId::ONE)
    );
    assert_eq!(
        runtime
            .workspaces
            .engine(WorkspaceId::ONE)
            .slot_for_window(&replacement),
        slot
    );
    assert!(!runtime.parked.contains_key(&replacement));
    assert_eq!(runtime.backend.frames[&replacement], accepted);
}

#[test]
fn a_new_window_beside_the_tile_still_follows_the_route_all_rule() {
    let (tiled, opened) = (window(10), window(11));
    let (mut runtime, accepted) = tiled_on_one(&tiled);
    runtime.backend.frames.insert(opened.clone(), frame(100));
    runtime.backend.focused = Some(opened.clone());
    runtime.backend.observation = Some(observation_for_windows(
        "com.example.Test",
        &opened,
        &[(tiled.clone(), accepted), (opened.clone(), frame(100))],
    ));

    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();

    assert_eq!(
        runtime.workspaces.workspace_of(&tiled),
        Some(WorkspaceId::ONE)
    );
    assert_eq!(
        runtime.workspaces.workspace_of(&opened),
        Some(WorkspaceId::SIX)
    );
}

/// #598: an ignored window from the same app, left open on the tile's display, is no new
/// window to that workspace, so the incoming tab still takes its tile's slot. Reported by
/// @coygeek.
#[test]
fn an_incoming_native_tab_keeps_its_slot_beside_an_ignored_window_of_its_app() {
    const RULES: &str = "version = 1\n[labs]\nroute_all_windows = true\n[[rules]]\nbundle_id = 'com.example.Test'\ntitle = 'Ignored'\naction = 'ignore'\n[[rules]]\nbundle_id = 'com.example.Test'\naction = 'route'\nworkspace = 6\nwindows = 'all'";
    let (previous, replacement, ignored) = (window(10), window(11), window(12));
    let mut backend = FakeBackend {
        observation: Some(observation(&previous, frame(50))),
        focused: Some(previous.clone()),
        ..FakeBackend::default()
    };
    backend.frames.insert(previous.clone(), frame(50));
    let mut runtime = runtime_with_config(backend, Config::parse(RULES).unwrap());
    bind(&mut runtime, WorkspaceId::ONE, &previous);
    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();
    let accepted = runtime.backend.frames[&previous];
    runtime.backend.observation = Some(observation(&previous, accepted));
    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();
    let slot = runtime
        .workspaces
        .engine(WorkspaceId::ONE)
        .slot_for_window(&previous);

    runtime.backend.frames.remove(&previous);
    runtime.backend.frames.insert(replacement.clone(), accepted);
    runtime.backend.frames.insert(ignored.clone(), frame(100));
    runtime.backend.focused = Some(replacement.clone());
    let mut tabbed = observation_for_windows(
        "com.example.Test",
        &replacement,
        &[
            (replacement.clone(), accepted),
            (ignored.clone(), frame(100)),
        ],
    );
    let crate::AppObservation::Complete { windows, .. } = &mut tabbed.applications[0] else {
        unreachable!();
    };
    windows[1].title = Some("Ignored".into());
    runtime.backend.observation = Some(tabbed);

    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();

    assert_eq!(
        runtime.workspaces.workspace_of(&replacement),
        Some(WorkspaceId::ONE)
    );
    assert_eq!(
        runtime
            .workspaces
            .engine(WorkspaceId::ONE)
            .slot_for_window(&replacement),
        slot
    );
    assert!(!runtime.parked.contains_key(&replacement));
    assert_eq!(runtime.backend.frames[&replacement], accepted);
    assert_eq!(runtime.workspaces.workspace_of_any(&ignored), None);
    assert_eq!(runtime.backend.frames[&ignored], frame(100));
}
