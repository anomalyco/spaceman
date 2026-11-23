//! Settings pane registry plus the General, Workspaces, and Shortcuts panes.

use rune::{El, Icon, Tx, col};
use rune_kit::SegmentedControl;
use spaceman::{WorkspaceId, app::UiLaunchAtLogin, config::HudStyle};

use crate::{
    Ui, activity,
    controls::{self, ButtonKind, Paint},
    map,
    playground::{Scene, reset_fixture},
    view::scroll_body,
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
}

type PaneRender = fn(&Ui, Paint, &mut Vec<El>) -> (Option<El>, El);

struct PaneSpec {
    key: &'static str,
    title: &'static str,
    icon: &'static Icon,
    render: PaneRender,
}

const fn pane_spec(
    key: &'static str,
    title: &'static str,
    icon: &'static Icon,
    render: PaneRender,
) -> PaneSpec {
    PaneSpec {
        key,
        title,
        icon,
        render,
    }
}

impl Pane {
    pub const ALL: [Pane; 5] = [
        Pane::General,
        Pane::Rules,
        Pane::Workspaces,
        Pane::Shortcuts,
        Pane::Activity,
    ];

    const fn spec(self) -> PaneSpec {
        match self {
            Pane::General => pane_spec("general", "General", &GENERAL_ICON, |ui, p, _| {
                (None, general(ui, p))
            }),
            Pane::Rules => pane_spec("rules", "Rules", &RULES_ICON, |ui, p, layer| {
                (None, map::pane(ui, p, layer))
            }),
            Pane::Workspaces => {
                pane_spec("workspaces", "Workspaces", &WORKSPACES_ICON, |ui, p, _| {
                    (None, workspaces(ui, p))
                })
            }
            Pane::Shortcuts => pane_spec(
                "shortcuts",
                "Shortcuts",
                &rune_theme_opencode::icons::KEYBOARD,
                |_, p, _| (None, shortcuts(p)),
            ),
            Pane::Activity => pane_spec("activity", "Activity", &ACTIVITY_ICON, |ui, p, _| {
                (Some(activity::header_actions(ui, p)), activity::pane(ui, p))
            }),
        }
    }

    pub const fn key(self) -> &'static str {
        self.spec().key
    }

    pub const fn title(self) -> &'static str {
        self.spec().title
    }

    pub const fn icon(self) -> &'static Icon {
        self.spec().icon
    }

    pub fn render(self, ui: &Ui, p: Paint, layer: &mut Vec<El>) -> (Option<El>, El) {
        (self.spec().render)(ui, p, layer)
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
    ("workspaces", |ui| reset_fixture(ui, Pane::Workspaces)),
    ("shortcuts", |ui| reset_fixture(ui, Pane::Shortcuts)),
];

fn general(ui: &Ui, p: Paint) -> El {
    let c = &p.k.c;
    let mut sections = Vec::new();
    if let Some(agent) = &ui.agent {
        let access = if agent.accessibility {
            controls::setting_row(
                p,
                "accessibility",
                "Accessibility",
                Some(("Granted. Pane can move windows.".into(), p.muted())),
                controls::status_dot(c.success.fg),
            )
        } else {
            controls::setting_row(
                p,
                "accessibility",
                "Accessibility",
                Some((
                    "Pane needs it to move and resize windows.".into(),
                    c.warning.fg,
                )),
                controls::button(
                    p,
                    "grant",
                    "Open System Settings",
                    ButtonKind::Primary,
                    |ui, _| ui.open_accessibility(),
                ),
            )
        };
        let tiling = if agent.accessibility {
            controls::toggle_row(
                p,
                "tiling",
                "Automatic tiling",
                Some((
                    if agent.automatic {
                        "On. New windows tile as they open.".into()
                    } else {
                        "Paused. Parked windows are restored.".into()
                    },
                    p.muted(),
                )),
                agent.automatic,
                |ui| ui.toggle_automatic(),
            )
        } else {
            controls::setting_row(
                p,
                "tiling",
                "Automatic tiling",
                Some((
                    "Starts once Accessibility is granted and the hotkeys register.".into(),
                    p.muted(),
                )),
                controls::status_dot(c.border_strong),
            )
        };
        let (on, description) = match &agent.launch_at_login {
            UiLaunchAtLogin::On => (true, None),
            UiLaunchAtLogin::Off => (false, None),
            UiLaunchAtLogin::NeedsApproval => (
                true,
                Some((
                    "Waiting for approval in System Settings › Login Items.".to_owned(),
                    c.warning.fg,
                )),
            ),
            UiLaunchAtLogin::Failed(error) => (false, Some((error.clone(), c.danger.fg))),
        };
        let login = controls::toggle_row(p, "login", "Launch at Login", description, on, |ui| {
            ui.toggle_launch_at_login();
        });
        let hud = controls::toggle_row(
            p,
            "hud",
            "Workspace HUD",
            Some((
                "Show a non-activating capsule when switching workspaces or layouts.".into(),
                p.muted(),
            )),
            ui.doc.hud(),
            |ui| ui.toggle_hud(),
        );
        let hud_style = controls::setting_row(
            p,
            "hud-style",
            "HUD style",
            Some((
                "Each workspace as its number and app icons, or as its layout.".into(),
                p.muted(),
            )),
            SegmentedControl::new(
                &HUD_STYLES.map(|(label, _)| label.to_owned()),
                HUD_STYLES
                    .iter()
                    .position(|(_, style)| *style == ui.doc.hud_style()),
            )
            .fit()
            .on_pick(|ui: &mut Ui, index, _: &mut Tx| {
                if let Some((_, style)) = HUD_STYLES.get(index) {
                    ui.set_hud_style(*style);
                }
            })
            .build(p.k)
            .key("hud-style-picker"),
        );
        let setup_tour = controls::setting_row(
            p,
            "setup-tour",
            "Setup tour",
            Some((
                "Walk through permissions, app rules, and shortcuts.".into(),
                p.muted(),
            )),
            controls::button(
                p,
                "run-tour",
                "Run Setup Tour…",
                ButtonKind::Secondary,
                |ui, _| ui.start_tour(),
            ),
        );
        sections.push(controls::section(
            p,
            "agent",
            "Pane",
            vec![access, tiling, hud, hud_style, login, setup_tour],
        ));
    }
    sections.push(controls::section(
        p,
        "config",
        "Configuration",
        vec![controls::setting_row(
            p,
            "file",
            "config.toml",
            Some((
                "Rules and workspaces save here. Edits from an editor show up too.".into(),
                p.muted(),
            )),
            controls::button(
                p,
                "reveal",
                "Show in Finder",
                ButtonKind::Secondary,
                |_: &mut Ui, _: &mut Tx| {
                    if let Some(path) = spaceman::config::config_path() {
                        controls::reveal_in_finder(&path);
                    }
                },
            ),
        )],
    ));
    scroll_body(
        "general-body",
        col().key("sections").gap(24.0).children(sections),
    )
}

