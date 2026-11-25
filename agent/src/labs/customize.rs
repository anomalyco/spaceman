//! The Scrolling Strip's Customize sheet: its picture, its two settings, and the keys
//! that drive it. Every key is read from the file, so a changed base modifier, a rebound
//! shortcut and the strip's own chord all show as they are. Changes apply at once; Done only
//! closes.

use rune::{El, ScrollSpec, col};
use spaceman::{labs::LabFeature, shortcuts::Chord};

use super::{ribbon, tuning};
use crate::{
    Ui,
    controls::{self, ButtonKind, METRICS, MODAL, ModalWidth, Paint},
    panes::shortcuts::{catalog, enabled_named_workspaces},
};

pub(super) fn strip_sheet(ui: &Ui, p: Paint, layer: &mut Vec<El>) {
    // In a window too short for the sheet, the body scrolls between the title and Done, and
    // dissolves into the gaps above and below it instead of clipping on a line. The body
    // reaches into both gaps by the fade and is inset by as much, so nothing moves at rest.
    // It clips at the panel's edges, not the cards', so the cards keep their shadows.
    let body = col()
        .key("body")
        .shrink()
        .min_h(0.0)
        .mx(-METRICS.card_edge())
        .my(-MODAL.gap)
        .clip_fade(0.0, MODAL.gap, 0.0, MODAL.gap)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(
            col()
                .key("blocks")
                .px(METRICS.card_edge())
                .py(MODAL.gap)
                .gap(MODAL.gap)
                .child(ribbon::picture(p))
                .child(controls::card(p, "tuning", tuning::rows(ui, p)))
                .child(controls::section(p, "keys", "Shortcuts", key_rows(ui, p))),
        );
    let panel = controls::modal_panel(
        p,
        "strip-sheet",
        LabFeature::ScrollingStrip.name(),
        ModalWidth::Medium,
        vec![body],
        vec![
            controls::button(
                p,
                "done",
                "Done",
                ButtonKind::Secondary,
                |ui: &mut Ui, _| {
                    ui.close_sheet();
                },
            )
            .default_button(),
        ],
    )
    .shrink()
    .min_h(0.0);
    controls::modal_sheet(p, "strip-sheet", panel, Ui::close_sheet, layer);
}

/// What the strip answers to, and the keys that say it, as the file has them now.
fn keys(ui: &Ui) -> [(&'static str, &'static str, Vec<Chord>); 4] {
    let shortcuts = ui.doc.shortcuts();
    let named = enabled_named_workspaces(ui);
    let pair = |left: &str, right: &str| -> Vec<Chord> {
        [left, right]
            .into_iter()
            .filter_map(|id| {
                catalog::find(id)?
                    .chords(&shortcuts, &named)
                    .into_iter()
                    .next()
            })
            .collect()
    };
    [
        (
            "toggle",
            "Turn on or off for this workspace",
            vec![ui.doc.lab_shortcut(LabFeature::ScrollingStrip).0],
        ),
        (
            "glide",
            "Glide left or right",
            pair("focus-left", "focus-right"),
        ),
        (
            "move",
            "Move the column left or right",
            pair("move-left", "move-right"),
        ),
        ("resize", "Narrower or wider", pair("shrink", "grow")),
    ]
}

fn key_rows(ui: &Ui, p: Paint) -> Vec<El> {
    keys(ui)
        .into_iter()
        .map(|(key, title, chords)| {
            controls::reference_row(p, key, title, controls::chords(p, &chords))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use spaceman::labs::LabShortcut;

    use super::*;
    use crate::{Pane, panes::shortcuts};

    fn symbols(ui: &Ui) -> Vec<Vec<String>> {
        keys(ui)
            .into_iter()
            .map(|(_, _, chords)| chords.iter().map(Chord::symbols).collect())
            .collect()
    }

    #[test]
    fn every_key_in_the_sheet_follows_the_file() {
        let mut ui = crate::playground::blank_fixture();
        crate::playground::reset_fixture(&mut ui, Pane::Labs);
        assert_eq!(
            symbols(&ui),
            [
                vec!["⌥⇧S"],
                vec!["⌥H", "⌥L"],
                vec!["⌥⇧H", "⌥⇧L"],
                vec!["⌥-", "⌥="],
            ]
        );

        // The Control + Option base moves every built-in key; the strip's own chord is its own.
        shortcuts::fixture_custom(&mut ui);
        let chord = spaceman::shortcuts::parse_chord("alt+cmd+f").unwrap();
        ui.doc
            .set_lab_shortcut(LabFeature::ScrollingStrip, LabShortcut(chord));
        let keys = symbols(&ui);
        assert_eq!(keys[0], ["⌥⌘F"]);
        for row in &keys[1..] {
            assert!(!row.is_empty());
            for chord in row {
                assert!(chord.starts_with("⌃⌥"), "{chord} keeps the base modifier");
            }
        }
    }
}
