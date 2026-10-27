use std::{
    collections::{HashMap, HashSet},
    io::Write,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender, TryRecvError, channel},
    },
    thread,
    time::{Duration, Instant},
};

use gpui::{Application, Timer};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

use crate::{
    AppInstanceId, BindingState, Decision, Direction, DisplaySelector, NavigationTile,
    ResizeDirection, Settings, WorkspaceId, Workspaces,
    config::{Config, WindowDisposition},
    control::{self, Command as ControlCommand, Request as ControlRequest},
    hotkeys::{HotkeyPlan, Hotkeys},
    live, login_item, macos, permission_guide, settings, sparkle, status_item, termination,
};

/// Timestamped diagnostic line on stderr (redirected to `agent.log` in bundle
/// launches). Never include window titles or other user content.
///
/// Unlike `eprintln!`, a failed write (such as a full disk) is ignored rather
/// than panicking the calling thread, and each line is written in one call.
macro_rules! log_event {
    ($($arg:tt)*) => {{
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let line = format!(
            "[{}.{:03}] {}\n",
            now.as_secs(),
            now.subsec_millis(),
            format_args!($($arg)*)
        );
        let _ = std::io::Write::write_all(&mut std::io::stderr().lock(), line.as_bytes());
    }};
}

mod app_loop;
mod backend;
mod commands;
mod cycle;
mod input;
mod journal;
mod memory;
mod minimums;
mod parking;
mod runtime;
mod switching;

use app_loop::{AppLoop, is_window_drag, is_window_resize, run_app_loop, spawn_worker};
use backend::*;
use input::{GestureTracker, InputQueue, QueuedInput};
use journal::JournalState;
use memory::MemoryState;
use runtime::*;
const TICK_INTERVAL: Duration = Duration::from_millis(25);
const FALLBACK_REFRESH_INTERVAL: Duration = Duration::from_secs(1);
const APPLICATION_REFRESH_INTERVAL: Duration = Duration::from_secs(1);
const RESTORE_RETRY_INTERVAL: Duration = Duration::from_secs(1);
const APPLICATION_ACTIVATION_SETTLE: Duration = Duration::from_millis(50);

const fn refresh_due(
    window_created: bool,
    event_dirty: bool,
    fallback_due: bool,
    activation_ready: bool,
) -> bool {
    window_created || (activation_ready && (event_dirty || fallback_due))
}

#[derive(Clone, Copy)]
enum ManagementScope {
    Both,
    Main,
    Secondary,
}

