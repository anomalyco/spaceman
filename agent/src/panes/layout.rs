//! Layout pane: space between tiled windows, padding at the screen's edges, and the width a lone
//! window keeps on a wide display, under a live miniature of the result.

mod preview;
mod slider;

use rune::{El, Tx, col, icon, row, text};
use rune_kit::SegmentedControl;
use spaceman::spacing::{Spacing, SpacingKey};

use crate::{
    Ui,
    controls::{self, Paint},
    view::scroll_body,
};

use slider::{Channel, slider};

/// The sliders' reach; `config.toml` takes more (48 between windows, 128 at an edge).
const SLIDER_MAX: i32 = 32;
/// Single-window widths offered besides Off.
const WIDTHS: [i32; 3] = [1440, 1680, 1920];

const INNER: Channel = Channel {
    get: |spacing| spacing.inner,
    set: |spacing, inner| Spacing { inner, ..spacing },
    max: SLIDER_MAX,
};

/// All four edges at once; while they differ, it shows the top edge and sets them all.
const EDGES: Channel = Channel {
    get: |spacing| spacing.uniform_edges().unwrap_or(spacing.top),
    set: Spacing::with_edges,
    max: SLIDER_MAX,
};

const EDGE_CHANNELS: [(SpacingKey, &str, Channel); 4] = [
    (SpacingKey::Top, "Top", edge(SpacingKey::Top)),
    (SpacingKey::Bottom, "Bottom", edge(SpacingKey::Bottom)),
    (SpacingKey::Left, "Left", edge(SpacingKey::Left)),
    (SpacingKey::Right, "Right", edge(SpacingKey::Right)),
];

const fn edge(key: SpacingKey) -> Channel {
    match key {
        SpacingKey::Top => Channel {
            get: |spacing| spacing.top,
            set: |spacing, top| Spacing { top, ..spacing },
            max: SLIDER_MAX,
        },
        SpacingKey::Bottom => Channel {
            get: |spacing| spacing.bottom,
            set: |spacing, bottom| Spacing { bottom, ..spacing },
            max: SLIDER_MAX,
        },
        SpacingKey::Left => Channel {
            get: |spacing| spacing.left,
            set: |spacing, left| Spacing { left, ..spacing },
            max: SLIDER_MAX,
        },
        _ => Channel {
            get: |spacing| spacing.right,
            set: |spacing, right| Spacing { right, ..spacing },
            max: SLIDER_MAX,
        },
    }
}

/// Whether the four edge sliders show: as the user left them, else while the edges differ.
pub(crate) fn per_edge_open(ui: &Ui, spacing: &Spacing) -> bool {
    ui.per_edge.unwrap_or(spacing.uniform_edges().is_none())
}

pub fn pane(ui: &Ui, p: Paint) -> El {
    let spacing = ui.spacing_draft.unwrap_or_else(|| ui.doc.spacing());
    let open = per_edge_open(ui, &spacing);
    let edges_mixed = spacing.uniform_edges().is_none();
    let mut gap_rows = vec![
        controls::setting_row(
            p,
            "inner",
            "Between windows",
            None,
            slider(p, "inner-slider", "Between windows", spacing.inner, INNER),
        ),
        controls::setting_row(
            p,
            "edges",
            "Screen edges",
            edges_mixed.then(|| ("Each edge is set on its own.".to_owned(), p.muted())),
            slider(
                p,
                "edges-slider",
                "Screen edges",
                (EDGES.get)(&spacing),
                EDGES,
            ),
        ),
        disclosure(p, open),
    ];
    if open {
        gap_rows.extend(EDGE_CHANNELS.iter().map(|(key, title, channel)| {
            let name = key.gap_name().unwrap_or("edge");
            controls::setting_row(
                p,
                &format!("edge-{name}"),
                title,
                None,
                slider(
                    p,
                    &format!("{name}-slider"),
                    &format!("{title} edge"),
                    (channel.get)(&spacing),
                    *channel,
                ),
            )
            .pl(28.0)
        }));
    }
    let width = controls::setting_row(
        p,
        "single-width",
        "Single-window width",
        Some((
            "A window alone on its display, and Fullscreen Layout, is centered at this width."
                .to_owned(),
            p.muted(),
        )),
        width_picker(p, spacing.single_window_max_width),
    );
    scroll_body(
        "layout-body",
        col()
            .key("sections")
            .gap(24.0)
            .child(preview::card(p, spacing))
            .child(controls::section(p, "gaps", "Gaps", gap_rows))
            .child(controls::section(p, "wide", "Wide displays", vec![width])),
    )
}

