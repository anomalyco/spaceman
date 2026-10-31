use super::*;

pub(super) struct AppLoop {
    pub(super) actions: Receiver<Action>,
    pub(super) actions_sender: Sender<Action>,
    pub(super) commands: Sender<WorkerCommand>,
    pub(super) results: Receiver<WorkerResult>,
    pub(super) cancelled: Arc<AtomicBool>,
    pub(super) scope: ManagementScope,
    pub(super) events: macos::WindowEventMonitor,
    pub(super) hotkey_workspaces: Vec<WorkspaceId>,
    pub(super) controls: Receiver<ControlRequest>,
    pub(super) _control_server: control::Server,
}

pub(super) async fn run_app_loop(cx: &mut gpui::AsyncApp, app: AppLoop) {
    let AppLoop {
        actions,
        actions_sender,
        commands,
        results,
        cancelled,
        scope,
        events,
        hotkey_workspaces,
        controls,
        _control_server,
    } = app;
    let (mut _hotkeys, hotkey_error) = if macos::accessibility_trusted() {
        match HotkeyPlan::current(&hotkey_workspaces)
            .and_then(|plan| Hotkeys::install(actions_sender.clone(), plan))
        {
            Ok(hotkeys) => (Some(hotkeys), None),
            Err(error) => (None, Some(format!("Hotkeys unavailable: {error}"))),
        }
    } else {
        (None, Some("Accessibility permission is required".into()))
    };
    let mut automatic = _hotkeys.is_some();
    let mut worker_busy = false;
    let mut worker_interruptible = false;
    let mut exit_intent = None;
    let mut restore_pending = false;
    let mut restoring = false;
    let mut next_restore_retry = None;
    let mut active_workspaces = scope.initial_workspaces().to_vec();
    let mut last_error = hotkey_error;
    let mut latest_debug: Option<RuntimeDebug> = None;
    let mut pending_inputs = InputQueue::default();
    let mut gesture_tracker = GestureTracker::default();
    let mut last_refresh = Instant::now() - FALLBACK_REFRESH_INTERVAL;
    let mut last_application_refresh = last_refresh;
    let mut last_hotkey_refresh = last_refresh;
    let mut mouse_was_down = macos::mouse_down();
    let mut frontmost_application = macos::frontmost_application();
    let mut activation_refresh_at = None;

    status_item::set_automatic(automatic);
    if automatic {
        status_item::set_status("Automatic tiling enabled");
        send_with_outputs(
            &commands,
            |outputs| WorkerCommand::Action(WorkerAction::Retile, outputs),
            &mut worker_busy,
            &mut worker_interruptible,
        );
    } else {
        status_item::set_status(last_error.as_deref().unwrap_or("Paused"));
        send_restore(
            &commands,
            &mut worker_busy,
            &mut worker_interruptible,
            &mut restoring,
        );
    }

    loop {
        loop {
            let result = match results.try_recv() {
                Ok(result) => result,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    _hotkeys = None;
                    pending_inputs.clear();
                    gesture_tracker.clear();
                    status_item::set_automatic(false);
                    automatic = false;
                    worker_busy = false;
                    restore_pending = false;
                    restoring = false;
                    next_restore_retry = None;
                    if exit_intent.is_some() {
                        cancel_unavailable_exit(cx, &mut exit_intent, &mut last_error);
                    } else {
                        status_item::set_status("Worker stopped unexpectedly");
                    }
                    break;
                }
            };
            match result {
                WorkerResult::Status { result, debug } if automatic => {
                    worker_busy = false;
                    cancelled.store(false, Ordering::Release);
                    latest_debug = Some(*debug);
                    // Newer input routinely cancels background work; only
                    // genuine failures are worth a log line.
                    if let Err(error) = &result
                        && last_error.as_ref() != Some(error)
                        && !error.ends_with("cancelled")
                    {
                        log_event!("worker error: {error}");
                    }
                    last_error = result.as_ref().err().cloned();
                    if let Ok(status) = &result {
                        active_workspaces.clone_from(&status.active);
                    }
                    show_result(result);
                }
                WorkerResult::Status { result, debug } => {
                    worker_busy = false;
                    latest_debug = Some(*debug);
                    if let Err(error) = result {
                        last_error = Some(error);
                    }
                }
                WorkerResult::Restored { failed: 0 } if exit_intent == Some(ExitIntent::Update) => {
                    worker_busy = false;
                    restoring = false;
                    let continuation = cx
                        .update(|_| sparkle::continue_install())
                        .map_err(|error| error.to_string())
                        .and_then(|result| result);
                    match continuation {
                        Ok(()) => {
                            termination::allow_next();
                            status_item::set_status("Installing update...");
                        }
                        Err(error) => {
                            log_event!("Sparkle continuation failed after restoration: {error}");
                            drop(cx.update(quit_app));
                            return;
                        }
                    }
                }
                WorkerResult::Restored { failed: 0 }
                    if exit_intent == Some(ExitIntent::Terminate) =>
                {
                    drop(commands.send(WorkerCommand::Stop));
                    termination::reply(true);
                    return;
                }
                WorkerResult::Restored { failed: 0 } if exit_intent.is_some() => {
                    drop(commands.send(WorkerCommand::Stop));
                    drop(cx.update(quit_app));
                    return;
                }
                WorkerResult::Restored { failed: 0 } => {
                    worker_busy = false;
                    restoring = false;
                    status_item::set_status(last_error.as_deref().unwrap_or("Paused"));
                }
                WorkerResult::Restored { failed } => {
                    worker_busy = false;
                    restoring = true;
                    next_restore_retry = Some(Instant::now() + RESTORE_RETRY_INTERVAL);
                    status_item::set_status(&format!("Restoring windows · {failed} failed"));
                }
            }
            if restore_pending && !worker_busy {
                restore_pending = false;
                send_restore(
                    &commands,
                    &mut worker_busy,
                    &mut worker_interruptible,
                    &mut restoring,
                );
                if !worker_busy && exit_intent.is_some() {
                    cancel_unavailable_exit(cx, &mut exit_intent, &mut last_error);
                }
            }
        }
        for request in controls.try_iter() {
            let response = match request.command {
                ControlCommand::Status => Ok(serde_json::json!({
                    "mode": if let Some(intent) = exit_intent {
                        intent.mode_label()
                    } else if restoring || restore_pending {
                        "restoring"
                    } else if automatic {
                        "automatic"
                    } else {
                        "paused"
                    },
                    "busy": worker_busy,
                    "last_error": last_error.as_deref(),
                    "active_workspaces": active_workspaces,
                })
                .to_string()),
                ControlCommand::Settings => {
                    Ok(
                        settings_state(automatic, &active_workspaces, last_error.as_deref())
                            .to_string(),
                    )
                }
                ControlCommand::App(ref action) => queue(&actions_sender, action.clone(), "queued"),
                ControlCommand::Debug => latest_debug
                    .as_ref()
                    .ok_or_else(|| "runtime debug state is not ready".into())
                    .and_then(|debug| {
                        serde_json::to_string(debug).map_err(|error| error.to_string())
                    }),
                ControlCommand::Queue(ref action) if automatic => queue(
                    &actions_sender,
                    Action::Worker(action.clone()),
                    &action.queued_acknowledgement(),
                ),
                ControlCommand::Queue(_) => Err("automatic tiling is paused".into()),
                ControlCommand::Restart if worker_busy => {
                    Err("Spaceman is busy; retry restart".into())
                }
                ControlCommand::Restart => {
                    queue(&actions_sender, Action::Restart, "restart queued")
                }
                ControlCommand::Pause if automatic => {
                    queue(&actions_sender, Action::ToggleAutomatic, "pause queued")
                }
                ControlCommand::Pause => Ok("already paused".into()),
                ControlCommand::Resume if restoring || restore_pending => {
                    Err("windows are still being restored".into())
                }
                ControlCommand::Resume if automatic => Ok("already automatic".into()),
                ControlCommand::Resume => {
                    queue(&actions_sender, Action::ToggleAutomatic, "resume queued")
                }
                ControlCommand::Quit => queue(&actions_sender, Action::Quit, "quit queued"),
            };
            request.respond(response);
        }
        for action in actions.try_iter() {
            if exit_intent.is_some() {
                if action == Action::Terminate {
                    drop(cx.update(|_| termination::cancel_pending()));
                }
                continue;
            }
            match action {
                Action::ToggleAutomatic if automatic => {
                    automatic = false;
                    cancelled.store(true, Ordering::Release);
                    _hotkeys = None;
                    pending_inputs.clear();
                    gesture_tracker.clear();
                    request_restore(
                        &commands,
                        &mut worker_busy,
                        &mut worker_interruptible,
                        &mut restore_pending,
                        &mut restoring,
                    );
                    status_item::set_automatic(false);
                    status_item::set_status(if worker_busy {
                        "Paused"
                    } else {
                        "Paused · worker unavailable"
                    });
                }
                Action::ToggleAutomatic if restoring || restore_pending => {
                    status_item::set_status("Restoring windows...");
                }
                Action::ToggleAutomatic => {
                    if !macos::accessibility_trusted() {
                        last_error = Some("Accessibility permission is required".into());
                        status_item::set_status("Paused · Accessibility required");
                        continue;
                    }
                    match HotkeyPlan::current(&hotkey_workspaces)
                        .and_then(|plan| Hotkeys::install(actions_sender.clone(), plan))
                    {
                        Ok(registered) => {
                            _hotkeys = Some(registered);
                            automatic = true;
                            last_error = None;
                            cancelled.store(false, Ordering::Release);
                            events.invalidate();
                            status_item::set_automatic(true);
                            status_item::set_status(scope.automatic_status());
                        }
                        Err(error) => {
                            let error = format!("Hotkeys unavailable: {error}");
                            last_error = Some(error.clone());
                            request_restore(
                                &commands,
                                &mut worker_busy,
                                &mut worker_interruptible,
                                &mut restore_pending,
                                &mut restoring,
                            );
                            status_item::set_status(&error);
                        }
                    }
                }
                Action::OpenSettings => {
                    if let Err(error) = settings::open() {
                        status_item::set_status(&format!("Could not open Settings: {error}"));
                    }
                }
                Action::OpenAccessibility => permission_guide::show(),
                Action::CheckForUpdates => sparkle::check_for_updates(),
                Action::ToggleLaunchAtLogin => match login_item::toggle() {
                    Ok(status) => {
                        status_item::set_launch_at_login(status);
                        if status == login_item::Status::RequiresApproval {
                            status_item::set_status(
                                "Launch at Login requires approval in System Settings",
                            );
                        }
                    }
                    Err(error) => {
                        status_item::set_status(&format!(
                            "Could not change Launch at Login: {error}"
                        ));
                    }
                },
                Action::Restart | Action::Quit | Action::InstallUpdate | Action::Terminate => {
                    let intent = match action {
                        Action::Restart => ExitIntent::Restart,
                        Action::InstallUpdate => ExitIntent::Update,
                        Action::Terminate => ExitIntent::Terminate,
                        _ => ExitIntent::Quit,
                    };
                    automatic = false;
                    exit_intent = Some(intent);
                    cancelled.store(true, Ordering::Release);
                    _hotkeys = None;
                    pending_inputs.clear();
                    gesture_tracker.clear();
                    request_restore(
                        &commands,
                        &mut worker_busy,
                        &mut worker_interruptible,
                        &mut restore_pending,
                        &mut restoring,
                    );
                    status_item::set_automatic(false);
                    if worker_busy {
                        status_item::set_status(match intent {
                            ExitIntent::Quit => "Restoring windows...",
                            ExitIntent::Restart => "Restarting · restoring windows...",
                            ExitIntent::Update => "Update ready · restoring windows...",
                            ExitIntent::Terminate => "Terminating · restoring windows...",
                        });
                    } else {
                        cancel_unavailable_exit(cx, &mut exit_intent, &mut last_error);
                    }
                }
                Action::Worker(action) => {
                    if automatic {
                        queue_input(
                            &mut pending_inputs,
                            QueuedInput::Action(action),
                            &mut last_error,
                        );
                    }
                }
            }
        }

        if restoring
            && !worker_busy
            && next_restore_retry.is_some_and(|retry| Instant::now() >= retry)
        {
            next_restore_retry = None;
            send_restore(
                &commands,
                &mut worker_busy,
                &mut worker_interruptible,
                &mut restoring,
            );
            if !worker_busy {
                if exit_intent.is_some() {
                    cancel_unavailable_exit(cx, &mut exit_intent, &mut last_error);
                } else {
                    status_item::set_status("Paused · worker unavailable");
                }
            }
        }
        if automatic {
            if last_hotkey_refresh.elapsed() >= APPLICATION_REFRESH_INTERVAL {
                last_hotkey_refresh = Instant::now();
                let layout_changed = match _hotkeys.as_ref().map(Hotkeys::matches_current_layout) {
                    Some(Ok(matches)) => !matches,
                    Some(Err(error)) => {
                        let error = format!("Hotkeys unavailable: {error}");
                        last_error = Some(error.clone());
                        status_item::set_status(&error);
                        false
                    }
                    None => false,
                };
                if layout_changed {
                    match HotkeyPlan::current(&hotkey_workspaces) {
                        // Resolve and validate while old registrations remain active. Carbon
                        // cannot register the replacement chords until those are released.
                        Ok(plan)
                            if !_hotkeys
                                .as_ref()
                                .is_some_and(|hotkeys| plan.matches(hotkeys)) =>
                        {
                            _hotkeys = None;
                            match Hotkeys::install(actions_sender.clone(), plan) {
                                Ok(hotkeys) => {
                                    _hotkeys = Some(hotkeys);
                                    last_error = None;
                                }
                                Err(error) => {
                                    automatic = false;
                                    pending_inputs.clear();
                                    gesture_tracker.clear();
                                    cancelled.store(true, Ordering::Release);
                                    let error = format!("Hotkeys unavailable: {error}");
                                    last_error = Some(error.clone());
                                    request_restore(
                                        &commands,
                                        &mut worker_busy,
                                        &mut worker_interruptible,
                                        &mut restore_pending,
                                        &mut restoring,
                                    );
                                    status_item::set_automatic(false);
                                    status_item::set_status(&error);
                                }
                            }
                        }
                        Ok(_) => {}
                        Err(error) => {
                            let error = format!("Hotkeys unavailable: {error}");
                            last_error = Some(error.clone());
                            status_item::set_status(&error);
                        }
                    }
                }
            }
            if !automatic {
                // Hotkey failure may have paused inside this iteration.
                Timer::after(TICK_INTERVAL).await;
                continue;
            }
            let current_frontmost = macos::frontmost_application();
            if current_frontmost != frontmost_application {
                frontmost_application = current_frontmost;
                activation_refresh_at = Some(Instant::now() + APPLICATION_ACTIVATION_SETTLE);
                // Background cycles and previews yield to activation changes. Explicit
                // user commands (switch, move, focus) change the frontmost app
                // themselves; cancelling them here would abort our own work.
                if worker_busy && worker_interruptible {
                    cancelled.store(true, Ordering::Release);
                }
            }
            let activation_ready =
                activation_refresh_at.is_none_or(|refresh_at| Instant::now() >= refresh_at);
            if activation_ready && activation_refresh_at.take().is_some() {
                events.invalidate();
            }
            let mouse_down = macos::mouse_down();
            if !mouse_was_down && mouse_down {
                // Core Graphics hit-testing and readback do not send synchronous
                // AX messages to another app from the main-thread input loop.
                gesture_tracker.begin(macos::mouse_position().and_then(macos::window_at_point));
            }
            if mouse_was_down && !mouse_down {
                let final_frame = gesture_tracker
                    .window()
                    .and_then(macos::window_server_frame);
                if let Some(gesture) = gesture_tracker.finish(final_frame, macos::mouse_position())
                {
                    if worker_busy && worker_interruptible {
                        cancelled.store(true, Ordering::Release);
                    }
                    queue_input(
                        &mut pending_inputs,
                        QueuedInput::Gesture(gesture),
                        &mut last_error,
                    );
                }
                events.invalidate();
            } else if mouse_down
                && !worker_busy
                && pending_inputs.is_empty()
                && let Some(current) = gesture_tracker
                    .window()
                    .and_then(macos::window_server_frame)
                && let Some(gesture) = gesture_tracker.preview(current, macos::mouse_position())
            {
                send_with_outputs(
                    &commands,
                    |outputs| WorkerCommand::Gesture(gesture, true, outputs),
                    &mut worker_busy,
                    &mut worker_interruptible,
                );
            }
            mouse_was_down = mouse_down;
            if last_application_refresh.elapsed() >= APPLICATION_REFRESH_INTERVAL {
                events.refresh_applications();
                last_application_refresh = Instant::now();
            }
            if !worker_busy && let Some(input) = pending_inputs.front().cloned() {
                cancelled.store(false, Ordering::Release);
                send_with_outputs(
                    &commands,
                    |outputs| match input {
                        QueuedInput::Action(action) => WorkerCommand::Action(action, outputs),
                        QueuedInput::Gesture(gesture) => {
                            WorkerCommand::Gesture(gesture, false, outputs)
                        }
                    },
                    &mut worker_busy,
                    &mut worker_interruptible,
                );
                if worker_busy {
                    pending_inputs.pop();
                }
            }
            if !mouse_down && !worker_busy && pending_inputs.is_empty() && {
                let window_created = events.take_window_created();
                let event_dirty = (window_created || activation_ready) && events.take_dirty();
                refresh_due(
                    window_created,
                    event_dirty,
                    last_refresh.elapsed() >= FALLBACK_REFRESH_INTERVAL,
                    activation_ready,
                )
            } {
                send_with_outputs(
                    &commands,
                    |outputs| WorkerCommand::Action(WorkerAction::Retile, outputs),
                    &mut worker_busy,
                    &mut worker_interruptible,
                );
                if worker_busy {
                    last_refresh = Instant::now();
                }
            }
        }
        Timer::after(TICK_INTERVAL).await;
    }
}

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

fn send_restore(
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

fn cancel_unavailable_exit(
    cx: &mut gpui::AsyncApp,
    exit_intent: &mut Option<ExitIntent>,
    last_error: &mut Option<String>,
) {
    if exit_intent.take() == Some(ExitIntent::Terminate) {
        drop(cx.update(|_| termination::cancel_pending()));
    }
    *last_error = Some("Exit blocked because window restoration is unavailable".into());
    status_item::set_status("Exit blocked · restoration unavailable");
}

fn queue_input(queue: &mut InputQueue, input: QueuedInput, last_error: &mut Option<String>) {
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

fn send_with_outputs(
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

fn show_result(result: Result<CycleStatus, String>) {
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
fn settings_state(
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
