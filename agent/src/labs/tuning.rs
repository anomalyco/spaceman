//! The Scrolling Strip's glide tuning on its card, shown while the strip is on: how long a glide
//! takes, how springy it is, and how columns meet another display's edge. Each pick writes
//! `[labs]` in `config.toml`, and the worker reloads it before the next key press.

use rune::{El, Tx};
use rune_kit::SegmentedControl;
use spaceman::labs::{GlideDamping, GlideDuration, SharedEdge};

use crate::{
    Ui,
    controls::{self, Paint},
};

pub(super) fn rows(ui: &Ui, p: Paint) -> Vec<El> {
    vec![
        duration_row(ui, p),
        damping_row(ui, p),
        shared_edge_row(ui, p),
    ]
}

/// Labels and values for a picker: the presets, plus the file's own value when it is none of
/// them, and which one is selected.
fn options<T: Copy + PartialEq>(
    presets: impl IntoIterator<Item = (String, T)>,
    current: T,
    custom: impl FnOnce(T) -> String,
) -> (Vec<String>, Vec<T>, usize) {
    let mut options: Vec<(String, T)> = presets.into_iter().collect();
    if !options.iter().any(|(_, value)| *value == current) {
        options.push((custom(current), current));
    }
    let selected = options
        .iter()
        .position(|(_, value)| *value == current)
        .unwrap_or(0);
    let (labels, values) = options.into_iter().unzip();
    (labels, values, selected)
}

fn picker<T: Copy + PartialEq + 'static>(
    p: Paint,
    key: &str,
    (labels, values, selected): (Vec<String>, Vec<T>, usize),
    pick: fn(&mut Ui, T),
) -> El {
    SegmentedControl::new(&labels, Some(selected))
        .fit()
        .on_pick(move |ui: &mut Ui, index, _: &mut Tx| {
            if let Some(value) = values.get(index) {
                pick(ui, *value);
            }
        })
        .build(p.k)
        .key(key)
}

fn duration_options(current: GlideDuration) -> (Vec<String>, Vec<GlideDuration>, usize) {
    options(
        GlideDuration::PRESETS
            .into_iter()
            .filter_map(|ms| GlideDuration::new(ms).ok())
            .map(|duration| (format!("{} ms", duration.ms()), duration)),
        current,
        |duration| format!("{} ms", duration.ms()),
    )
}

fn damping_options(current: GlideDamping) -> (Vec<String>, Vec<GlideDamping>, usize) {
    options(
        GlideDamping::PRESETS
            .into_iter()
            .filter_map(|(name, ratio)| Some((name.to_owned(), GlideDamping::new(ratio).ok()?))),
        current,
        |damping| format!("{:.2}", damping.ratio()),
    )
}

fn shared_edge_options(current: SharedEdge) -> (Vec<String>, Vec<SharedEdge>, usize) {
    options(
        SharedEdge::ALL.into_iter().map(|edge| {
            let label = match edge {
                SharedEdge::Tuck => "Tuck",
                SharedEdge::Slide => "Slide",
            };
            (label.to_owned(), edge)
        }),
        current,
        |edge| edge.as_str().to_owned(),
    )
}

fn duration_row(ui: &Ui, p: Paint) -> El {
    let control = picker(
        p,
        "duration-picker",
        duration_options(ui.doc.strip_duration()),
        |ui, duration| ui.edit(|doc| doc.set_strip_duration(duration)),
    );
    controls::setting_row(p, "glide-duration", "Glide", None, control)
}

fn damping_row(ui: &Ui, p: Paint) -> El {
    let control = picker(
        p,
        "damping-picker",
        damping_options(ui.doc.strip_damping()),
        |ui, damping| ui.edit(|doc| doc.set_strip_damping(damping)),
    );
    controls::setting_row(p, "glide-feel", "Feel", None, control)
}

fn shared_edge_row(ui: &Ui, p: Paint) -> El {
    let control = picker(
        p,
        "shared-edge-picker",
        shared_edge_options(ui.doc.strip_shared_edge()),
        |ui, edge| ui.edit(|doc| doc.set_strip_shared_edge(edge)),
    );
    controls::setting_row(
        p,
        "shared-edge",
        "Next to another display",
        Some((
            "Tuck keeps columns on this display. Slide lets them glide halfway past the edge."
                .into(),
            p.muted(),
        )),
        control,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pickers_offer_the_presets_and_a_custom_value_from_the_file() {
        let (labels, _, selected) = duration_options(GlideDuration::default());
        assert_eq!(labels, ["150 ms", "250 ms", "380 ms", "520 ms"]);
        assert_eq!(selected, 1);
        let (labels, values, selected) = duration_options(GlideDuration::new(600).unwrap());
        assert_eq!(labels.len(), 5);
        assert_eq!(
            (labels[selected].as_str(), values[selected].ms()),
            ("600 ms", 600)
        );

        let (labels, _, selected) = damping_options(GlideDamping::default());
        assert_eq!(labels, ["Bouncy", "Snappy", "Soft"]);
        assert_eq!(selected, 1);
        let (labels, _, selected) = damping_options(GlideDamping::new(0.9).unwrap());
        assert_eq!(labels[selected], "0.90");

        let (labels, _, selected) = shared_edge_options(SharedEdge::Slide);
        assert_eq!(labels, ["Tuck", "Slide"]);
        assert_eq!(selected, 1);
    }
}