/// The quiet row that shows or hides the four edge sliders.
fn disclosure(p: Paint, open: bool) -> El {
    let chevron = if open {
        p.k.glyph.chevron_down
    } else {
        p.k.glyph.caret_right
    };
    row()
        .key("per-edge")
        .role("button")
        .label("Set each edge")
        .h(32.0)
        .px(12.0)
        .gap(6.0)
        .center_y()
        .radius(p.k.radius.md)
        .press(p.k.c.pressed)
        .focusable(true)
        .focus_ring(2.0, -2.0, p.k.c.border_focus)
        .on_press(move |ui: &mut Ui| ui.per_edge = Some(!open))
        .child(icon(chevron, 12.0).key("chevron").color(p.k.c.icon_muted))
        .child(
            text("Set each edge")
                .key("label")
                .font(p.k.ty.small)
                .color(p.muted()),
        )
}

/// Off, the common widths, and a width only `config.toml` set when it holds one.
fn width_options(current: i32) -> (Vec<(String, i32)>, usize) {
    let mut options = vec![("Off".to_owned(), 0)];
    options.extend(WIDTHS.map(|width| (width.to_string(), width)));
    if !options.iter().any(|(_, width)| *width == current) {
        options.push((format!("{current} px"), current));
    }
    let selected = options
        .iter()
        .position(|(_, width)| *width == current)
        .unwrap_or(0);
    (options, selected)
}

fn width_picker(p: Paint, current: i32) -> El {
    let (options, selected) = width_options(current);
    let labels: Vec<String> = options.iter().map(|(label, _)| label.clone()).collect();
    let widths: Vec<i32> = options.iter().map(|(_, width)| *width).collect();
    SegmentedControl::new(&labels, Some(selected))
        .fit()
        .on_pick(move |ui: &mut Ui, index, _: &mut Tx| {
            if let Some(width) = widths.get(index) {
                let base = ui.doc.spacing();
                ui.set_spacing(Spacing {
                    single_window_max_width: *width,
                    ..base
                });
            }
        })
        .build(p.k)
        .key("width-picker")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_picker_offers_off_the_presets_and_a_custom_width() {
        let (options, selected) = width_options(0);
        assert_eq!(options.len(), 4);
        assert_eq!(selected, 0);
        let (_, selected) = width_options(1680);
        assert_eq!(selected, 2);
        let (options, selected) = width_options(1600);
        assert_eq!(options.len(), 5);
        assert_eq!(options[selected], ("1600 px".to_owned(), 1600));
    }

    #[test]
    fn edge_channels_write_only_their_edge() {
        let base = Spacing::NONE.with_edges(4);
        for (key, _, channel) in EDGE_CHANNELS {
            let moved = (channel.set)(base, 20);
            assert_eq!(key.get(&moved), 20);
            assert_eq!((channel.get)(&moved), 20);
            for other in SpacingKey::EDGES.into_iter().filter(|other| *other != key) {
                assert_eq!(other.get(&moved), 4);
            }
        }
        assert_eq!((EDGES.set)(base, 9).uniform_edges(), Some(9));
        assert_eq!((INNER.set)(base, 9).inner, 9);
    }
}
