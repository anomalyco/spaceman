use super::*;

#[test]
fn first_run_setup_tour_holds_tiling_until_it_finishes() {
    // A fresh install opens the setup tour before Accessibility is granted.
    let mut startup = TilingStartup::new(true);

    // The grant is noticed while the user is still reading the tour: hold, and
    // keep holding on every later tick, so nothing retiles behind the window.
    assert_eq!(startup.accessibility_granted(), TilingStart::AfterSetup);
    assert_eq!(startup.accessibility_granted(), TilingStart::AfterSetup);

    // Finishing or dismissing the tour starts tiling, and it stays started.
    assert_eq!(startup.setup_finished(), TilingStart::Now);
    assert_eq!(startup.accessibility_granted(), TilingStart::Now);
}

#[test]
fn manual_tour_reopen_does_not_hold_tiling() {
    // Re-running the tour from Settings never pauses a running install.
    let mut startup = TilingStartup::new(false);
    assert_eq!(startup.accessibility_granted(), TilingStart::Now);
    assert_eq!(startup.setup_finished(), TilingStart::Now);
}

#[test]
fn mouse_gesture_requires_position_change_without_resize() {
    let start = crate::Rect {
        x: 100,
        y: 100,
        width: 800,
        height: 600,
    };
    assert!(!is_window_drag(start, start));
    assert!(is_window_drag(
        start,
        crate::Rect {
            x: 120,
            y: 110,
            ..start
        }
    ));
    assert!(!is_window_drag(
        start,
        crate::Rect {
            x: 120,
            width: 780,
            ..start
        }
    ));
    assert!(is_window_resize(
        start,
        crate::Rect {
            width: 760,
            ..start
        }
    ));
}

#[test]
fn management_scope_rejects_unknown_values() {
    assert!(ManagementScope::parse("secondary").is_ok());
    assert!(ManagementScope::parse("typo").is_err());
}

#[test]
fn activation_settling_defers_refresh_without_an_ax_event() {
    assert!(!refresh_due(false, false, true, false));
    assert!(refresh_due(false, false, true, true));
}

#[test]
fn fallback_retile_backs_off_once_idle() {
    assert_eq!(
        fallback_refresh_interval(Duration::ZERO),
        FALLBACK_REFRESH_INTERVAL
    );
    assert_eq!(
        fallback_refresh_interval(RECENT_ACTIVITY - Duration::from_millis(1)),
        FALLBACK_REFRESH_INTERVAL
    );
    assert_eq!(
        fallback_refresh_interval(RECENT_ACTIVITY),
        IDLE_FALLBACK_REFRESH_INTERVAL
    );
}

#[test]
fn window_creation_bypasses_activation_settling() {
    assert!(refresh_due(true, true, false, false));
    assert!(!refresh_due(false, true, false, false));
}

#[test]
fn invalid_startup_metadata_falls_back_so_parked_recovery_can_start() {
    let (config, workspace_state) =
        recover_startup_metadata(Err("bad config".into()), Err("bad state".into()));

    assert!(!config.has_rules());
    assert!(workspace_state.is_none());

    let valid_state = Workspaces::default().state();
    let (_, workspace_state) =
        recover_startup_metadata(Err("bad config".into()), Ok(Some(valid_state)));
    assert!(workspace_state.is_none());
}

#[test]
fn parked_state_round_trips_and_rejects_malformed_data() {
    let window = window(10);
    let mut parked = HashMap::new();
    parked.insert(window.clone(), frame(50));
    let encoded = serde_json::to_vec(&ParkedWindows(parked.clone())).unwrap();

    assert_eq!(
        serde_json::from_slice::<ParkedWindows>(&encoded).unwrap().0,
        parked
    );
    assert!(serde_json::from_slice::<ParkedWindows>(b"not json").is_err());
    let duplicate =
        serde_json::to_vec(&vec![(window.clone(), frame(50)), (window, frame(100))]).unwrap();
    assert!(serde_json::from_slice::<ParkedWindows>(&duplicate).is_err());
}
