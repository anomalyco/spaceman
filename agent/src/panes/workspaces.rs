//! Workspaces pane: display assignments for numbered and referenced named workspaces.

use rune::{El, Tx, col};
use rune_kit::SegmentedControl;
use spaceman::WorkspaceId;

use crate::{
    Ui,
    controls::{self, Paint},
    rules::Doc,
    view::scroll_body,
};

const MONITORS: [&str; 2] = ["Main", "Secondary"];

/// Built-in default display for `workspace` when `config.toml` sets no explicit `monitor`.
const fn default_monitor(workspace: WorkspaceId) -> &'static str {
    match workspace.number() {
        Some(n) if n >= 6 => "secondary",
        _ => "main",
    }
}

/// Which display `workspace` currently targets: explicit override from `config.toml`, or its
/// built-in default (1–5 and named workspaces on Main, 6–10 on Secondary).
fn effective_monitor(doc: &Doc, workspace: WorkspaceId) -> String {
    doc.monitor(workspace)
        .unwrap_or_else(|| default_monitor(workspace).to_owned())
}

pub fn pane(ui: &Ui, p: Paint) -> El {
    let all = ui.doc.workspaces_with_referenced(WorkspaceId::numbered());
    let mut main_rows = Vec::new();
    let mut secondary_rows = Vec::new();
    let mut custom_rows = Vec::new();
    for workspace in all {
        let target = effective_monitor(&ui.doc, workspace);
        let row = monitor_row(p, workspace, &target);
        match target.as_str() {
            "main" => main_rows.push(row),
            "secondary" => secondary_rows.push(row),
            _ => custom_rows.push(row),
        }
    }
    let mut sections = Vec::new();
    if !main_rows.is_empty() {
        sections.push(
            col()
                .key("main-display")
                .gap(8.0)
                .child(controls::heading(
                    p,
                    "main-header",
                    "Main display",
                    Some("Default for 1–5"),
                ))
                .child(controls::card(p, "main-card", main_rows)),
        );
    }
    if !secondary_rows.is_empty() {
        sections.push(
            col()
                .key("secondary-display")
                .gap(8.0)
                .child(controls::heading(
                    p,
                    "secondary-header",
                    "Secondary display",
                    Some("Default for 6–10 · main when unplugged"),
                ))
                .child(controls::card(p, "secondary-card", secondary_rows)),
        );
    }
    if !custom_rows.is_empty() {
        sections.push(
            col()
                .key("custom-display")
                .gap(8.0)
                .child(controls::heading(
                    p,
                    "custom-header",
                    "Named displays",
                    Some("Configured in config.toml"),
                ))
                .child(controls::card(p, "custom-card", custom_rows)),
        );
    }
    scroll_body(
        "workspaces-body",
        col().key("sections").gap(24.0).children(sections),
    )
}

fn monitor_row(p: Paint, workspace: WorkspaceId, effective: &str) -> El {
    let mut labels: Vec<String> = MONITORS.iter().map(|label| (*label).to_owned()).collect();
    let selected = match effective {
        "main" => 0,
        "secondary" => 1,
        name => {
            labels.push(name.to_owned());
            2
        }
    };
    let picker = SegmentedControl::new(&labels, Some(selected))
        .fit()
        .on_pick(move |ui: &mut Ui, index, _: &mut Tx| {
            let chosen = match index {
                0 => "main",
                1 => "secondary",
                _ => return,
            };
            // Picking a workspace's built-in default clears any redundant override in config.toml.
            let override_value = (chosen != default_monitor(workspace)).then_some(chosen);
            ui.edit(|doc| doc.set_monitor(workspace, override_value));
        })
        .build(p.k)
        .key("monitor");
    let title = match workspace.number() {
        Some(10) => "Workspace 10 (⌥0)".to_owned(),
        Some(_) => format!("Workspace {workspace}"),
        None => format!("Workspace {workspace} (named)"),
    };
    let description = (effective != default_monitor(workspace)).then(|| {
        let default_label = if default_monitor(workspace) == "secondary" {
            "Secondary"
        } else {
            "Main"
        };
        (format!("Default: {default_label}"), p.faint())
    });
    controls::setting_row(
        p,
        &format!("workspace-{workspace}"),
        &title,
        description,
        picker,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_monitor_defaults_split_main_and_secondary() {
        let doc = Doc::parse("version = 1\n");
        for number in 1..=5 {
            let ws = WorkspaceId::new(number).unwrap();
            assert_eq!(default_monitor(ws), "main");
            assert_eq!(effective_monitor(&doc, ws), "main");
        }
        for number in 6..=10 {
            let ws = WorkspaceId::new(number).unwrap();
            assert_eq!(default_monitor(ws), "secondary");
            assert_eq!(effective_monitor(&doc, ws), "secondary");
        }
        let named = WorkspaceId::named('M').unwrap();
        assert_eq!(default_monitor(named), "main");
        assert_eq!(effective_monitor(&doc, named), "main");

        let mut overridden = doc;
        overridden.set_monitor(WorkspaceId::new(2).unwrap(), Some("secondary"));
        overridden.set_monitor(WorkspaceId::new(7).unwrap(), Some("main"));
        assert_eq!(
            effective_monitor(&overridden, WorkspaceId::new(2).unwrap()),
            "secondary"
        );
        assert_eq!(
            effective_monitor(&overridden, WorkspaceId::new(7).unwrap()),
            "main"
        );
    }
}