impl ManagementScope {
    fn initial_workspaces(self) -> &'static [WorkspaceId] {
        match self {
            Self::Both => &[WorkspaceId::ONE, WorkspaceId::SIX],
            Self::Main => &[WorkspaceId::ONE],
            Self::Secondary => &[WorkspaceId::SIX],
        }
    }

    fn automatic_status(self) -> &'static str {
        match self {
            Self::Both => "Automatic tiling enabled · Workspaces 1 / 6",
            Self::Main => "Automatic tiling enabled · Workspace 1",
            Self::Secondary => "Automatic tiling enabled · Workspace 6",
        }
    }

    fn from_environment() -> Result<Self, String> {
        match std::env::var("SPACEMAN_MANAGED_DISPLAY") {
            Ok(value) => Self::parse(&value),
            Err(std::env::VarError::NotPresent) => Ok(Self::Both),
            Err(std::env::VarError::NotUnicode(_)) => {
                Err("SPACEMAN_MANAGED_DISPLAY must be valid UTF-8".into())
            }
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "main" => Ok(Self::Main),
            "secondary" => Ok(Self::Secondary),
            "both" => Ok(Self::Both),
            _ => Err(format!(
                "SPACEMAN_MANAGED_DISPLAY must be main, secondary, or both; got {value:?}"
            )),
        }
    }

    fn manages_selector(self, display: &DisplaySelector) -> bool {
        matches!(
            (self, display),
            (Self::Both, _)
                | (Self::Main, DisplaySelector::Main)
                | (Self::Secondary, DisplaySelector::Secondary)
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Action {
    Worker(WorkerAction),
    ToggleAutomatic,
    OpenSettings,
    OpenAccessibility,
    CheckForUpdates,
    InstallUpdate,
    Terminate,
    ToggleLaunchAtLogin,
    Restart,
    Quit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum WorkerAction {
    Retile,
    SwitchWorkspace(WorkspaceId),
    WorkspaceBackAndForth,
    MoveFocused(WorkspaceId),
    MoveApp(String, WorkspaceId),
    CycleFocus,
    Focus(Direction),
    Reorder(Direction),
    ToggleLayout,
    Resize(ResizeDirection),
    Balance,
    ToggleFullscreenLayout,
}

impl WorkerAction {
    fn queued_acknowledgement(&self) -> String {
        match self {
            Self::Retile => "retile queued".into(),
            Self::SwitchWorkspace(workspace) => format!("switch to workspace {workspace} queued"),
            Self::WorkspaceBackAndForth => "workspace back-and-forth queued".into(),
            Self::MoveFocused(workspace) => format!("move to workspace {workspace} queued"),
            Self::MoveApp(_, workspace) => format!("move app to workspace {workspace} queued"),
            Self::CycleFocus => "focus cycle queued".into(),
            Self::Focus(_) => "focus queued".into(),
            Self::Reorder(_) => "directional move queued".into(),
            Self::ToggleLayout => "layout toggle queued".into(),
            Self::Resize(_) => "resize queued".into(),
            Self::Balance => "balance queued".into(),
            Self::ToggleFullscreenLayout => "fullscreen layout toggle queued".into(),
        }
    }
}

enum WorkerCommand {
    Action(WorkerAction, macos::DisplayOutputs),
    Gesture(FinalGesture, bool, macos::DisplayOutputs),
    RestoreAll,
    Stop,
}

/// Worker commands slower than this are logged; long Accessibility stalls
/// otherwise look like Spaceman silently doing nothing.
const SLOW_WORKER_COMMAND: Duration = Duration::from_secs(2);

impl WorkerCommand {
    const fn label(&self) -> &'static str {
        match self {
            Self::Action(WorkerAction::Retile, _) => "retile",
            Self::Action(..) => "action",
            Self::Gesture(..) => "gesture",
            Self::RestoreAll => "restore all",
            Self::Stop => "stop",
        }
    }

    fn interruptible(&self) -> bool {
        matches!(
            self,
            Self::Action(WorkerAction::Retile, _) | Self::Gesture(_, true, _)
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum FinalGesture {
    Drop(crate::PhysicalWindowId, (i32, i32)),
    Resize(crate::PhysicalWindowId, crate::Rect),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExitIntent {
    Quit,
    Restart,
    Update,
    Terminate,
}

impl ExitIntent {
    fn mode_label(self) -> &'static str {
        match self {
            Self::Quit => "quitting",
            Self::Restart => "restarting",
            Self::Update => "updating",
            Self::Terminate => "terminating",
        }
    }
}

enum WorkerResult {
    Status {
        result: Result<CycleStatus, String>,
        debug: Box<RuntimeDebug>,
    },
    Restored {
        failed: usize,
    },
}

pub fn run() -> Result<(), String> {
    log_event!(
        "agent started (pid {}, version {})",
        std::process::id(),
        env!("CARGO_PKG_VERSION")
    );
    let scope = ManagementScope::from_environment()?;
    let parked = load_parked()?;
    let (config, workspace_state) =
        recover_startup_metadata(Config::load(), load_workspace_state());
    let mut configured_workspaces = Workspaces::default();
    config.configure_workspaces(&mut configured_workspaces);
    let mut hotkey_workspaces = WorkspaceId::numbered()
        .chain(config.named_workspaces())
        .chain(
            workspace_state
                .as_ref()
                .into_iter()
                .flat_map(crate::workspace::WorkspaceState::named_workspaces_in_use),
        )
        .filter(|workspace| scope.manages_selector(configured_workspaces.assignment(*workspace)))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    hotkey_workspaces.sort();
    let (commands, worker_commands) = channel();
    let (worker_results, results) = channel();
    let (actions_sender, actions) = channel();
    let (control_requests, controls) = channel();
    let control_server = control::listen(control_requests)?;
    let signal_actions = actions_sender.clone();
    ctrlc::set_handler(move || {
        let _ = signal_actions.send(Action::Quit);
    })
    .map_err(|error| format!("failed to install app signal handler: {error}"))?;
    let cancelled = Arc::new(AtomicBool::new(false));
    spawn_worker(
        worker_commands,
        worker_results,
        Arc::clone(&cancelled),
        scope,
        parked,
        config,
        workspace_state,
    );

    let application = Application::new();
    application.run(move |cx| {
        if let Some(marker) = MainThreadMarker::new() {
            NSApplication::sharedApplication(marker)
                .setActivationPolicy(NSApplicationActivationPolicy::Accessory);
        }
        if let Err(error) = termination::install(actions_sender.clone()) {
            log_event!("failed to install application termination gate: {error}");
            cx.quit();
            return;
        }
        if let Err(error) = status_item::install(actions_sender.clone()) {
            log_event!("failed to install Spaceman status item: {error}");
            quit_app(cx);
            return;
        }
        if let Err(error) = sparkle::start(actions_sender.clone()) {
            log_event!("Sparkle unavailable: {error}");
        }
        match login_item::status() {
            Ok(status) => status_item::set_launch_at_login(status),
            Err(error) => status_item::set_status(&format!("Launch at Login unavailable: {error}")),
        }
        let workspace_numbers = scope
            .initial_workspaces()
            .iter()
            .filter_map(|workspace| workspace.number())
            .collect::<Vec<_>>();
        status_item::set_workspace_numbers(&workspace_numbers);
        if !macos::accessibility_trusted() {
            status_item::set_status("Paused · Accessibility required");
            if let Err(error) = settings::open() {
                status_item::set_status(&format!("Could not open Settings: {error}"));
            }
        }
        let events = macos::WindowEventMonitor::new();
        cx.spawn(async move |cx| {
            run_app_loop(
                cx,
                AppLoop {
                    actions,
                    actions_sender,
                    commands,
                    results,
                    cancelled,
                    scope,
                    events,
                    hotkey_workspaces,
                    controls,
                    _control_server: control_server,
                },
            )
            .await;
        })
        .detach();
    });
    Ok(())
}

fn quit_app(cx: &mut gpui::App) {
    termination::allow_next();
    cx.quit();
}

fn queue(actions: &Sender<Action>, action: Action, ack: &str) -> Result<String, String> {
    actions
        .send(action)
        .map(|()| ack.into())
        .map_err(|_| "Spaceman agent is stopping".into())
}

#[cfg(test)]
mod tests;
