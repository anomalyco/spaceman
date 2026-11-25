//! A Labs feature's shortcut on its card, recorded with Settings › Shortcuts' own recorder: the
//! same keycap tray, hotkeys held off while it listens, and a chord another shortcut uses
//! refused by name. It writes only the shortcut's key in `[labs]`, never the feature's switch.
//! Backspace while recording puts the default back.

use rune::El;
use spaceman::labs::LabFeature;

use crate::{
    Ui,
    controls::{self, Paint},
    panes::shortcuts::recorder_button,
};

pub(super) fn row(ui: &Ui, p: Paint, feature: LabFeature) -> El {
    // At rest the recorder shows what the file says, so an edit made elsewhere shows too.
    let chord = ui.doc.lab_shortcut(feature).0;
    let recorder = match feature.shortcut_key() {
        Some(id) => recorder_button(ui, p, id, &[chord]),
        None => controls::chords(p, &[]),
    };
    controls::setting_row(
        p,
        "shortcut",
        "Shortcut",
        Some((shortcut_description(feature).into(), p.muted())),
        recorder,
    )
}

const fn shortcut_description(feature: LabFeature) -> &'static str {
    match feature {
        LabFeature::ScrollingStrip => "Turns the strip on or off for the workspace in front of you",
        LabFeature::Scratchpad | LabFeature::Activity => "",
    }
}
