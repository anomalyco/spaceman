//! #516: closing an app's visible tile and its parked one together, with the app still frontmost
//! and no focused window, switched to the parked window's now-empty workspace. Reported by
//! @coygeek.

use super::super::*;

#[test]
fn frontmost_app_whose_windows_all_closed_does_not_pull_a_switch() {
    // The parked slot is not yet retired, but the window server no longer
    // lists it, so there is nothing on workspace 2 to follow.
    let closed = window(20);
    let backend = FakeBackend {
        frontmost: Some(closed.app.clone()),
        ..FakeBackend::default()
    };
    let mut parked = HashMap::new();
    parked.insert(closed.clone(), frame(50));
    let mut runtime = WorkspaceRuntime::with_backend(ManagementScope::Both, backend, parked);
    bind(&mut runtime, WorkspaceId::new(2).unwrap(), &closed);

    let status = runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();

    assert_eq!(status.active, vec![WorkspaceId::ONE, WorkspaceId::SIX]);
}
