use std::{
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, SystemTime},
};

use crate::{config::Config, login_item};

/// Settings the agent polls but only the user changes. Reading them means an XPC
/// round trip to `smd` and file-system calls, each of which can stall for a
/// second or more while the disk or launchd is busy, so a background thread
/// reads them and the main thread only ever receives the results.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Probe {
    pub(super) launch_at_login: Result<login_item::Status, String>,
    pub(super) config_stamp: Option<SystemTime>,
    /// The HUD setting read for `config_stamp`, when that config loaded.
    pub(super) hud: Option<bool>,
}

/// Sends a [`Probe`] whenever one changes, checking every `interval`. The thread
/// ends when the receiver is dropped.
pub(super) fn spawn(interval: Duration) -> Receiver<Probe> {
    let (sender, receiver) = mpsc::channel();
    let spawned = thread::Builder::new()
        .name("spaceman-probe".into())
        .spawn(move || {
            let mut last: Option<Probe> = None;
            loop {
                let config_stamp = crate::config::modified();
                let hud = match &last {
                    Some(previous) if previous.config_stamp == config_stamp => previous.hud,
                    _ => Config::load().ok().map(|config| config.hud()),
                };
                let probe = Probe {
                    launch_at_login: login_item::status(),
                    config_stamp,
                    hud,
                };
                if last.as_ref() != Some(&probe) {
                    if sender.send(probe.clone()).is_err() {
                        return;
                    }
                    last = Some(probe);
                }
                thread::sleep(interval);
            }
        });
    if let Err(error) = spawned {
        log_event!("settings probe unavailable: {error}");
    }
    receiver
}
