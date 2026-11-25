//! The Scrolling Strip's two settings in its Customize sheet: how columns move, and how they
//! meet another display's edge. Each pick writes `[labs]` in `config.toml`, and the worker
//! reloads it before the next key press.

use rune::{El, Tx};
use rune_kit::SegmentedControl;
use spaceman::labs::{GlideMotion, SharedEdge};

use crate::{
    Ui,
    controls::{self, Paint},
};

pub(super) fn rows(ui: &Ui, p: Paint) -> Vec<El> {
    vec![motion_row(ui, p), shared_edge_row(ui, p)]
}

fn picker<T: Copy + 'static>(
    p: Paint,
    key: &str,
    label: &str,
    (labels, values, selected): (Vec<&'static str>, Vec<T>, usize),
    pick: fn(&mut Ui, T),
) -> El {
    SegmentedControl::new(&labels, Some(selected))
        .label(label)
        .fit()
        .on_pick(move |ui: &mut Ui, index, _: &mut Tx| {
            if let Some(value) = values.get(index) {
                pick(ui, *value);
            }
        })
        .build(p.k)
        .key(key)
}

/// Off, Fast and Smooth, plus Custom only while the file tunes the glide by hand: the picker
/// says what runs. Picking Custom changes nothing.
fn motion_options(
    current: Option<GlideMotion>,
) -> (Vec<&'static str>, Vec<Option<GlideMotion>>, usize) {
    let mut options: Vec<_> = GlideMotion::ALL
        .into_iter()
        .map(|motion| (motion.name(), Some(motion)))
        .collect();
    if current.is_none() {
        options.push(("Custom", None));
    }
    let selected = options
        .iter()
        .position(|(_, motion)| *motion == current)
        .unwrap_or(0);
    let (labels, values) = options.into_iter().unzip();
    (labels, values, selected)
}

fn motion_row(ui: &Ui, p: Paint) -> El {
    let control = picker(
        p,
        "motion-picker",
        "Motion",
        motion_options(ui.doc.strip_motion()),
        |ui, motion| {
            if let Some(motion) = motion {
                ui.edit(|doc| doc.set_strip_motion(motion));
            }
        },
    );
    controls::setting_row(p, "motion", "Motion", None, control)
}

const fn shared_edge_name(edge: SharedEdge) -> &'static str {
    match edge {
        SharedEdge::Tuck => "Tuck",
        SharedEdge::Slide => "Slide",
    }
}

fn shared_edge_options(current: SharedEdge) -> (Vec<&'static str>, Vec<SharedEdge>, usize) {
    let selected = SharedEdge::ALL
        .into_iter()
        .position(|edge| edge == current)
        .unwrap_or(0);
    (
        SharedEdge::ALL.into_iter().map(shared_edge_name).collect(),
        SharedEdge::ALL.to_vec(),
        selected,
    )
}

/// What the chosen option does: the row's description says what is true now.
const fn shared_edge_effect(edge: SharedEdge) -> &'static str {
    match edge {
        SharedEdge::Tuck => "Columns stay on this display",
        SharedEdge::Slide => "Columns glide halfway past the shared edge",
    }
}

fn shared_edge_row(ui: &Ui, p: Paint) -> El {
    let edge = ui.doc.strip_shared_edge();
    let control = picker(
        p,
        "shared-edge-picker",
        "Next to another display",
        shared_edge_options(edge),
        |ui, edge| ui.edit(|doc| doc.set_strip_shared_edge(edge)),
    );
    controls::setting_row(
        p,
        "shared-edge",
        "Next to another display",
        Some((shared_edge_effect(edge).into(), p.muted())),
        control,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Pane;

    #[test]
    fn the_sheet_has_two_settings_and_motion_defaults_to_fast() {
        let p = Paint::new(true, 2.0);
        let mut ui = crate::playground::blank_fixture();
        crate::playground::reset_fixture(&mut ui, Pane::Labs);
        assert_eq!(rows(&ui, p).len(), 2);

        let (labels, values, selected) = motion_options(ui.doc.strip_motion());
        assert_eq!(labels, ["Off", "Fast", "Smooth"]);
        assert_eq!(values[selected], Some(GlideMotion::Fast));

        let (labels, _, selected) = shared_edge_options(SharedEdge::Slide);
        assert_eq!(labels, ["Tuck", "Slide"]);
        assert_eq!(selected, 1);
    }

    #[test]
    fn a_hand_tuned_glide_shows_as_custom_until_a_pick_replaces_it() {
        let (labels, values, selected) = motion_options(None);
        assert_eq!(labels, ["Off", "Fast", "Smooth", "Custom"]);
        assert_eq!(values[selected], None);
        let (labels, _, selected) = motion_options(Some(GlideMotion::Off));
        assert_eq!((labels.len(), labels[selected]), (3, "Off"));
    }
}
