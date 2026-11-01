use std::time::SystemTime;

use super::{
    worker::{
        cancel_unavailable_exit, queue_input, request_restore, send_restore, send_with_outputs,
        settings_state, show_result,
    },
    *,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TickOutcome {
    Continue,
    /// Clean quit or restart after parked windows restored: exit the Rune event loop.
    Quit,
    /// AppKit termination (`applicationShouldTerminate:`) finished restoring; reply `true`.
    TerminateAppKit,
}

pub(super) struct AppLoopInit {
    pub(super) actions: Receiver<Action>,
    pub(super) actions_sender: Sender<Action>,
    pub(super) commands: Sender<WorkerCommand>,
    pub(super) results: Receiver<WorkerResult>,
    pub(super) cancelled: Arc<AtomicBool>,
    pub(super) scope: ManagementScope,
    pub(super) events: macos::WindowEventMonitor,
    pub(super) hotkey_workspaces: Vec<WorkspaceId>,
    pub(super) hud: bool,
    pub(super) controls: Receiver<ControlRequest>,
    pub(super) control_server: control::Server,
    pub(super) bridge: UiBridge,
    pub(super) callbacks: HostCallbacks,
}

pub struct MainThreadAgent {
    actions: Receiver<Action>,
    actions_sender: Sender<Action>,
    commands: Sender<WorkerCommand>,
    results: Receiver<WorkerResult>,
    cancelled: Arc<AtomicBool>,
    scope: ManagementScope,
    events: macos::WindowEventMonitor,
    hotkey_workspaces: Vec<WorkspaceId>,
    controls: Receiver<ControlRequest>,
    _control_server: control::Server,
    bridge: UiBridge,
    callbacks: HostCallbacks,
    hotkeys: Option<Hotkeys>,
    automatic: bool,
    hud: bool,
    awaiting_accessibility: bool,
    worker_busy: bool,
    worker_interruptible: bool,
    exit_intent: Option<ExitIntent>,
    restore_pending: bool,
    restoring: bool,
    next_restore_retry: Option<Instant>,
    active_workspaces: Vec<WorkspaceId>,
    last_error: Option<String>,
    latest_debug: Option<RuntimeDebug>,
    pending_inputs: InputQueue,
    gesture_tracker: GestureTracker,
    last_refresh: Instant,
    last_application_refresh: Instant,
    last_hotkey_refresh: Instant,
    last_ui_refresh: Instant,
    config_stamp: Option<SystemTime>,
    launch_at_login: UiLaunchAtLogin,
    ui_dirty: bool,
    mouse_was_down: bool,
    frontmost_application: Option<AppInstanceId>,
    activation_refresh_at: Option<Instant>,
}

impl MainThreadAgent {
    pub(super) fn new(
        init: AppLoopInit,
        launch_status: Result<login_item::Status, String>,
    ) -> Self {
        let trusted = macos::accessibility_trusted();
        let (hotkeys, hotkey_error) = if trusted {
            match HotkeyPlan::current(&init.hotkey_workspaces)
                .and_then(|plan| Hotkeys::install(init.actions_sender.clone(), plan))
            {
                Ok(hotkeys) => (Some(hotkeys), None),
                Err(error) => (None, Some(format!("Hotkeys unavailable: {error}"))),
            }
        } else {
            (None, Some("Accessibility permission is required".into()))
        };
        let automatic = hotkeys.is_some();
        let mut worker_busy = false;
        let mut worker_interruptible = false;
        let mut restoring = false;

        status_item::set_automatic(automatic);
        if automatic {
            status_item::set_status("Automatic tiling enabled");
            send_with_outputs(
                &init.commands,
                |outputs| WorkerCommand::Action(WorkerAction::Retile, outputs),
                &mut worker_busy,
                &mut worker_interruptible,
            );
        } else {
            status_item::set_status(hotkey_error.as_deref().unwrap_or("Paused"));
            send_restore(
                &init.commands,
                &mut worker_busy,
                &mut worker_interruptible,
                &mut restoring,
            );
        }

        let initial_refresh = Instant::now() - FALLBACK_REFRESH_INTERVAL;
        let agent = Self {
            actions: init.actions,
            actions_sender: init.actions_sender,
            commands: init.commands,
            results: init.results,
            cancelled: init.cancelled,
            scope: init.scope,
            events: init.events,
            hotkey_workspaces: init.hotkey_workspaces,
            controls: init.controls,
            _control_server: init.control_server,
            bridge: init.bridge,
            callbacks: init.callbacks,
            hotkeys,
            automatic,
            hud: init.hud,
            awaiting_accessibility: !trusted,
            worker_busy,
            worker_interruptible,
            exit_intent: None,
            restore_pending: false,
            restoring,
            next_restore_retry: None,
            active_workspaces: init.scope.initial_workspaces().to_vec(),
            last_error: hotkey_error,
            latest_debug: None,
            pending_inputs: InputQueue::default(),
            gesture_tracker: GestureTracker::default(),
            last_refresh: initial_refresh,
            last_application_refresh: initial_refresh,
            last_hotkey_refresh: initial_refresh,
            last_ui_refresh: Instant::now(),
            config_stamp: crate::config::modified(),
            launch_at_login: UiLaunchAtLogin::from_result(launch_status),
            ui_dirty: true,
            mouse_was_down: macos::mouse_down(),
            frontmost_application: macos::frontmost_application(),
            activation_refresh_at: None,
        };
        agent.publish_ui(false);
        agent
    }

    pub(super) fn set_callbacks(&mut self, callbacks: HostCallbacks) {
        self.callbacks = callbacks;
    }

    pub fn tick(&mut self) -> TickOutcome {
        loop {
            let result = match self.results.try_recv() {
                Ok(result) => result,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.hotkeys = None;
                    self.pending_inputs.clear();
                    self.gesture_tracker.clear();
                    status_item::set_automatic(false);
                    self.automatic = false;
                    self.worker_busy = false;
                    self.restore_pending = false;
                    self.restoring = false;
                    self.next_restore_retry = None;
                    self.ui_dirty = true;
                    if self.exit_intent.is_some() {
                        cancel_unavailable_exit(&mut self.exit_intent, &mut self.last_error);
                    } else {
                        status_item::set_status("Worker stopped unexpectedly");
                    }
                    break;
                }
            };
            match result {
                WorkerResult::Status { result, debug } if self.automatic => {
                    self.worker_busy = false;
                    self.cancelled.store(false, Ordering::Release);
                    self.latest_debug = Some(*debug);
                    // Newer input routinely cancels background work; only
                    // genuine failures are worth a log line.
                    if let Err(error) = &result
                        && self.last_error.as_ref() != Some(error)
                        && !error.ends_with("cancelled")
                    {
                        log_event!("worker error: {error}");
                    }
                    self.last_error = result.as_ref().err().cloned();
                    if let Ok(status) = &result {
                        self.active_workspaces.clone_from(&status.active);
                        if self.hud
                            && let Some(hud) = status.hud.clone()
                        {
                            self.bridge.publish_hud(hud);
                        }
                    }
                    self.ui_dirty = true;
                    show_result(result);
                }
                WorkerResult::Status { result, debug } => {
                    self.worker_busy = false;
                    self.latest_debug = Some(*debug);
                    if let Err(error) = result {
                        self.last_error = Some(error);
                        self.ui_dirty = true;
                    }
                }
                WorkerResult::Restored { failed: 0 }
                    if self.exit_intent == Some(ExitIntent::Update) =>
                {
                    self.worker_busy = false;
                    self.restoring = false;
                    match sparkle::continue_install() {
                        Ok(()) => {
                            termination::allow_next();
                            status_item::set_status("Installing update...");
                        }
                        Err(error) => {
                            log_event!("Sparkle continuation failed after restoration: {error}");
                            termination::allow_next();
                            return TickOutcome::Quit;
                        }
                    }
                }
                WorkerResult::Restored { failed: 0 }
                    if self.exit_intent == Some(ExitIntent::Terminate) =>
                {
                    drop(self.commands.send(WorkerCommand::Stop));
                    return TickOutcome::TerminateAppKit;
                }
                WorkerResult::Restored { failed: 0 } if self.exit_intent.is_some() => {
                    drop(self.commands.send(WorkerCommand::Stop));
                    termination::allow_next();
                    return TickOutcome::Quit;
                }
                WorkerResult::Restored { failed: 0 } => {
                    self.worker_busy = false;
                    self.restoring = false;
                    status_item::set_status(self.last_error.as_deref().unwrap_or("Paused"));
                }
                WorkerResult::Restored { failed } => {
                    self.worker_busy = false;
                    self.restoring = true;
                    self.next_restore_retry = Some(Instant::now() + RESTORE_RETRY_INTERVAL);
                    status_item::set_status(&format!("Restoring windows · {failed} failed"));
                }
            }
            if self.restore_pending && !self.worker_busy {
                self.restore_pending = false;
                send_restore(
                    &self.commands,
                    &mut self.worker_busy,
                    &mut self.worker_interruptible,
                    &mut self.restoring,
                );
                if !self.worker_busy && self.exit_intent.is_some() {
                    cancel_unavailable_exit(&mut self.exit_intent, &mut self.last_error);
                    self.ui_dirty = true;
                }
            }
        }

        self.drain_controls();
        self.drain_actions();

        if self.restoring
            && !self.worker_busy
            && self
                .next_restore_retry
                .is_some_and(|retry| Instant::now() >= retry)
        {
            self.next_restore_retry = None;
            send_restore(
                &self.commands,
                &mut self.worker_busy,
                &mut self.worker_interruptible,
                &mut self.restoring,
            );
            if !self.worker_busy {
                if self.exit_intent.is_some() {
                    cancel_unavailable_exit(&mut self.exit_intent, &mut self.last_error);
                    self.ui_dirty = true;
                } else {
                    status_item::set_status("Paused · worker unavailable");
                }
            }
        }

        if self.automatic {
            self.tick_automatic();
        }

        self.maybe_publish_ui();
        TickOutcome::Continue
    }

    fn drain_controls(&mut self) {
        for request in self.controls.try_iter() {
            let response = match request.command {
                ControlCommand::Status => Ok(serde_json::json!({
                    "mode": if let Some(intent) = self.exit_intent {
                        intent.mode_label()
                    } else if self.restoring || self.restore_pending {
                        "restoring"
                    } else if self.automatic {
                        "automatic"
                    } else {
                        "paused"
                    },
                    "busy": self.worker_busy,
                    "last_error": self.last_error.as_deref(),
                    "active_workspaces": self.active_workspaces,
                })
                .to_string()),
                ControlCommand::Settings => Ok(settings_state(
                    self.automatic,
                    &self.active_workspaces,
                    self.last_error.as_deref(),
                )
                .to_string()),
                ControlCommand::App(ref action) => {
                    queue(&self.actions_sender, action.clone(), "queued")
                }
                ControlCommand::Debug => self
                    .latest_debug
                    .as_ref()
                    .ok_or_else(|| "runtime debug state is not ready".into())
                    .and_then(|debug| {
                        serde_json::to_string(debug).map_err(|error| error.to_string())
                    }),
                ControlCommand::Queue(ref action) if self.automatic => queue(
                    &self.actions_sender,
                    Action::Worker(action.clone()),
                    &action.queued_acknowledgement(),
                ),
                ControlCommand::Queue(_) => Err("automatic tiling is paused".into()),
                ControlCommand::Restart if self.worker_busy => {
                    Err("Spaceman is busy; retry restart".into())
                }
                ControlCommand::Restart => {
                    queue(&self.actions_sender, Action::Restart, "restart queued")
                }
                ControlCommand::Pause if self.automatic => queue(
                    &self.actions_sender,
                    Action::ToggleAutomatic,
                    "pause queued",
                ),
                ControlCommand::Pause => Ok("already paused".into()),
                ControlCommand::Resume if self.restoring || self.restore_pending => {
                    Err("windows are still being restored".into())
                }
                ControlCommand::Resume if self.automatic => Ok("already automatic".into()),
                ControlCommand::Resume => queue(
                    &self.actions_sender,
                    Action::ToggleAutomatic,
                    "resume queued",
                ),
                ControlCommand::Quit => queue(&self.actions_sender, Action::Quit, "quit queued"),
            };
            request.respond(response);
        }
    }

    fn drain_actions(&mut self) {
        while let Ok(action) = self.actions.try_recv() {
            if self.exit_intent.is_some() {
                if action == Action::Terminate {
                    termination::cancel_pending();
                }
                continue;
            }
            match action {
                Action::ToggleAutomatic if self.automatic => {
                    self.automatic = false;
                    self.awaiting_accessibility = false;
                    self.cancelled.store(true, Ordering::Release);
                    self.hotkeys = None;
                    self.pending_inputs.clear();
                    self.gesture_tracker.clear();
                    request_restore(
                        &self.commands,
                        &mut self.worker_busy,
                        &mut self.worker_interruptible,
                        &mut self.restore_pending,
                        &mut self.restoring,
                    );
                    self.ui_dirty = true;
                    status_item::set_automatic(false);
                    status_item::set_status(if self.worker_busy {
                        "Paused"
                    } else {
                        "Paused · worker unavailable"
                    });
                }
                Action::ToggleAutomatic if self.restoring || self.restore_pending => {
                    status_item::set_status("Restoring windows...");
                }
                Action::ToggleAutomatic => {
                    if !macos::accessibility_trusted() {
                        self.awaiting_accessibility = true;
                        self.last_error = Some("Accessibility permission is required".into());
                        self.ui_dirty = true;
                        status_item::set_status("Paused · Accessibility required");
                        continue;
                    }
                    self.awaiting_accessibility = false;
                    match HotkeyPlan::current(&self.hotkey_workspaces)
                        .and_then(|plan| Hotkeys::install(self.actions_sender.clone(), plan))
                    {
                        Ok(registered) => {
                            self.hotkeys = Some(registered);
                            self.automatic = true;
                            self.last_error = None;
                            self.cancelled.store(false, Ordering::Release);
                            self.events.invalidate();
                            self.ui_dirty = true;
                            status_item::set_automatic(true);
                            status_item::set_status(self.scope.automatic_status());
                        }
                        Err(error) => {
                            let error = format!("Hotkeys unavailable: {error}");
                            self.last_error = Some(error.clone());
                            self.ui_dirty = true;
                            request_restore(
                                &self.commands,
                                &mut self.worker_busy,
                                &mut self.worker_interruptible,
                                &mut self.restore_pending,
                                &mut self.restoring,
                            );
                            status_item::set_status(&error);
                        }
                    }
                }
                Action::OpenSettings => {
                    if let Err(error) = (self.callbacks.open_settings)() {
                        status_item::set_status(&format!("Could not open Settings: {error}"));
                    }
                }
                Action::OpenSetup => {
                    if let Err(error) = (self.callbacks.open_setup)() {
                        status_item::set_status(&format!("Could not open Setup: {error}"));
                    }
                }
                Action::OpenAccessibility => permission_guide::show(),
                Action::ToggleHud => {
                    let next = !self.hud;
                    match (self.callbacks.save_hud)(next) {
                        Ok(()) => {
                            self.hud = next;
                            self.config_stamp = crate::config::modified();
                            status_item::set_hud(next);
                            self.publish_ui(true);
                        }
                        Err(error) => {
                            status_item::set_status(&format!(
                                "Could not change Workspace HUD: {error}"
                            ));
                        }
                    }
                }
                Action::SetHud(enabled) => {
                    self.hud = enabled;
                    self.config_stamp = crate::config::modified();
                    status_item::set_hud(enabled);
                    self.ui_dirty = true;
                }
                Action::CheckForUpdates => {
                    sparkle::check_for_updates();
                    self.ui_dirty = true;
                }
                Action::ToggleLaunchAtLogin => {
                    let result = login_item::toggle();
                    self.launch_at_login = UiLaunchAtLogin::from_result(result.clone());
                    self.ui_dirty = true;
                    match result {
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
                    }
                }
                Action::Restart | Action::Quit | Action::InstallUpdate | Action::Terminate => {
                    let intent = match action {
                        Action::Restart => ExitIntent::Restart,
                        Action::InstallUpdate => ExitIntent::Update,
                        Action::Terminate => ExitIntent::Terminate,
                        _ => ExitIntent::Quit,
                    };
                    self.automatic = false;
                    self.exit_intent = Some(intent);
                    self.cancelled.store(true, Ordering::Release);
                    self.hotkeys = None;
                    self.pending_inputs.clear();
                    self.gesture_tracker.clear();
                    self.ui_dirty = true;
                    request_restore(
                        &self.commands,
                        &mut self.worker_busy,
                        &mut self.worker_interruptible,
                        &mut self.restore_pending,
                        &mut self.restoring,
                    );
                    status_item::set_automatic(false);
                    if self.worker_busy {
                        status_item::set_status(match intent {
                            ExitIntent::Quit => "Restoring windows...",
                            ExitIntent::Restart => "Restarting · restoring windows...",
                            ExitIntent::Update => "Update ready · restoring windows...",
                            ExitIntent::Terminate => "Terminating · restoring windows...",
                        });
                    } else {
                        cancel_unavailable_exit(&mut self.exit_intent, &mut self.last_error);
                    }
                }
                Action::Worker(action) => {
                    if self.automatic {
                        queue_input(
                            &mut self.pending_inputs,
                            QueuedInput::Action(action),
                            &mut self.last_error,
                        );
                        self.ui_dirty = true;
                    }
                }
            }
        }
    }

    fn tick_automatic(&mut self) {
        if self.last_hotkey_refresh.elapsed() >= APPLICATION_REFRESH_INTERVAL {
            self.last_hotkey_refresh = Instant::now();
            let layout_changed = match self.hotkeys.as_ref().map(Hotkeys::matches_current_layout) {
                Some(Ok(matches)) => !matches,
                Some(Err(error)) => {
                    let error = format!("Hotkeys unavailable: {error}");
                    self.last_error = Some(error.clone());
                    self.ui_dirty = true;
                    status_item::set_status(&error);
                    false
                }
                None => false,
            };
            if layout_changed {
                match HotkeyPlan::current(&self.hotkey_workspaces) {
                    // Resolve and validate while old registrations remain active. Carbon
                    // cannot register the replacement chords until those are released.
                    Ok(plan)
                        if !self
                            .hotkeys
                            .as_ref()
                            .is_some_and(|hotkeys| plan.matches(hotkeys)) =>
                    {
                        self.hotkeys = None;
                        match Hotkeys::install(self.actions_sender.clone(), plan) {
                            Ok(hotkeys) => {
                                self.hotkeys = Some(hotkeys);
                                self.last_error = None;
                                self.ui_dirty = true;
                            }
                            Err(error) => {
                                self.automatic = false;
                                self.pending_inputs.clear();
                                self.gesture_tracker.clear();
                                self.cancelled.store(true, Ordering::Release);
                                let error = format!("Hotkeys unavailable: {error}");
                                self.last_error = Some(error.clone());
                                self.ui_dirty = true;
                                request_restore(
                                    &self.commands,
                                    &mut self.worker_busy,
                                    &mut self.worker_interruptible,
                                    &mut self.restore_pending,
                                    &mut self.restoring,
                                );
                                status_item::set_automatic(false);
                                status_item::set_status(&error);
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(error) => {
                        let error = format!("Hotkeys unavailable: {error}");
                        self.last_error = Some(error.clone());
                        self.ui_dirty = true;
                        status_item::set_status(&error);
                    }
                }
            }
        }
        if !self.automatic {
            // Hotkey failure may have paused inside this iteration.
            return;
        }
        let current_frontmost = macos::frontmost_application();
        if current_frontmost != self.frontmost_application {
            self.frontmost_application = current_frontmost;
            self.activation_refresh_at = Some(Instant::now() + APPLICATION_ACTIVATION_SETTLE);
            // Background cycles and previews yield to activation changes. Explicit
            // user commands (switch, move, focus) change the frontmost app
            // themselves; cancelling them here would abort our own work.
            if self.worker_busy && self.worker_interruptible {
                self.cancelled.store(true, Ordering::Release);
            }
        }
        let activation_ready = self
            .activation_refresh_at
            .is_none_or(|refresh_at| Instant::now() >= refresh_at);
        if activation_ready && self.activation_refresh_at.take().is_some() {
            self.events.invalidate();
        }
        let mouse_down = macos::mouse_down();
        if !self.mouse_was_down && mouse_down {
            // Core Graphics hit-testing and readback do not send synchronous
            // AX messages to another app from the main-thread input loop.
            self.gesture_tracker
                .begin(macos::mouse_position().and_then(macos::window_at_point));
        }
        if self.mouse_was_down && !mouse_down {
            let final_frame = self
                .gesture_tracker
                .window()
                .and_then(macos::window_server_frame);
            if let Some(gesture) = self
                .gesture_tracker
                .finish(final_frame, macos::mouse_position())
            {
                if self.worker_busy && self.worker_interruptible {
                    self.cancelled.store(true, Ordering::Release);
                }
                queue_input(
                    &mut self.pending_inputs,
                    QueuedInput::Gesture(gesture),
                    &mut self.last_error,
                );
                self.ui_dirty = true;
            }
            self.events.invalidate();
        } else if mouse_down
            && !self.worker_busy
            && self.pending_inputs.is_empty()
            && let Some(current) = self
                .gesture_tracker
                .window()
                .and_then(macos::window_server_frame)
            && let Some(gesture) = self
                .gesture_tracker
                .preview(current, macos::mouse_position())
        {
            send_with_outputs(
                &self.commands,
                |outputs| WorkerCommand::Gesture(gesture, true, outputs),
                &mut self.worker_busy,
                &mut self.worker_interruptible,
            );
        }
        self.mouse_was_down = mouse_down;
        if self.last_application_refresh.elapsed() >= APPLICATION_REFRESH_INTERVAL {
            self.events.refresh_applications();
            self.last_application_refresh = Instant::now();
        }
        if !self.worker_busy
            && let Some(input) = self.pending_inputs.front().cloned()
        {
            self.cancelled.store(false, Ordering::Release);
            send_with_outputs(
                &self.commands,
                |outputs| match input {
                    QueuedInput::Action(action) => WorkerCommand::Action(action, outputs),
                    QueuedInput::Gesture(gesture) => {
                        WorkerCommand::Gesture(gesture, false, outputs)
                    }
                },
                &mut self.worker_busy,
                &mut self.worker_interruptible,
            );
            if self.worker_busy {
                self.pending_inputs.pop();
            }
        }
        if !mouse_down && !self.worker_busy && self.pending_inputs.is_empty() && {
            let window_created = self.events.take_window_created();
            let event_dirty = (window_created || activation_ready) && self.events.take_dirty();
            refresh_due(
                window_created,
                event_dirty,
                self.last_refresh.elapsed() >= FALLBACK_REFRESH_INTERVAL,
                activation_ready,
            )
        } {
            send_with_outputs(
                &self.commands,
                |outputs| WorkerCommand::Action(WorkerAction::Retile, outputs),
                &mut self.worker_busy,
                &mut self.worker_interruptible,
            );
            if self.worker_busy {
                self.last_refresh = Instant::now();
            }
        }
    }

    fn maybe_publish_ui(&mut self) {
        let periodic = self.last_ui_refresh.elapsed() >= APPLICATION_REFRESH_INTERVAL;
        if !self.ui_dirty && !periodic {
            return;
        }
        let mut config_changed = false;
        if periodic {
            self.last_ui_refresh = Instant::now();
            self.launch_at_login = UiLaunchAtLogin::from_result(login_item::status());
            let stamp = crate::config::modified();
            config_changed = stamp != self.config_stamp;
            self.config_stamp = stamp;
            if config_changed && let Ok(config) = Config::load() {
                self.hud = config.hud();
                status_item::set_hud(self.hud);
            }
        }
        self.ui_dirty = false;
        let trusted = macos::accessibility_trusted();
        status_item::set_accessibility(trusted);
        if self.awaiting_accessibility
            && trusted
            && !self.automatic
            && !self.restoring
            && !self.restore_pending
            && self.exit_intent.is_none()
        {
            self.awaiting_accessibility = false;
            drop(self.actions_sender.send(Action::ToggleAutomatic));
        }
        self.publish_ui(config_changed);
    }

    fn publish_ui(&self, config_changed: bool) {
        self.bridge.publish(
            UiAgentSnapshot {
                accessibility: macos::accessibility_trusted(),
                automatic: self.automatic,
                hud: self.hud,
                launch_at_login: self.launch_at_login.clone(),
                update: sparkle::status().into(),
                active_workspaces: self.active_workspaces.clone(),
                last_error: self.last_error.clone(),
            },
            config_changed,
        );
    }
}