const MONITORS: [&str; 3] = ["Any", "Main", "Secondary"];
const HUD_STYLES: [(&str, HudStyle); 2] =
    [("Icons", HudStyle::Icons), ("Layouts", HudStyle::Layouts)];

fn workspaces(ui: &Ui, p: Paint) -> El {
    let rows = ui
        .doc
        .workspaces_with_referenced(WorkspaceId::numbered())
        .into_iter()
        .map(|workspace| monitor_row(ui, p, workspace))
        .collect();
    scroll_body(
        "workspaces-body",
        col()
            .key("sections")
            .gap(8.0)
            .child(controls::heading(
                p,
                "header",
                "Display",
                Some("Where each workspace shows first"),
            ))
            .child(controls::card(p, "card", rows)),
    )
}

fn monitor_row(ui: &Ui, p: Paint, workspace: WorkspaceId) -> El {
    let monitor = ui.doc.monitor(workspace);
    let mut labels: Vec<String> = MONITORS.iter().map(|label| (*label).to_owned()).collect();
    let selected = match monitor.as_deref() {
        None => 0,
        Some("main") => 1,
        Some("secondary") => 2,
        Some(name) => {
            labels.push(name.to_owned());
            3
        }
    };
    let picker = SegmentedControl::new(&labels, Some(selected))
        .fit()
        .on_pick(move |ui: &mut Ui, index, _: &mut Tx| {
            let monitor = match index {
                0 => None,
                1 => Some("main"),
                2 => Some("secondary"),
                _ => return,
            };
            ui.edit(|doc| doc.set_monitor(workspace, monitor));
        })
        .build(p.k)
        .key("monitor");
    let title = match workspace.number() {
        Some(_) => format!("Workspace {workspace}"),
        None => format!("Workspace {workspace} (named)"),
    };
    controls::setting_row(p, &format!("workspace-{workspace}"), &title, None, picker)
}

/// The README's hotkeys.
const SHORTCUTS: &[(&str, &[&[&str]])] = &[
    ("Switch to workspace 1–10", &[&["⌥", "1…0"]]),
    ("Move window to workspace 1–10", &[&["⌥", "⇧", "1…0"]]),
    ("Switch to a named workspace", &[&["⌥", "A…Z"]]),
    ("Move window to a named workspace", &[&["⌥", "⇧", "A…Z"]]),
    ("Previous workspace on this display", &[&["⌥", "Tab"]]),
    ("Next window in the workspace", &[&["⌥", "`"]]),
    ("Focus left, down, up, right", &[&["⌥", "H J K L"]]),
    ("Move tile left, down, up, right", &[&["⌥", "⇧", "H J K L"]]),
    ("Flip the split", &[&["⌥", "/"]]),
    ("Shrink or grow the tile", &[&["⌥", "-"], &["⌥", "="]]),
    ("Fullscreen layout", &[&["⌥", "⇧", "F"]]),
];

fn shortcuts(p: Paint) -> El {
    let rows = controls::shortcut_rows(p, "shortcut", SHORTCUTS);
    scroll_body(
        "shortcuts-body",
        col()
            .key("sections")
            .gap(8.0)
            .child(controls::heading(
                p,
                "header",
                "Hotkeys",
                Some("Follow your keyboard layout"),
            ))
            .child(controls::card(p, "card", rows)),
    )
}
