//! Settings › Labs: every experiment in [`LabFeature::ALL`] with its own switch, all off by
//! default. A switch writes `[labs]` in `config.toml`; the agent picks it up from there,
//! registering the feature's shortcut and letting its work start.
//!
//! The Scrolling Strip has a card of its own: its switch, and while it is on its shortcut
//! ([`shortcut`]) and a quiet `Customize…` that opens its sheet ([`customize`]). The
//! experiments that are only a switch share the card under it.

mod customize;
mod ribbon;
mod shortcut;
mod tuning;

use rune::{El, Icon, row, text};
use rune_kit::Switch;
use spaceman::labs::{LabFeature, LabShortcut};

use crate::{
    Pane, Sheet, Ui,
    controls::{self, METRICS, Paint},
    panes::shortcuts,
    playground::{Scene, reset_fixture},
    view::scroll_body,
};

pub(crate) static LABS_ICON: Icon = Icon {
    name: "spaceman-labs",
    view: 16.0,
    body: include_str!("../assets/labs.svg"),
};

pub(crate) const SCENES: &[Scene] = &[
    ("labs", |ui| reset_fixture(ui, Pane::Labs)),
    ("labs-on", |ui| {
        reset_fixture(ui, Pane::Labs);
        ui.doc.set_lab(LabFeature::ScrollingStrip, true);
    }),
    // Every experiment on: Activity joins the sidebar.
    ("labs-all-on", |ui| {
        reset_fixture(ui, Pane::Labs);
        for feature in LabFeature::ALL {
            ui.doc.set_lab(feature, true);
        }
    }),
    ("labs-customize", |ui| {
        reset_fixture(ui, Pane::Labs);
        ui.doc.set_lab(LabFeature::ScrollingStrip, true);
        ui.sheet = Some(Sheet::Lab(LabFeature::ScrollingStrip));
    }),
    // The Control + Option base and a picked chord: every key in the sheet follows the file.
    ("labs-custom-keys", |ui| {
        reset_fixture(ui, Pane::Labs);
        shortcuts::fixture_custom(ui);
        ui.shortcut_capture = shortcuts::Capture::default();
        ui.doc.set_lab(LabFeature::ScrollingStrip, true);
        if let Ok(chord) = spaceman::shortcuts::parse_chord("alt+cmd+f") {
            ui.doc
                .set_lab_shortcut(LabFeature::ScrollingStrip, LabShortcut(chord));
        }
        ui.sheet = Some(Sheet::Lab(LabFeature::ScrollingStrip));
    }),
];

pub(crate) fn pane(ui: &Ui, p: Paint) -> El {
    let intro = text("Experiments can change or go away")
        .key("intro")
        .mx(METRICS.row_px)
        .font(p.description())
        .color(p.muted());
    let mut strip = vec![strip_row(ui, p)];
    if ui.doc.lab(LabFeature::ScrollingStrip) {
        strip.push(shortcut::row(ui, p, LabFeature::ScrollingStrip));
    }
    let switches = LabFeature::ALL
        .into_iter()
        .filter(|feature| *feature != LabFeature::ScrollingStrip)
        .map(|feature| switch_row(ui, p, feature))
        .collect();
    scroll_body(
        "labs-body",
        controls::page(
            "sections",
            vec![
                intro,
                controls::card(p, "scrolling-strip-card", strip),
                controls::card(p, "switches-card", switches),
            ],
        ),
    )
}

/// Mounts a Labs feature's Customize sheet.
pub(crate) fn sheet(ui: &Ui, feature: LabFeature, p: Paint, layer: &mut Vec<El>) {
    match feature {
        LabFeature::ScrollingStrip => customize::strip_sheet(ui, p, layer),
        LabFeature::Scratchpad | LabFeature::Activity => {}
    }
}

/// The name a `spaceman://labs/<slug>` deep link uses for `feature`.
fn slug(feature: LabFeature) -> String {
    feature.key().replace('_', "-")
}

