use std::sync::atomic::{AtomicUsize, Ordering};

use super::flow::{
    Effect, Flow, Input, SetupState, Step, completion_recorded_at, launch_step_at,
    record_completion_at, record_step_at, saved_step_at,
};
use crate::{apps::window_manager_name, controls::PermissionControl};

fn temp_dir(name: &str) -> std::path::PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "spaceman-onboarding-test-{name}-{}-{id}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn fresh_untrusted_launch_opens_welcome_and_blocks_continue_until_granted() {
    let dir = temp_dir("fresh");
    assert_eq!(launch_step_at(&dir, false), Some(Step::Welcome));

    let mut flow = Flow::new(
        Step::Welcome,
        SetupState {
            accessibility: false,
            launch_at_login: false,
        },
        false,
    );
    assert_eq!(flow.view().step, Step::Welcome);
    assert!(!flow.view().can_go_back);

    assert_eq!(flow.on(Input::Continue), Effect::None);
    assert_eq!(flow.view().step, Step::Setup);
    assert_eq!(flow.view().direction, 1);
    assert_eq!(flow.view().accessibility, PermissionControl::Allow);
    assert!(!flow.view().ready);

    // Continue stays blocked while Accessibility is missing.
    assert_eq!(flow.on(Input::Continue), Effect::None);
    assert_eq!(flow.view().step, Step::Setup);

    // Asking opens the Accessibility guide and enters Waiting.
    assert_eq!(
        flow.on(Input::GrantAccessibility),
        Effect::OpenAccessibility
    );
    assert_eq!(flow.view().accessibility, PermissionControl::Waiting);

    // Granting triggers AdvanceSoon so the green check settles before advancing.
    assert_eq!(
        flow.on(Input::Status(SetupState {
            accessibility: true,
            launch_at_login: true,
        })),
        Effect::AdvanceSoon
    );
    assert_eq!(
        flow.view().accessibility,
        PermissionControl::Allowed { animated: true }
    );
    assert!(flow.view().ready);

    assert_eq!(flow.on(Input::Continue), Effect::None);
    assert_eq!(flow.view().step, Step::Rules);
    assert_eq!(flow.view().index, 2);

    assert_eq!(flow.on(Input::Continue), Effect::None);
    assert_eq!(flow.view().step, Step::Done);

    assert_eq!(flow.on(Input::Continue), Effect::Finish);
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn existing_trusted_install_records_completion_without_opening_window() {
    let dir = temp_dir("trusted-upgrade");
    assert_eq!(launch_step_at(&dir, true), None);
    assert!(completion_recorded_at(&dir));
    // Subsequent launches even without Accessibility do not restart Welcome.
    assert_eq!(launch_step_at(&dir, false), None);
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn saved_setup_step_resumes_at_rules_after_accessibility_grant_relaunch() {
    let dir = temp_dir("resume");
    record_step_at(&dir, Step::Setup).unwrap();
    assert_eq!(saved_step_at(&dir), Some(Step::Setup));
    assert_eq!(launch_step_at(&dir, true), Some(Step::Setup));

    let flow = Flow::new(
        Step::Setup,
        SetupState {
            accessibility: true,
            launch_at_login: false,
        },
        false,
    );
    assert_eq!(flow.step(), Step::Rules);

    record_completion_at(&dir).unwrap();
    assert!(completion_recorded_at(&dir));
    assert_eq!(saved_step_at(&dir), None);
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn manual_tour_walks_all_steps_and_allows_dismiss() {
    let mut flow = Flow::new(
        Step::Welcome,
        SetupState {
            accessibility: true,
            launch_at_login: true,
        },
        true,
    );
    assert_eq!(flow.on(Input::Continue), Effect::None);
    assert_eq!(flow.step(), Step::Setup);
    assert!(flow.view().ready);
    assert_eq!(flow.on(Input::Back), Effect::None);
    assert_eq!(flow.step(), Step::Welcome);
    assert_eq!(flow.view().direction, -1);
    assert_eq!(flow.on(Input::Dismiss), Effect::Finish);
}

#[test]
fn recognizes_conflicting_window_managers_by_bundle_or_daemon_name() {
    assert_eq!(
        window_manager_name(Some("bobko.aerospace"), None),
        Some("AeroSpace")
    );
    assert_eq!(
        window_manager_name(Some("com.knollsoft.Rectangle"), None),
        Some("Rectangle")
    );
    assert_eq!(window_manager_name(None, Some("yabai")), Some("yabai"));
    assert_eq!(
        window_manager_name(Some("com.apple.Safari"), Some("Safari")),
        None
    );
}
