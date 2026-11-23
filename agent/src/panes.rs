//! Settings pane registry and playground scenes.

mod general;
mod shortcuts;
mod workspaces;

use rune::{El, Icon};
use spaceman::app::{UiLaunchAtLogin, UiUpdateStatus};

use crate::{
    Ui, activity,
    controls::Paint,
    labs, map,
    playground::{Scene, reset_fixture},
};

static GENERAL_ICON: Icon = Icon {
    name: "spaceman-general",
    view: 16.0,
    body: include_str!("../assets/general.svg"),
};
static RULES_ICON: Icon = Icon {
    name: "spaceman-rules",
    view: 16.0,
    body: include_str!("../assets/rules.svg"),
};
static WORKSPACES_ICON: Icon = Icon {
    name: "spaceman-workspaces",
    view: 16.0,
    body: include_str!("../assets/workspaces.svg"),
};
static ACTIVITY_ICON: Icon = Icon {
    name: "spaceman-activity",
    view: 24.0,
    body: include_str!("../assets/activity.svg"),
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pane {
    General,
    Rules,
    Workspaces,
    Shortcuts,
    Activity,
    Labs,
}

impl Pane {
    pub const ALL: [Pane; 6] = [
        Pane::General,
        Pane::Rules,
        Pane::Workspaces,
        Pane::Shortcuts,
        Pane::Activity,
        Pane::Labs,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Pane::General => "general",
            Pane::Rules => "rules",
            Pane::Workspaces => "workspaces",
            Pane::Shortcuts => "shortcuts",
            Pane::Activity => "activity",
            Pane::Labs => "labs",
        }
    }

    pub const fn title(self) -> &'static str {
        match self {
            Pane::General => "General",
            Pane::Rules => "Rules",
            Pane::Workspaces => "Workspaces",
            Pane::Shortcuts => "Shortcuts",
            Pane::Activity => "Activity",
            Pane::Labs => "Labs",
        }
    }

    pub const fn icon(self) -> &'static Icon {
        match self {
            Pane::General => &GENERAL_ICON,
            Pane::Rules => &RULES_ICON,
            Pane::Workspaces => &WORKSPACES_ICON,
            Pane::Shortcuts => &rune_theme_opencode::icons::KEYBOARD,
            Pane::Activity => &ACTIVITY_ICON,
            Pane::Labs => &labs::LABS_ICON,
        }
    }

    pub fn render(self, ui: &Ui, p: Paint, layer: &mut Vec<El>) -> El {
        match self {
            Pane::General => general::pane(ui, p),
            Pane::Rules => map::pane(ui, p, layer),
            Pane::Workspaces => workspaces::pane(ui, p),
            Pane::Shortcuts => shortcuts::pane(p),
            Pane::Activity => activity::pane(ui, p),
            Pane::Labs => labs::pane(ui, p),
        }
    }

    /// Maps `⌘1`..`⌘N` in sidebar order.
    pub fn from_shortcut(digit: char) -> Option<Pane> {
        let index = digit.to_digit(10)?.checked_sub(1)? as usize;
        Self::ALL.get(index).copied()
    }
}

pub(crate) const SCENES: &[Scene] = &[
    ("general", |ui| reset_fixture(ui, Pane::General)),
    ("general-setup", |ui| {
        reset_fixture(ui, Pane::General);
        if let Some(agent) = &mut ui.agent {
            agent.accessibility = false;
            agent.automatic = false;
            agent.launch_at_login = UiLaunchAtLogin::Off;
        }
    }),
    ("general-update-ready", |ui| {
        reset_fixture(ui, Pane::General);
        if let Some(agent) = &mut ui.agent {
            agent.update = UiUpdateStatus::Ready {
                version: "0.3.3".into(),
            };
        }
    }),
    ("workspaces", |ui| reset_fixture(ui, Pane::Workspaces)),
    ("shortcuts", |ui| reset_fixture(ui, Pane::Shortcuts)),
];
