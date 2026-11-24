//! A Labs feature's "How it works" dialog: a small picture of the feature and its keys.

use rune::{Color, El, col, el, row, text};
use rune_kit::{Dialog, Keybind};
use spaceman::labs::LabFeature;

use crate::{
    Ui,
    controls::{DESCRIPTION, Paint},
};

const COLUMN_HEIGHT: f32 = 72.0;
const GAP: f32 = 6.0;

pub(super) fn dialog(ui: &Ui, p: Paint, feature: LabFeature, layer: &mut Vec<El>) {
    match feature {
        LabFeature::ScrollingStrip => strip(ui, p, layer),
    }
}

fn strip(ui: &Ui, p: Paint, layer: &mut Vec<El>) {
    let toggle = ui.doc.lab_shortcut(LabFeature::ScrollingStrip).0.keycaps();
    let keys: [(&str, Vec<Vec<String>>); 4] = [
        ("Turn the strip on or off for this workspace", vec![toggle]),
        (
            "Glide to the column on the left or right",
            vec![caps(&["⌥", "H"]), caps(&["⌥", "L"])],
        ),
        (
            "Move the focused column left or right",
            vec![caps(&["⌥", "⇧", "H"]), caps(&["⌥", "⇧", "L"])],
        ),
        (
            "Make the column narrower or wider",
            vec![caps(&["⌥", "-"]), caps(&["⌥", "="])],
        ),
    ];
    let rows = keys
        .into_iter()
        .enumerate()
        .map(|(index, (title, combos))| {
            let mut chords = row().key("keys").gap(6.0).center_y();
            for combo in &combos {
                chords = chords.child(Keybind::new(combo).build(p.k));
            }
            row()
                .key(format!("key-{index}"))
                .h(28.0)
                .gap(16.0)
                .center_y()
                .justify_between()
                .child(
                    text(title)
                        .key("title")
                        .font(p.k.ty.ui)
                        .color(p.text())
                        .grow(1.0)
                        .shrink(),
                )
                .child(chords)
        });
    let body = col()
        .key("how-it-works")
        .px(16.0)
        .pb(16.0)
        .gap(16.0)
        .child(ribbon(p))
        .child(col().key("keys").gap(2.0).children(rows))
        .child(
            text(
                "Columns out of view keep a sliver showing past a free screen edge. Past an \
                 edge shared with another display they wait at the edge, then hide like an \
                 inactive workspace's windows. Pausing or quitting puts every window back.",
            )
            .key("edges")
            .font(DESCRIPTION)
            .color(p.muted())
            .wrap(),
        );
    Dialog::new("scrolling-strip", "How the Scrolling Strip works")
        .description(
            "Windows sit side by side on a strip wider than the display, each one a column. \
             The display shows part of the strip and glides along it as focus moves.",
        )
        .open(true)
        .width(520.0)
        .fit()
        .child(body)
        .on_close(|ui: &mut Ui| ui.labs.explained = None)
        .build(p.k, layer);
}

fn caps(keys: &[&str]) -> Vec<String> {
    keys.iter().map(|key| (*key).to_owned()).collect()
}

/// The strip under the display: a sliver past the left edge, two columns in view (the focused
/// one marked), and the columns waiting to the right fading out.
fn ribbon(p: Paint) -> El {
    let c = &p.k.c;
    let column = |key: &str, label: &str, fill: Color, ink: Color, width: f32| {
        col()
            .key(key)
            .w(width)
            .h(COLUMN_HEIGHT)
            .radius(4.0)
            .fill(fill)
            .border(p.hair, c.border_weak)
            .center()
            .child(text(label).font(p.k.ty.small_medium).color(ink))
    };
    let display = col()
        .key("display")
        .gap(6.0)
        .items_center()
        .child(
            row()
                .key("screen")
                .padding(GAP)
                .gap(GAP)
                .radius(8.0)
                .border(1.5, c.border_focus)
                .child(column(
                    "focused",
                    "Focused",
                    c.bg_accent,
                    c.text_contrast,
                    132.0,
                ))
                .child(column("next", "In view", c.layer_03, p.text(), 132.0)),
        )
        .child(
            text("Your display")
                .key("caption")
                .font(p.k.ty.small_medium)
                .color(p.muted()),
        );
    let waiting = |key: &str, opacity: f32| {
        column(key, "", c.layer_02, p.faint(), 132.0)
            .opacity(opacity)
            .mt(-22.0)
    };
    row()
        .key("ribbon")
        .w_full()
        .gap(GAP)
        .items_start()
        // Only the strip's two ends fade; the display's caption underneath stays crisp.
        .clip_fade(24.0, 0.0, 24.0, 0.0)
        .child(
            el().key("sliver")
                .w(10.0)
                .h(COLUMN_HEIGHT)
                .mt(GAP + 1.5)
                .radius(3.0)
                .fill(c.layer_03)
                .opacity(0.7),
        )
        .child(display)
        .child(waiting("later", 0.55).mt(GAP + 1.5))
        .child(waiting("last", 0.25).mt(GAP + 1.5))
}
