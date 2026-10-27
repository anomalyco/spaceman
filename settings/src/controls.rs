//! Settings controls on `rune-kit` with OpenCode's theme, the same as Hex's and Invoke's: the
//! raised card, its rows, switches and buttons.

use rune::{Color, El, TextStyle, Tx, col, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit, Switch};

use crate::Ui;

/// Row descriptions and secondary copy.
pub const DESCRIPTION: TextStyle = TextStyle::new(12.0, 440.0).lh(16.0);
const ROW_HEIGHT: f32 = 44.0;
const ROW_HEIGHT_DESCRIBED: f32 = 56.0;

/// Everything a control needs to paint itself.
#[derive(Clone, Copy)]
pub struct Paint {
    pub k: &'static Kit,
    pub hair: f32,
}

impl Paint {
    pub fn new(dark: bool, scale: f32) -> Self {
        Self {
            k: rune_theme_opencode::kit(dark),
            hair: 1.0 / scale,
        }
    }
    pub fn text(self) -> Color {
        self.k.c.text_base
    }
    pub fn muted(self) -> Color {
        self.k.c.text_muted
    }
    pub fn faint(self) -> Color {
        self.k.c.text_faint
    }
}

/// A small heading over a card, with optional copy on the right.
pub fn heading(p: Paint, key: &str, title: &str, trailing: Option<&str>) -> El {
    row()
        .key(key)
        .h(20.0)
        .px(12.0)
        .center_y()
        .justify_between()
        .child(
            text(title)
                .key("title")
                .font(p.k.ty.ui_medium.lh(16.0))
                .color(p.muted()),
        )
        .children(trailing.map(|copy| {
            text(copy)
                .key("trailing")
                .font(p.k.ty.small)
                .color(p.faint())
        }))
}

/// A titled card of rows.
pub fn section(p: Paint, key: &str, title: &str, rows: Vec<El>) -> El {
    col()
        .key(key)
        .gap(8.0)
        .child(heading(p, "header", title, None))
        .child(card(p, "card", rows))
}

/// A raised card of rows with hairline separators between them.
pub fn card(p: Paint, key: &str, rows: Vec<El>) -> El {
    let mut card = surface(p, col().key(key).padding(4.0));
    for (index, row) in rows.into_iter().enumerate() {
        if index > 0 {
            card = card.child(
                el().key(format!("sep{index}"))
                    .h(p.hair)
                    .ml(12.0)
                    .mr(12.0)
                    .fill(p.k.c.border_muted),
            );
        }
        card = card.child(row);
    }
    card
}

/// The card's raised surface on any element.
pub fn surface(p: Paint, element: El) -> El {
    let mut element = element.fill(p.k.c.bg_base).radius(p.k.radius.lg);
    for shadow in &p.k.elevation.raised {
        element = element.box_shadow(*shadow);
    }
    element
}

/// A setting: title and optional description on the left, its control on the right.
pub fn setting_row(
    p: Paint,
    key: &str,
    title: &str,
    description: Option<(String, Color)>,
    control: El,
) -> El {
    let height = if description.is_some() {
        ROW_HEIGHT_DESCRIBED
    } else {
        ROW_HEIGHT
    };
    row()
        .key(key)
        .min_h(height)
        .px(12.0)
        .py(8.0)
        .gap(24.0)
        .center_y()
        .justify_between()
        .radius(p.k.radius.md)
        .label(title)
        .child(
            col()
                .key("copy")
                .grow(1.0)
                .shrink()
                .gap(2.0)
                .child(text(title).key("label").font(p.k.ty.ui).color(p.text()))
                .children(description.map(|(copy, color)| {
                    text(copy)
                        .key("description")
                        .font(DESCRIPTION)
                        .color(color)
                        .wrap()
                })),
        )
        .child(control)
}

/// A whole-row switch: the row presses, the switch shows the state.
pub fn toggle_row(
    p: Paint,
    key: &str,
    title: &str,
    description: Option<(String, Color)>,
    on: bool,
    flip: impl Fn(&mut Ui) + 'static,
) -> El {
    let switch = Switch::new(on).build(p.k).key("switch").focusable(false);
    setting_row(p, key, title, description, switch)
        .role("toggle")
        .focusable(true)
        .on_press(flip)
}

#[derive(Clone, Copy, PartialEq)]
pub enum ButtonKind {
    Primary,
    Secondary,
}

pub fn button(
    p: Paint,
    key: &str,
    label: &str,
    kind: ButtonKind,
    press: impl Fn(&mut Ui, &mut Tx) + 'static,
) -> El {
    Button::new(label)
        .variant(match kind {
            ButtonKind::Primary => ButtonVariant::Contrast,
            ButtonKind::Secondary => ButtonVariant::Neutral,
        })
        .on_press_tx(press)
        .control_size(ControlSize::Large)
        .build(p.k)
        .px(11.0)
        .radius(p.k.radius.md)
        .key(key)
}

pub fn status_dot(color: Color) -> El {
    el().key("dot").size(8.0, 8.0).radius(4.0).fill(color)
}
