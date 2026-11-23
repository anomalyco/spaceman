use super::*;

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

#[test]
fn legacy_spaceman_parked_windows_migrate_and_recover_without_losing_malformed_state() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let home = std::env::temp_dir().join(format!(
        "pane-startup-migration-{}-{id}",
        std::process::id()
    ));
    let legacy_dir = home.join("Library/Application Support/Spaceman");
    let pane_dir = home.join("Library/Application Support/Pane");
    std::fs::create_dir_all(&legacy_dir).unwrap();

    // 1. Valid legacy parked-windows.json migrates and loads exact original frames.
    let mut expected = HashMap::new();
    expected.insert(window(42), frame(120));
    let encoded = serde_json::to_vec(&ParkedWindows(expected.clone())).unwrap();
    std::fs::write(legacy_dir.join("parked-windows.json"), &encoded).unwrap();

    let report = crate::config::migrate_legacy_data_in_home(&home, true, true).unwrap();
    assert!(
        report
            .copied_state_files
            .contains(&"parked-windows.json".to_owned())
    );
    let loaded = load_parked_at(&pane_dir.join("parked-windows.json")).unwrap();
    assert_eq!(loaded, expected);
    // Original file stays untouched.
    assert!(legacy_dir.join("parked-windows.json").is_file());

    // 2. Malformed legacy parked-windows.json is preserved and still rejected on load.
    let bad_home = std::env::temp_dir().join(format!(
        "pane-startup-migration-bad-{}-{id}",
        std::process::id()
    ));
    let bad_legacy = bad_home.join("Library/Application Support/Spaceman");
    let bad_pane = bad_home.join("Library/Application Support/Pane");
    std::fs::create_dir_all(&bad_legacy).unwrap();
    std::fs::write(bad_legacy.join("parked-windows.json"), b"{corrupt").unwrap();

    crate::config::migrate_legacy_data_in_home(&bad_home, true, true).unwrap();
    assert!(load_parked_at(&bad_pane.join("parked-windows.json")).is_err());
    assert_eq!(
        std::fs::read(bad_pane.join("parked-windows.json")).unwrap(),
        b"{corrupt"
    );
    assert_eq!(
        std::fs::read(bad_legacy.join("parked-windows.json")).unwrap(),
        b"{corrupt"
    );

    std::fs::remove_dir_all(home).unwrap();
    std::fs::remove_dir_all(bad_home).unwrap();
}
