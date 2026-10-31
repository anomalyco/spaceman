//! The running Spaceman agent, over its control socket: what it reports each second, and the
//! requests the window sends it. Calls block, so they run off the main thread.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use rune::Waker;
use serde_json::Value;

const POLL: Duration = Duration::from_secs(1);

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Agent {
    pub accessibility: bool,
    pub automatic: bool,
    pub launch_at_login: LaunchAtLogin,
    pub update: Update,
    pub active: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum LaunchAtLogin {
    #[default]
    Off,
    On,
    NeedsApproval,
    Failed(String),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Update {
    /// Builds without Sparkle (dev builds).
    #[default]
    Unavailable,
    Idle,
    Checking,
    UpToDate,
    Available,
}

impl Agent {
    fn parse(json: &str) -> Option<Agent> {
        let value: Value = serde_json::from_str(json).ok()?;
        let launch = value["launch_at_login"].as_str().unwrap_or_default();
        Some(Agent {
            accessibility: value["accessibility"].as_bool().unwrap_or(false),
            automatic: value["automatic"].as_bool().unwrap_or(false),
            launch_at_login: match launch {
                "enabled" => LaunchAtLogin::On,
                "disabled" => LaunchAtLogin::Off,
                "requires-approval" => LaunchAtLogin::NeedsApproval,
                other => {
                    LaunchAtLogin::Failed(other.strip_prefix("error: ").unwrap_or(other).to_owned())
                }
            },
            update: match value["update"].as_str().unwrap_or_default() {
                "idle" => Update::Idle,
                "checking" => Update::Checking,
                "up-to-date" => Update::UpToDate,
                "available" => Update::Available,
                _ => Update::Unavailable,
            },
            active: value["active_workspaces"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|id| match id {
                    Value::String(name) => name.clone(),
                    other => other.to_string(),
                })
                .collect(),
        })
    }
}

/// What the poller found since the last frame.
#[derive(Default)]
pub struct Inbox {
    /// `Some(None)`: the agent isn't running.
    pub agent: Option<Option<Agent>>,
    /// config.toml changed on disk.
    pub config_changed: bool,
}

pub type Shared = Arc<Mutex<Inbox>>;

/// Polls the agent and the config file's timestamp, waking the window when either changes.
pub fn start(waker: Waker) -> Shared {
    let shared = Shared::default();
    let inbox = shared.clone();
    if let Err(error) = std::thread::Builder::new()
        .name("spaceman-agent-poll".into())
        .spawn(move || {
            let mut last: Option<Option<Agent>> = None;
            let mut stamp = spaceman::config::modified();
            loop {
                let agent = spaceman::control::send(&["settings".into()])
                    .ok()
                    .and_then(|json| Agent::parse(&json));
                let now = spaceman::config::modified();
                let config_changed = now != stamp;
                stamp = now;
                if last.as_ref() != Some(&agent) || config_changed {
                    let mut inbox = inbox
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    if last.as_ref() != Some(&agent) {
                        inbox.agent = Some(agent.clone());
                    }
                    inbox.config_changed |= config_changed;
                    last = Some(agent);
                    drop(inbox);
                    waker();
                }
                std::thread::sleep(POLL);
            }
        })
    {
        eprintln!("could not start the agent poll: {error}");
    }
    shared
}

/// Sends `command` to the agent without waiting for it.
pub fn send(command: &'static str) {
    std::thread::spawn(move || {
        drop(spaceman::control::send(&[command.into()]));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_agents_settings_state() {
        let agent = Agent::parse(
            r#"{"accessibility":true,"automatic":true,"launch_at_login":"requires-approval",
                "update":"checking","active_workspaces":[3,"M"]}"#,
        )
        .unwrap();
        assert!(agent.accessibility);
        assert_eq!(agent.launch_at_login, LaunchAtLogin::NeedsApproval);
        assert_eq!(agent.update, Update::Checking);
        assert_eq!(agent.active, vec!["3".to_string(), "M".to_string()]);
    }
}
