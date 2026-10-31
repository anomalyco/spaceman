use super::*;

pub(super) fn is_window_drag(start: crate::Rect, current: crate::Rect) -> bool {
    let moved = (start.x - current.x).abs().max((start.y - current.y).abs()) >= 6;
    let resized = (start.width - current.width)
        .abs()
        .max((start.height - current.height).abs())
        > 2;
    moved && !resized
}

pub(super) fn is_window_resize(start: crate::Rect, current: crate::Rect) -> bool {
    (start.width - current.width)
        .abs()
        .max((start.height - current.height).abs())
        >= 6
}

pub(super) fn request_restore(
    commands: &Sender<WorkerCommand>,
    worker_busy: &mut bool,
    worker_interruptible: &mut bool,
    restore_pending: &mut bool,
    restoring: &mut bool,
) {
    if *worker_busy {
        *restore_pending = true;
    } else {
        send_restore(commands, worker_busy, worker_interruptible, restoring);
    }
}

pub(super) fn send_restore(
    commands: &Sender<WorkerCommand>,
    worker_busy: &mut bool,
    worker_interruptible: &mut bool,
    restoring: &mut bool,
) {
    *worker_busy = send_worker_command(
        commands,
        WorkerCommand::RestoreAll,
        worker_busy,
        worker_interruptible,
    );
    *restoring = *worker_busy;
}

pub(super) fn cancel_unavailable_exit(
    exit_intent: &mut Option<ExitIntent>,
    last_error: &mut Option<String>,
) {
    if exit_intent.take() == Some(ExitIntent::Terminate) {
        termination::cancel_pending();
    }
    *last_error = Some("Exit blocked because window restoration is unavailable".into());
    status_item::set_status("Exit blocked · restoration unavailable");
}

pub(super) fn queue_input(
    queue: &mut InputQueue,
    input: QueuedInput,
    last_error: &mut Option<String>,
) {
    if let Err(error) = queue.push(input) {
        log_event!("{error}");
        *last_error = Some(error.into());
        status_item::set_status(error);
    }
}

pub(super) fn send_worker_command(
    commands: &Sender<WorkerCommand>,
    command: WorkerCommand,
    worker_busy: &mut bool,
    worker_interruptible: &mut bool,
) -> bool {
    let interruptible = command.interruptible();
    if commands.send(command).is_err() {
        return false;
    }
    *worker_busy = true;
    *worker_interruptible = interruptible;
    true
}

pub(super) fn send_with_outputs(
    commands: &Sender<WorkerCommand>,
    command: impl FnOnce(macos::DisplayOutputs) -> WorkerCommand,
    worker_busy: &mut bool,
    worker_interruptible: &mut bool,
) {
    match macos::outputs() {
        Ok(outputs) => {
            send_worker_command(
                commands,
                command(outputs),
                worker_busy,
                worker_interruptible,
            );
        }
        Err(error) => status_item::set_status(&format!("Display error: {error}")),
    }
}

pub(super) fn show_result(result: Result<CycleStatus, String>) {
    match result {
        Ok(status) => {
            status_item::set_workspace_layouts(&status.layouts);
            if status.failed > 0 {
                status_item::set_status(&format!(
                    "{} window operation{} failed",
                    status.failed,
                    if status.failed == 1 { "" } else { "s" },
                ));
            } else {
                status_item::clear_status();
            }
        }
        Err(error) => status_item::set_status(&error),
    }
}

pub(super) fn spawn_worker(
    commands: Receiver<WorkerCommand>,
    results: Sender<WorkerResult>,
    cancelled: Arc<AtomicBool>,
    scope: ManagementScope,
    parked: HashMap<crate::PhysicalWindowId, crate::Rect>,
    config: Config,
    workspace_state: Option<crate::workspace::WorkspaceState>,
) {
    thread::spawn(move || {
        let mut config_stamp = crate::config::modified();
        let restore_on_start = workspace_state.is_none();
        let mut runtime = objc2::rc::autoreleasepool(|_| {
            let mut runtime = WorkspaceRuntime::new(scope, parked, config, workspace_state);
            runtime.restore_window_memory(load_window_memory());
            runtime.journal_start(!restore_on_start);
            // Windows still parked afterwards are retried by the paused loop's
            // restore or, while tiling, by each cycle's unowned-window recovery.
            if restore_on_start {
                let requested = runtime.parked.len();
                let failed = runtime.restore_all();
                if requested > 0 {
                    runtime.journal_restore("startup without saved workspaces", requested, failed);
                }
            }
            runtime
        });
        while let Ok(command) = commands.recv() {
            // Every command starts uncancelled: a cancellation aimed at previous
            // work must never abort a freshly dispatched command. Background
            // cycles and previews remain interruptible; explicit user
            // commands are atomic once native writes begin.
            cancelled.store(false, Ordering::Release);
            let stamp = crate::config::modified();
            if stamp != config_stamp {
                config_stamp = stamp;
                match Config::load() {
                    Ok(config) => {
                        runtime.replace_config(config);
                        log_event!("reloaded config");
                    }
                    Err(error) => log_event!("kept the previous config: {error}"),
                }
            }
            let label = command.label();
            let started = Instant::now();
            let Some(result) = objc2::rc::autoreleasepool(|_| runtime.execute(command, &cancelled))
            else {
                return;
            };
            let elapsed = started.elapsed();
            if elapsed >= SLOW_WORKER_COMMAND {
                log_event!(
                    "slow worker command: {label} took {}ms",
                    elapsed.as_millis()
                );
            }
            if results.send(result).is_err() {
                return;
            }
        }
    });
}

/// The agent's half of the settings window: what only this process can read.
pub(super) fn settings_state(
    automatic: bool,
    active_workspaces: &[WorkspaceId],
    last_error: Option<&str>,
) -> serde_json::Value {
    let launch_at_login = match login_item::status() {
        Ok(login_item::Status::Enabled) => "enabled".to_string(),
        Ok(login_item::Status::Disabled) => "disabled".to_string(),
        Ok(login_item::Status::RequiresApproval) => "requires-approval".to_string(),
        Err(error) => format!("error: {error}"),
    };
    let update = match sparkle::status() {
        sparkle::UpdateStatus::Unavailable => "unavailable",
        sparkle::UpdateStatus::Idle => "idle",
        sparkle::UpdateStatus::Checking => "checking",
        sparkle::UpdateStatus::UpToDate => "up-to-date",
        sparkle::UpdateStatus::UpdateAvailable => "available",
    };
    serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "accessibility": macos::accessibility_trusted(),
        "automatic": automatic,
        "launch_at_login": launch_at_login,
        "update": update,
        "active_workspaces": active_workspaces,
        "last_error": last_error,
    })
}
