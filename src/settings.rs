//! The settings window is its own Rune app, `spaceman-settings`, beside the agent in the
//! bundle: GPUI owns this process's event loop, and Rune needs one of its own. The agent starts
//! it, brings it forward when asked again, and never tiles it. It reads and writes
//! `config.toml` itself and asks the agent for the rest over the control socket.

use std::{
    process::{Child, Command},
    sync::{
        Mutex,
        atomic::{AtomicI32, Ordering},
    },
};

use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};

const EXECUTABLE: &str = "spaceman-settings";

static CHILD: Mutex<Option<Child>> = Mutex::new(None);
/// The running settings window's process, which the agent leaves alone.
static PID: AtomicI32 = AtomicI32::new(0);

pub(crate) fn open() -> Result<(), String> {
    let mut child = CHILD.lock().map_err(|_| "settings state is poisoned")?;
    if let Some(running) = child.as_mut()
        && matches!(running.try_wait(), Ok(None))
    {
        let pid = running.id() as i32;
        if let Some(application) =
            NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
        {
            #[allow(deprecated)]
            application.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows);
        }
        return Ok(());
    }
    let path = std::env::current_exe()
        .map_err(|error| format!("could not find Spaceman: {error}"))?
        .with_file_name(EXECUTABLE);
    let spawned = Command::new(&path)
        .spawn()
        .map_err(|error| format!("could not start {}: {error}", path.display()))?;
    PID.store(spawned.id() as i32, Ordering::Release);
    *child = Some(spawned);
    Ok(())
}

/// The settings window's process, or one started from the same bundle.
pub(crate) fn is_settings_process(pid: i32, bundle_id: Option<&str>) -> bool {
    pid == PID.load(Ordering::Acquire) || bundle_id == Some(BUNDLE_ID)
}

const BUNDLE_ID: &str = "com.kitlangton.spaceman";
