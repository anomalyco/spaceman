use super::*;

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
        loop {
            let command = match runtime.strip_views.next_deadline() {
                Some(deadline) => {
                    match commands.recv_timeout(deadline.saturating_duration_since(Instant::now()))
                    {
                        Ok(command) => command,
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            // A glide has settled: lay its strip out for good.
                            objc2::rc::autoreleasepool(|_| {
                                runtime.settle_strip_glides(Instant::now());
                            });
                            if !send_strip_commands(&mut runtime, &results) {
                                return;
                            }
                            continue;
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
                    }
                }
                None => match commands.recv() {
                    Ok(command) => command,
                    Err(_) => return,
                },
            };
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
            if !send_strip_commands(&mut runtime, &results) || results.send(result).is_err() {
                return;
            }
        }
    });
}

/// Hands glides started or stopped by the last command to the main thread, ahead of the
/// command's status.
fn send_strip_commands(runtime: &mut WorkspaceRuntime, results: &Sender<WorkerResult>) -> bool {
    runtime
        .strip_views
        .outbox
        .drain(..)
        .all(|command| results.send(WorkerResult::Strip(command)).is_ok())
}
