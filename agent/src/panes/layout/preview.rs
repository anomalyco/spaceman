//! The miniature displays at the top of Settings › Layout, drawn with the engine's own spacing
//! math so they match what the windows will do.

use rune::{El, Spring, col, el, row, text};
use spaceman::{Rect, spacing::Spacing};

use crate::controls::{self, Paint};

/// A 16:10 display's visible frame, sized so the gaps read at preview scale.
const DISPLAY: Rect = Rect {
    x: 0,
    y: 0,
    width: 1280,
    height: 800,
};
/// A 21:9 ultrawide, where the single-window width shows.
const ULTRAWIDE: Rect = Rect {
    x: 0,
    y: 0,
    width: 3440,
    height: 1440,
};
const DISPLAY_WIDTH: f32 = 300.0;
const ULTRAWIDE_WIDTH: f32 = 196.0;
const MENU_BAR: f32 = 5.0;
const SETTLE: Spring = Spring::new(0.22, 0.0);

/// Three tiles as the engine places them: one column on the left, two stacked on the right.
pub(super) fn three_tiles(spacing: Spacing, display: Rect) -> Vec<Rect> {
    let usable = spacing.usable(display);
    let canvas = spacing.canvas(usable);
    let half = canvas.width / 2;
    let cells = vec![
        Rect {
            width: half,
            ..canvas
        },
        Rect {
            x: canvas.x + half,
            width: canvas.width - half,
            height: canvas.height / 2,
            ..canvas
        },
        Rect {
            x: canvas.x + half,
            y: canvas.y + canvas.height / 2,
            width: canvas.width - half,
            height: canvas.height - canvas.height / 2,
        },
    ];
    spacing
        .place(usable, cells.into_iter().map(|cell| ((), cell)).collect())
        .into_iter()
        .map(|(_, frame)| frame)
        .collect()
}

/// The preview card: the three-window display, joined by an ultrawide holding one capped window
/// while a single-window width is set.
pub(super) fn card(p: Paint, spacing: Spacing) -> El {
    let mut displays = row()
        .key("displays")
        .gap(28.0)
        .justify_center()
        .items_end()
        .child(display(
            p,
            "split",
            "Three windows",
            DISPLAY,
            DISPLAY_WIDTH,
            &three_tiles(spacing, DISPLAY),
        ));
    if spacing.single_window_max_width > 0 {
        displays = displays.child(display(
            p,
            "single",
            "One window, ultrawide",
            ULTRAWIDE,
            ULTRAWIDE_WIDTH,
            &[spacing.single(spacing.usable(ULTRAWIDE))],
        ));
    }
    controls::surface(p, col().key("preview").py(20.0).px(16.0).items_center())
        .label("Layout preview")
        .child(displays)
}

fn display(p: Paint, key: &str, caption: &str, screen: Rect, width: f32, windows: &[Rect]) -> El {
    let c = &p.k.c;
    let scale = width / screen.width as f32;
    let height = screen.height as f32 * scale;
    let place = |rect: Rect| {
        (
            rect.x as f32 * scale,
            MENU_BAR + rect.y as f32 * scale,
            rect.width as f32 * scale,
            rect.height as f32 * scale,
        )
    };
    let mut glass = el()
        .key("screen")
        .size(width, MENU_BAR + height)
        .radius(6.0)
        .fill(c.bg_deep)
        .border(p.hair, c.border_base)
        .clip_soft(0.0)
        // A tinted wallpaper, so the gaps and the padded edges read as space.
        .child(
            el().key("wallpaper")
                .absolute(0.0, MENU_BAR)
                .size(width, height)
                .fill(c.text_accent.alpha(0.22)),
        )
        .child(
            el().key("menu-bar")
                .absolute(0.0, 0.0)
                .size(width, MENU_BAR)
                .fill(c.layer_03),
        );
    for (index, window) in windows.iter().enumerate() {
        let (x, y, w, h) = place(*window);
        glass = glass.child(
            el().key(format!("window-{index}"))
                .absolute(x, y)
                .size(w, h)
                .radius(3.0)
                .fill(c.layer_02)
                .border(p.hair, c.border_strong)
                .clip_soft(0.0)
                .motion(SETTLE)
                .child(el().key("title-bar").size(w, 4.0).fill(c.layer_03)),
        );
    }
    col().key(key).gap(8.0).items_center().child(glass).child(
        text(caption)
            .key("caption")
            .font(p.k.ty.small)
            .color(p.faint()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_tiles_meet_the_padded_edges_and_sit_one_gap_apart() {
        let spacing = Spacing {
            inner: 10,
            top: 30,
            ..Spacing::NONE
        };
        let [left, top, bottom] = three_tiles(spacing, DISPLAY)[..] else {
            panic!("three tiles")
        };
        assert_eq!((left.x, left.y), (0, 30));
        assert_eq!(top.x - (left.x + left.width), 10);
        assert_eq!(bottom.y - (top.y + top.height), 10);
        assert_eq!(bottom.y + bottom.height, DISPLAY.height);
        assert_eq!(top.x + top.width, DISPLAY.width);
    }
}
