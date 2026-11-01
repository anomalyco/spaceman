//! The panes besides Rules: General (permission, login item, config file), Workspaces (which
//! display each one opens on) and Shortcuts (the hotkeys, read-only).

use rune::{El, Tx, col, row, text};
use rune_kit::{Keybind, SegmentedControl};
use spaceman::WorkspaceId;

use crate::{
    Ui,
    agent::LaunchAtLogin,
    controls::{self, ButtonKind, Paint},
    view::scroll_body,
};

pub fn general(ui: &Ui, p: Paint) -> El {
    let c = &p.k.c;
    let agent = ui.agent.clone().flatten();
    let mut sections = Vec::new();
    match &agent {
        None if ui.agent.is_some() => sections.push(controls::section(
            p,
            "agent",
            "Spaceman",
            vec![controls::setting_row(
                p,
                "not-running",
                "Spaceman isn't running",
                Some((
                    "Rules still save; it reads them when it starts.".into(),
                    p.muted(),
                )),
                controls::button(p, "open", "Open Spaceman", ButtonKind::Primary, |_, _| {
                    drop(
                        std::process::Command::new("open")
                            .args(["-b", "com.kitlangton.spaceman"])
                            .spawn(),
                    );
                }),
            )],
        )),
        None => {}
        Some(agent) => {
            let access = if agent.accessibility {
                controls::setting_row(
                    p,
                    "accessibility",
                    "Accessibility",
                    Some(("Granted. Spaceman can move windows.".into(), p.muted())),
                    controls::status_dot(c.success.fg),
                )
            } else {
                controls::setting_row(
                    p,
                    "accessibility",
                    "Accessibility",
                    Some((
                        "Spaceman needs it to move and resize windows.".into(),
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
                LaunchAtLogin::On => (true, None),
                LaunchAtLogin::Off => (false, None),
                LaunchAtLogin::NeedsApproval => (
                    true,
                    Some((
                        "Waiting for approval in System Settings › Login Items.".to_owned(),
                        c.warning.fg,
                    )),
                ),
                LaunchAtLogin::Failed(error) => (false, Some((error.clone(), c.danger.fg))),
            };
            let login =
                controls::toggle_row(p, "login", "Launch at Login", description, on, |ui| {
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
                "Spaceman",
                vec![access, tiling, hud, login, setup_tour],
            ));
        }
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
                        drop(
                            std::process::Command::new("open")
                                .arg("-R")
                                .arg(path)
                                .spawn(),
                        );
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

pub fn workspaces(ui: &Ui, p: Paint) -> El {
    let mut workspaces: Vec<WorkspaceId> = WorkspaceId::numbered().collect();
    workspaces.extend(
        WorkspaceId::all()
            .skip(WorkspaceId::NUMBERED_COUNT)
            .filter(|workspace| {
                ui.doc.monitor(*workspace).is_some()
                    || ui
                        .doc
                        .rules()
                        .iter()
                        .any(|rule| rule.place == crate::rules::Place::Workspace(*workspace))
            }),
    );
    let rows = workspaces
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

pub fn shortcuts(p: Paint) -> El {
    let rows = SHORTCUTS
        .iter()
        .enumerate()
        .map(|(index, (title, combos))| {
            let mut keys = row().key("keys").gap(6.0).center_y();
            for (n, combo) in combos.iter().enumerate() {
                if n > 0 {
                    keys = keys.child(text("or").font(p.k.ty.small).color(p.faint()));
                }
                keys = keys.child(Keybind::new(combo).build(p.k));
            }
            controls::setting_row(p, &format!("shortcut-{index}"), title, None, keys)
        })
        .collect();
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
