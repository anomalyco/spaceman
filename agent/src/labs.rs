//! Settings › Labs: one card per experiment in [`LabFeature::ALL`], each with its own switch,
//! all off by default. A switch writes `[labs]` in `config.toml`; the agent picks it up from
//! there, registering the feature's shortcut and letting its work start. A card also holds the
//! feature's shortcut ([`shortcut`]), its tuning while it is on ([`tuning`]), and a "How it
//! works" dialog ([`how_it_works`]).

mod how_it_works;
mod shortcut;
mod tuning;

use rune::{El, Icon, col, row, text};
use spaceman::labs::LabFeature;

use crate::{
    Pane, Ui,
    controls::{self, ButtonKind, DESCRIPTION, Paint},
    playground::{Scene, reset_fixture},
    view::scroll_body,
};

pub(crate) static LABS_ICON: Icon = Icon {
    name: "spaceman-labs",
    view: 16.0,
    body: include_str!("../assets/labs.svg"),
};

pub(crate) static INFO_ICON: Icon = Icon {
    name: "spaceman-info",
    view: 16.0,
    body: include_str!("../assets/info.svg"),
};

/// What Settings › Labs keeps between frames: an open dialog. Its shortcut records through
/// `Ui::shortcut_capture`, as Settings › Shortcuts' do.
#[derive(Default)]
pub(crate) struct LabsView {
    pub(crate) explained: Option<LabFeature>,
}

pub(crate) const SCENES: &[Scene] = &[
    ("labs", |ui| reset_fixture(ui, Pane::Labs)),
    ("labs-on", |ui| {
        reset_fixture(ui, Pane::Labs);
        ui.doc.set_lab(LabFeature::ScrollingStrip, true);
    }),
    ("labs-how-it-works", |ui| {
        reset_fixture(ui, Pane::Labs);
        ui.doc.set_lab(LabFeature::ScrollingStrip, true);
        ui.labs.explained = Some(LabFeature::ScrollingStrip);
    }),
];

pub(crate) fn pane(ui: &Ui, p: Paint, layer: &mut Vec<El>) -> El {
    let cards = LabFeature::ALL.iter().map(|feature| card(ui, p, *feature));
    if let Some(feature) = ui.labs.explained {
        how_it_works::dialog(ui, p, feature, layer);
    }
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
        // Flips what the file says now, not a copy read before another edit landed.
        move |ui| ui.edit(|doc| doc.set_lab(feature, !doc.lab(feature))),
    );
    let explain = controls::icon_row(
        p,
        "how-it-works",
        &INFO_ICON,
        "How it works",
        None,
        controls::button(p, "show", "Show", ButtonKind::Secondary, move |ui, _| {
            ui.labs.explained = Some(feature);
        }),
    );
    let note = row().key("note").px(12.0).py(8.0).child(
        text(note(feature))
            .key("copy")
            .font(DESCRIPTION)
            .color(p.faint())
            .wrap()
            .grow(1.0)
            .shrink(),
    );
    let mut rows = vec![switch, shortcut::row(ui, p, feature)];
    if ui.doc.lab(feature) {
        rows.extend(match feature {
            LabFeature::ScrollingStrip => tuning::rows(ui, p),
        });
    }
    rows.extend([explain, note]);
    controls::card(p, &format!("{}-card", feature.id()), rows)
}

const fn note(feature: LabFeature) -> &'static str {
    match feature {
        LabFeature::ScrollingStrip => {
            "Zero private APIs: columns glide at your display's refresh rate, columns out of \
             view are recorded like hidden workspaces, and pause or quit brings every one back. \
             Tell us how it feels on GitHub."
        }
    }
}
