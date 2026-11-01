use super::*;

impl MainThreadAgent {
    pub(super) fn drain_actions(&mut self) {
        while let Ok(action) = self.actions.try_recv() {
            if self.exit_intent.is_some() {
                if action == Action::Terminate {
                    termination::cancel_pending();
                }
                continue;
            }
            match action {
                Action::ToggleAutomatic if self.automatic => {
                    self.awaiting_accessibility = false;
                    self.stop_automatic_and_restore();
                    status_item::set_status(if self.worker.is_busy() {
                        "Paused"
                    } else {
                        "Paused · worker unavailable"
                    });
                }
                Action::ToggleAutomatic if self.worker.is_restoring_or_pending() => {
                    status_item::set_status("Restoring windows...");
                }
                Action::ToggleAutomatic => {
                    let trusted = macos::accessibility_trusted();
                    self.accessibility = trusted;
                    if !trusted {
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
                            self.worker.request_restore();
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
                    self.exit_intent = Some(intent);
                    self.stop_automatic_and_restore();
                    if self.worker.is_busy() {
                        status_item::set_status(match intent {
                            ExitIntent::Quit => "Restoring windows...",
                            ExitIntent::Restart => "Restarting · restoring windows...",
                            ExitIntent::Update => "Update ready · restoring windows...",
                            ExitIntent::Terminate => "Terminating · restoring windows...",
                        });
                    } else {
                        self.cancel_unavailable_exit();
                    }
                }
                Action::Worker(action) => {
                    if self.automatic {
                        self.queue_worker_input(QueuedInput::Action(action));
                    }
                }
            }
        }
    }
}
