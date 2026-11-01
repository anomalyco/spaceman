//! Agent state shown in the Rune window, fed directly by the in-process runtime bridge.

#[cfg(test)]
use serde_json::Value;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Agent {
    pub accessibility: bool,
    pub automatic: bool,
    pub hud: bool,
    pub launch_at_login: LaunchAtLogin,
    pub update: Update,
    pub active: Vec<String>,
    pub last_error: Option<String>,
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
    #[cfg(target_os = "macos")]
    #[must_use]
    pub fn from_snapshot(snapshot: spaceman::app::UiAgentSnapshot) -> Self {
        use spaceman::app::{UiLaunchAtLogin, UiUpdateStatus};

        Self {
            accessibility: snapshot.accessibility,
            automatic: snapshot.automatic,
            hud: snapshot.hud,
            launch_at_login: match snapshot.launch_at_login {
                UiLaunchAtLogin::Off => LaunchAtLogin::Off,
                UiLaunchAtLogin::On => LaunchAtLogin::On,
                UiLaunchAtLogin::NeedsApproval => LaunchAtLogin::NeedsApproval,
                UiLaunchAtLogin::Failed(error) => LaunchAtLogin::Failed(error),
            },
            update: match snapshot.update {
                UiUpdateStatus::Unavailable => Update::Unavailable,
                UiUpdateStatus::Idle => Update::Idle,
                UiUpdateStatus::Checking => Update::Checking,
                UiUpdateStatus::UpToDate => Update::UpToDate,
                UiUpdateStatus::Available => Update::Available,
            },
            active: snapshot
                .active_workspaces
                .into_iter()
                .map(|workspace| workspace.to_string())
                .collect(),
            last_error: snapshot.last_error,
        }
    }

    #[cfg(test)]
    fn parse(json: &str) -> Option<Agent> {
        let value: Value = serde_json::from_str(json).ok()?;
        let launch = value["launch_at_login"].as_str().unwrap_or_default();
        Some(Agent {
            accessibility: value["accessibility"].as_bool().unwrap_or(false),
            automatic: value["automatic"].as_bool().unwrap_or(false),
            hud: value["hud"].as_bool().unwrap_or(true),
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
            last_error: value["last_error"].as_str().map(str::to_owned),
        })
    }
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

    #[cfg(target_os = "macos")]
    #[test]
    fn converts_in_process_snapshot() {
        use spaceman::{
            WorkspaceId,
            app::{UiAgentSnapshot, UiLaunchAtLogin, UiUpdateStatus},
        };

        let agent = Agent::from_snapshot(UiAgentSnapshot {
            accessibility: true,
            automatic: true,
            hud: true,
            launch_at_login: UiLaunchAtLogin::On,
            update: UiUpdateStatus::UpToDate,
            active_workspaces: vec![WorkspaceId::ONE, WorkspaceId::SIX],
            last_error: None,
        });
        assert!(agent.accessibility);
        assert!(agent.automatic);
        assert_eq!(agent.launch_at_login, LaunchAtLogin::On);
        assert_eq!(agent.update, Update::UpToDate);
        assert_eq!(agent.active, vec!["1".to_string(), "6".to_string()]);
    }
}
