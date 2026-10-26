use super::*;

#[derive(serde::Deserialize, serde::Serialize)]
pub(super) struct ParkedWindows(
    #[serde(with = "crate::serde_map")] pub(super) HashMap<crate::PhysicalWindowId, crate::Rect>,
);

#[derive(serde::Serialize)]
struct ParkedWindowsRef<'a>(
    #[serde(with = "crate::serde_map")] &'a HashMap<crate::PhysicalWindowId, crate::Rect>,
);

pub(super) trait WorkspaceBackend {
    fn snapshot(
        &mut self,
        cycle: u64,
        outputs: &macos::DisplayOutputs,
    ) -> Result<crate::NativeObservation, String>;
    fn is_process_running(&mut self, app: &AppInstanceId) -> bool;
    fn apply_observed(
        &mut self,
        placements: &[crate::Placement],
        settings: &Settings,
        cancelled: &AtomicBool,
    ) -> Vec<macos::ApplyResult> {
        self.apply_managed(placements, settings, cancelled)
    }
    fn frame(
        &mut self,
        window: &crate::PhysicalWindowId,
    ) -> Result<crate::Rect, macos::WindowAccessError>;
    /// Authoritative window-server frame, immune to stale Accessibility reads.
    fn window_server_frame(&mut self, window: &crate::PhysicalWindowId) -> Option<crate::Rect>;
    /// Authoritative window-server frames for all windows in one snapshot.
    /// Use for per-cycle checks instead of repeated single-window queries.
    fn window_server_frames(&mut self) -> Option<HashMap<u32, crate::Rect>>;
    /// Window-server frames for windows currently on screen; proves a window
    /// still exists when an Accessibility snapshot omits it.
    fn onscreen_window_frames(&mut self) -> Option<HashMap<u32, crate::Rect>>;
    /// On-screen window-server IDs, front-most first.
    fn onscreen_window_order(&mut self) -> Option<Vec<u32>>;
    fn set_position(
        &mut self,
        window: &crate::PhysicalWindowId,
        target: crate::Rect,
    ) -> macos::ApplyOutcome;
    fn focused_window(&mut self) -> Option<crate::PhysicalWindowId>;
    fn frontmost_application(&mut self) -> Option<AppInstanceId>;
    /// Diagnostics only: recent user input timing for focus-follow logs.
    fn recent_input(&mut self) -> Option<macos::RecentInput> {
        None
    }
    fn raise_window(&mut self, window: &crate::PhysicalWindowId) -> bool;
    fn focus_window(&mut self, window: &crate::PhysicalWindowId) -> bool;
    fn apply_managed(
        &mut self,
        placements: &[crate::Placement],
        settings: &Settings,
        cancelled: &AtomicBool,
    ) -> Vec<macos::ApplyResult>;
    fn apply_managed_during_drag(
        &mut self,
        placements: &[crate::Placement],
        settings: &Settings,
        dragged: &crate::PhysicalWindowId,
        cancelled: &AtomicBool,
    ) -> Vec<macos::ApplyResult>;
    fn persist_parked(
        &mut self,
        parked: &HashMap<crate::PhysicalWindowId, crate::Rect>,
    ) -> Result<(), String>;
    fn persist_workspace_state(
        &mut self,
        state: &crate::workspace::WorkspaceState,
    ) -> Result<(), String>;
    fn persist_window_memory(
        &mut self,
        memory: &crate::window_memory::WindowMemory,
    ) -> Result<(), String>;
    /// Best-effort diagnostics; a journal failure never affects management.
    fn journal(&mut self, line: &crate::journal::JournalLine);
}

pub(super) struct MacWorkspaceBackend;

impl WorkspaceBackend for MacWorkspaceBackend {
    fn snapshot(
        &mut self,
        cycle: u64,
        outputs: &macos::DisplayOutputs,
    ) -> Result<crate::NativeObservation, String> {
        macos::snapshot_in_outputs(cycle, outputs).map_err(|error| error.to_string())
    }

    fn is_process_running(&mut self, app: &AppInstanceId) -> bool {
        macos::is_process_incarnation_running(app)
    }

    fn frame(
        &mut self,
        window: &crate::PhysicalWindowId,
    ) -> Result<crate::Rect, macos::WindowAccessError> {
        macos::frame(window)
    }

    fn window_server_frame(&mut self, window: &crate::PhysicalWindowId) -> Option<crate::Rect> {
        macos::window_server_frame(window)
    }

    fn window_server_frames(&mut self) -> Option<HashMap<u32, crate::Rect>> {
        macos::window_server_frames()
    }

    fn onscreen_window_frames(&mut self) -> Option<HashMap<u32, crate::Rect>> {
        macos::onscreen_window_frames()
    }

    fn onscreen_window_order(&mut self) -> Option<Vec<u32>> {
        macos::onscreen_window_order()
    }

    fn set_position(
        &mut self,
        window: &crate::PhysicalWindowId,
        target: crate::Rect,
    ) -> macos::ApplyOutcome {
        macos::set_position_only(window, target)
    }

    fn focused_window(&mut self) -> Option<crate::PhysicalWindowId> {
        macos::focused_window()
    }

    fn frontmost_application(&mut self) -> Option<AppInstanceId> {
        macos::frontmost_application()
    }

    fn recent_input(&mut self) -> Option<macos::RecentInput> {
        Some(macos::recent_input())
    }

    fn raise_window(&mut self, window: &crate::PhysicalWindowId) -> bool {
        macos::raise_window(window)
    }

    fn focus_window(&mut self, window: &crate::PhysicalWindowId) -> bool {
        macos::focus_window(window)
    }