/// The fill a deep link gives the row of the experiment it names.
fn highlighted(ui: &Ui, p: Paint, feature: LabFeature, row: El) -> El {
    controls::highlight(
        p,
        row,
        ui.highlight_lab.as_deref() == Some(slug(feature).as_str()),
    )
}

/// An experiment that is only a switch: the whole row flips it.
fn switch_row(ui: &Ui, p: Paint, feature: LabFeature) -> El {
    let row = controls::toggle_row(
        p,
        feature.key(),
        feature.name(),
        Some((feature.summary().into(), p.muted())),
        ui.doc.lab(feature),
        move |ui| flip(ui, feature),
    );
    highlighted(ui, p, feature, row)
}

/// The Scrolling Strip in one row: its switch on the control rail, and a quiet `Customize…`
/// to its left while the strip is on. The switch is anchored to the rail, so nothing moves
/// when the button comes and goes. The row holds two controls, so only the switch flips it.
fn strip_row(ui: &Ui, p: Paint) -> El {
    let feature = LabFeature::ScrollingStrip;
    let on = ui.doc.lab(feature);
    let customize = on.then(|| {
        controls::quiet_button(
            p,
            "strip-customize",
            "Customize…",
            move |ui: &mut Ui, _| {
                ui.open_lab_sheet(feature);
            },
        )
    });
    let switch = Switch::new(on)
        .label(feature.name())
        .hide_label()
        .on_toggle(move |ui: &mut Ui| flip(ui, feature))
        .build(p.k)
        .key("switch");
    let row = controls::setting_row(
        p,
        feature.key(),
        feature.name(),
        Some((feature.summary().into(), p.muted())),
        row()
            .key("strip-controls")
            .gap(8.0)
            .center_y()
            .children(customize)
            .child(switch),
    );
    highlighted(ui, p, feature, row)
}

/// Flips what the file says now, not a copy read before another edit landed.
fn flip(ui: &mut Ui, feature: LabFeature) {
    ui.edit(|doc| doc.set_lab(feature, !doc.lab(feature)));
    ui.leave_hidden_lab_pane();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn customize_opens_the_strip_sheet_and_only_while_the_strip_is_on() {
        let p = Paint::new(true, 2.0);
        let mut ui = crate::playground::blank_fixture();
        reset_fixture(&mut ui, Pane::Labs);
        let strip = LabFeature::ScrollingStrip;

        ui.open_lab_sheet(strip);
        assert!(ui.sheet.is_none(), "nothing to customize while it is off");

        flip(&mut ui, strip);
        ui.open_lab_sheet(strip);
        assert!(matches!(
            ui.sheet,
            Some(Sheet::Lab(LabFeature::ScrollingStrip))
        ));
        let mut layer = Vec::new();
        crate::panes::sheet(&ui, p, false, &mut layer);
        assert_eq!(layer.len(), 2, "a scrim and the sheet");

        // Tuning saves at once with the sheet still open, and the HUD being off doesn't close it.
        ui.set_hud(None);
        ui.edit(|doc| doc.set_strip_shared_edge(spaceman::labs::SharedEdge::Slide));
        assert!(ui.sheet.is_some());

        // Switching the strip off, here or in config.toml, leaves nothing to customize.
        flip(&mut ui, strip);
        assert!(ui.sheet.is_none());
        let _ = pane(&ui, p);
    }

    #[test]
    fn every_experiment_has_a_row_and_a_deep_link_slug() {
        let p = Paint::new(true, 2.0);
        let mut ui = crate::playground::blank_fixture();
        reset_fixture(&mut ui, Pane::Labs);
        assert_eq!(slug(LabFeature::ScrollingStrip), "scrolling-strip");
        for feature in LabFeature::ALL {
            assert!(!ui.doc.lab(feature));
            flip(&mut ui, feature);
            assert!(ui.doc.lab(feature), "{}", feature.key());
            assert_eq!(ui.pane, Pane::Labs);
        }
        let _ = pane(&ui, p);
        // Turning Activity off while looking at Labs stays on Labs.
        flip(&mut ui, LabFeature::Activity);
        assert_eq!(ui.pane, Pane::Labs);
    }
}
