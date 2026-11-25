//! Labs pane: experimental features, each behind its own `[labs]` switch and off by default,
//! grouped by the part of Spaceman they change. With every switch off Spaceman behaves as if
//! Labs did not exist.

use rune::El;
use spaceman::config::LabFeature;

use crate::{Ui, controls, controls::Paint, view::scroll_body};

/// A Lab's row: its switch, its name, and what it does.
type Lab = (LabFeature, &'static str, &'static str);

/// The pane's groups, in order: a key, a title, and their Labs.
const GROUPS: [(&str, &str, &[Lab]); 3] = [
    (
        "windows",
        "Windows",
        &[
            (
                LabFeature::Scratchpad,
                "Scratchpad",
                "Hide a window with ⌥⇧\\ and bring it back with ⌥\\",
            ),
            (
                LabFeature::PortraitEdgeRows,
                "Portrait edge rows",
                "Drop a window on a portrait display's top or bottom edge for a row",
            ),
        ],
    ),
    (
        "menu-bar",
        "Menu bar & HUD",
        &[(
            LabFeature::PhysicalDisplays,
            "Physical display order",
            "Workspaces left to right as your displays sit, portrait ones drawn tall",
        )],
    ),
    (
        "settings",
        "Settings",
        &[(
            LabFeature::Activity,
            "Activity",
            "Adds the Activity pane: arrangement log and diagnostics",
        )],
    ),
];

/// The `spaceman://labs/<slug>` that highlights `feature`'s row, such as `physical-displays`.
pub fn slug(feature: LabFeature) -> String {
    feature.key().replace('_', "-")
}

pub fn pane(ui: &Ui, p: Paint) -> El {
    let focus = ui.highlight_lab.as_deref();
    let sections = GROUPS
        .iter()
        .map(|(key, title, labs)| {
            let rows = labs
                .iter()
                .map(|&(feature, name, description)| {
                    let slug = slug(feature);
                    let row = controls::toggle_row(
                        p,
                        &slug,
                        name,
                        Some((description.to_owned(), p.muted())),
                        ui.doc.lab(feature),
                        move |ui| {
                            let on = !ui.doc.lab(feature);
                            ui.edit(|doc| doc.set_lab(feature, on));
                        },
                    );
                    controls::highlight(p, row, focus == Some(slug.as_str()))
                })
                .collect();
            controls::section(p, key, title, rows)
        })
        .collect();
    scroll_body("labs-body", controls::page("sections", sections))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_lab_has_one_row() {
        for feature in LabFeature::ALL {
            let rows = GROUPS
                .iter()
                .flat_map(|(_, _, labs)| labs.iter())
                .filter(|(listed, _, _)| *listed == feature)
                .count();
            assert_eq!(rows, 1, "{}", feature.key());
        }
        assert_eq!(slug(LabFeature::PhysicalDisplays), "physical-displays");
    }
}