    fn apply_managed(
        &mut self,
        placements: &[crate::Placement],
        settings: &Settings,
        cancelled: &AtomicBool,
    ) -> Vec<macos::ApplyResult> {
        macos::apply_managed(placements, settings, None, || {
            !cancelled.load(Ordering::Acquire)
        })
    }

    fn apply_managed_during_drag(
        &mut self,
        placements: &[crate::Placement],
        settings: &Settings,
        dragged: &crate::PhysicalWindowId,
        cancelled: &AtomicBool,
    ) -> Vec<macos::ApplyResult> {
        macos::apply_managed(placements, settings, Some(dragged), || {
            !cancelled.load(Ordering::Acquire)
        })
    }

    fn persist_parked(
        &mut self,
        parked: &HashMap<crate::PhysicalWindowId, crate::Rect>,
    ) -> Result<(), String> {
        persist_parked(parked)
    }

    fn persist_workspace_state(
        &mut self,
        state: &crate::workspace::WorkspaceState,
    ) -> Result<(), String> {
        persist_workspace_state(state)
    }

    fn persist_window_memory(
        &mut self,
        memory: &crate::window_memory::WindowMemory,
    ) -> Result<(), String> {
        let path = window_memory_path().ok_or("HOME is unavailable")?;
        persist_json(&path, memory, "window memory")
    }

    fn journal(&mut self, line: &crate::journal::JournalLine) {
        static FAILED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        let result = crate::journal::default_paths()
            .ok_or_else(|| "HOME is unavailable".to_owned())
            .and_then(|(path, rotated)| crate::journal::append(&path, &rotated, line));
        match result {
            Ok(()) => FAILED.store(false, Ordering::Relaxed),
            // Log the first failure of a run, not every event.
            Err(error) if !FAILED.swap(true, Ordering::Relaxed) => {
                log_event!("arrangement journal unavailable: {error}");
            }
            Err(_) => {}
        }
    }
}

fn window_memory_path() -> Option<PathBuf> {
    crate::config::application_support_path("window-memory.json")
}

/// Window memory is advisory: unreadable or invalid memory is logged and
/// replaced by an empty memory rather than blocking startup or recovery.
pub(super) fn load_window_memory() -> crate::window_memory::WindowMemory {
    let Some(path) = window_memory_path() else {
        return crate::window_memory::WindowMemory::default();
    };
    let memory = load_json::<crate::window_memory::WindowMemory>(&path, "window memory").and_then(
        |memory| {
            let memory = memory.unwrap_or_default();
            memory
                .validate()
                .map(|()| memory)
                .map_err(|error| format!("window memory is invalid: {error}"))
        },
    );
    memory.unwrap_or_else(|error| {
        log_event!("ignoring {error}");
        crate::window_memory::WindowMemory::default()
    })
}

fn parked_path() -> Option<PathBuf> {
    crate::config::application_support_path("parked-windows.json")
}

fn workspace_state_path() -> Option<PathBuf> {
    crate::config::application_support_path("workspace-state.json")
}

pub(super) fn load_parked() -> Result<HashMap<crate::PhysicalWindowId, crate::Rect>, String> {
    let path = parked_path().ok_or("HOME is unavailable")?;
    load_json::<ParkedWindows>(&path, "parked-window")
        .map(|parked| parked.map_or_else(HashMap::new, |parked| parked.0))
}

fn persist_parked(parked: &HashMap<crate::PhysicalWindowId, crate::Rect>) -> Result<(), String> {
    let path = parked_path().ok_or("HOME is unavailable")?;
    persist_json(&path, &ParkedWindowsRef(parked), "parked-window")
}

pub(super) fn load_workspace_state() -> Result<Option<crate::workspace::WorkspaceState>, String> {
    let path = workspace_state_path().ok_or("HOME is unavailable")?;
    let state = load_json::<crate::workspace::WorkspaceState>(&path, "workspace")?;
    if let Some(state) = &state {
        state
            .validate()
            .map_err(|error| format!("workspace state is invalid: {error}"))?;
    }
    Ok(state)
}

fn load_json<T: serde::de::DeserializeOwned>(
    path: &std::path::Path,
    state: &str,
) -> Result<Option<T>, String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("failed to read {state} state: {error}")),
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|error| format!("{state} state is invalid: {error}"))
}

pub(super) fn recover_startup_metadata(
    config: Result<Config, String>,
    workspace_state: Result<Option<crate::workspace::WorkspaceState>, String>,
) -> (Config, Option<crate::workspace::WorkspaceState>) {
    let config = match config {
        Ok(config) => config,
        Err(error) => {
            log_event!("ignoring invalid Spaceman config during recovery: {error}");
            return (Config::default(), None);
        }
    };
    let workspace_state = workspace_state.unwrap_or_else(|error| {
        log_event!("ignoring invalid Spaceman workspace state during recovery: {error}");
        None
    });
    (config, workspace_state)
}

fn persist_workspace_state(state: &crate::workspace::WorkspaceState) -> Result<(), String> {
    let path = workspace_state_path().ok_or("HOME is unavailable")?;
    persist_json(&path, state, "workspace")
}

fn persist_json(
    path: &std::path::Path,
    value: &impl serde::Serialize,
    state: &str,
) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{state} state path has no parent"))?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create state directory: {error}"))?;
    let bytes = serde_json::to_vec(value)
        .map_err(|error| format!("failed to serialize {state} state: {error}"))?;
    let mut file = atomic_write_file::AtomicWriteFile::open(path)
        .map_err(|error| format!("failed to open {state} state: {error}"))?;
    file.write_all(&bytes)
        .map_err(|error| format!("failed to write {state} state: {error}"))?;
    file.commit()
        .map_err(|error| format!("failed to commit {state} state: {error}"))
}
