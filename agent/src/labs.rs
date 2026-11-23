//! Settings › Labs: one card per experiment in [`LabFeature::ALL`], each with its own switch,
//! all off by default. A switch writes `[labs]` in `config.toml`; the agent picks it up from
//! there, registering the feature's shortcut and letting its work start.

use rune::{El, Icon, col, row, text};
use spaceman::labs::LabFeature;

use crate::{
    Pane, Ui,
    controls::{self, DESCRIPTION, Paint},
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
];

pub(crate) fn pane(ui: &Ui, p: Paint) -> El {
    let cards = LabFeature::ALL.iter().map(|feature| card(ui, p, *feature));
    scroll_body(
        "labs-body",
        col()
            .key("sections")
            .gap(8.0)
            .child(controls::heading(
                p,
                "header",
                "Experimental features, off by default.",
                None,
            ))
            .children(cards),
    )
}

fn card(ui: &Ui, p: Paint, feature: LabFeature) -> El {
    let switch = controls::toggle_row(
        p,
        feature.id(),
        feature.name(),
        Some((feature.summary().into(), p.muted())),
        ui.doc.lab(feature),
        move |ui| {
            let next = !ui.doc.lab(feature);
            ui.set_lab(feature, next);
        },
    );
    let note = row().key("note").px(12.0).py(8.0).child(
        text(note(feature))
            .key("copy")
            .font(DESCRIPTION)
            .color(p.faint())
            .wrap(),
    );
    controls::card(p, &format!("{}-card", feature.id()), vec![switch, note])
}

const fn note(feature: LabFeature) -> &'static str {
    match feature {
        LabFeature::ScrollingStrip => {
            "Zero private APIs: columns off screen are parked like hidden workspaces, and pause \
             or quit brings every one back. Tell us how it feels on GitHub."
        }
    }
}
