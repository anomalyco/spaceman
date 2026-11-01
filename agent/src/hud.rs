//! Non-activating Workspace HUD overlay: a compact capsule near the bottom of
//! the primary display showing workspace switches, cross-workspace window
//! moves, and layout toggles with a miniature live split/fullscreen diagram.
//! Never includes window titles or steals focus.

#[cfg(test)]
mod tests;

use std::time::Duration;

use rune::{
    Align, Color, Cx, El, KeyPolicy, OverlayHandle, OverlayInput, OverlayOptions, OverlaySpec,
    ScreenEdge, TextStyle, Tx, col, el, row, text,
};
use spaceman::{
    Axis, Rect, WorkspaceId,
    app::{HUD_PREVIEW_RECT, UiHudEvent, UiHudKind, UiHudLayout},
};

use crate::{
    Pane,
    controls::Paint,
    playground::{Scene, reset_fixture},
};

pub const HOLD_MS: u64 = 1_150;
const HOLD_DURATION: Duration = Duration::from_millis(HOLD_MS);
const FADE_IN: Duration = Duration::from_millis(110);
const FADE_OUT: Duration = Duration::from_millis(180);
const OVERLAY_SIZE: (f32, f32) = (380.0, 68.0);
const OVERLAY_MARGIN: f32 = 48.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HudTransition {
    Hidden,
    Show,
    Update,
}

/// Pure HUD visibility and hold-deadline state machine.
#[derive(Clone, Debug)]
pub struct HudFlow {
    enabled: bool,
    visible: bool,
    hide_at_ms: Option<u64>,
    current: Option<UiHudEvent>,
}

impl Default for HudFlow {
    fn default() -> Self {
        Self::new(true)
    }
}

impl HudFlow {
    #[must_use]
    pub const fn new(enabled: bool) -> Self {
        Self {
            enabled,
            visible: false,
            hide_at_ms: None,
            current: None,
        }
    }

    #[must_use]
    pub const fn current(&self) -> Option<&UiHudEvent> {
        self.current.as_ref()
    }

    #[cfg(test)]
    #[must_use]
    pub const fn visible(&self) -> bool {
        self.visible
    }

    pub fn trigger(&mut self, event: UiHudEvent, now_ms: u64) -> HudTransition {
        if !self.enabled {
            return HudTransition::Hidden;
        }
        let was_visible = self.visible;
        self.visible = true;
        self.hide_at_ms = Some(now_ms.saturating_add(HOLD_MS));
        self.current = Some(event);
        if was_visible {
            HudTransition::Update
        } else {
            HudTransition::Show
        }
    }

    /// Advances the timer to `now_ms`; returns `true` when a visible HUD should fade out.
    pub fn tick(&mut self, now_ms: u64) -> bool {
        let Some(until) = self.hide_at_ms else {
            return false;
        };
        if now_ms < until {
            return false;
        }
        self.hide_at_ms = None;
        let was_visible = self.visible;
        self.visible = false;
        was_visible
    }

    #[must_use]
    pub fn remaining_ms(&self, now_ms: u64) -> Option<u64> {
        self.hide_at_ms
            .filter(|_| self.visible)
            .map(|until| until.saturating_sub(now_ms).max(1))
    }

    /// Updates `enabled`; returns `true` if a currently shown HUD must hide immediately.
    pub fn set_enabled(&mut self, enabled: bool) -> bool {
        self.enabled = enabled;
        if !enabled && self.visible {
            self.visible = false;
            self.hide_at_ms = None;
            return true;
        }
        false
    }
}

struct HudOverlay {
    flow: HudFlow,
    handle: Option<OverlayHandle<HudOverlay>>,
}

#[derive(Clone)]
pub struct HudDriver {
    handle: OverlayHandle<HudOverlay>,
}

pub fn install<S: 'static>(app: &mut rune::App<S>, enabled: bool) -> HudDriver {
    let spec = OverlaySpec {
        name: "Spaceman HUD",
        size: OVERLAY_SIZE,
        edge: ScreenEdge::Bottom,
        margin: OVERLAY_MARGIN,
        align: Align::Center,
        inset: 0.0,
        interactive: false,
        view: overlay_view,
    }
    .options(OverlayOptions {
        input: OverlayInput::None,
        keys: KeyPolicy::None,
        prewarm: false,
        ..Default::default()
    })
    .poll(overlay_poll);

    let handle = app.overlay(
        spec,
        HudOverlay {
            flow: HudFlow::new(enabled),
            handle: None,
        },
    );
    let inner = handle.clone();
    handle.update(move |overlay, _| {
        overlay.handle = Some(inner);
    });
    HudDriver { handle }
}

impl HudDriver {
    pub fn trigger(&self, event: UiHudEvent) {
        let handle = self.handle.clone();
        self.handle.update(move |overlay, tx| {
            let now_ms = spaceman::journal::unix_millis();
            match overlay.flow.trigger(event, now_ms) {
                HudTransition::Hidden => {}
                HudTransition::Show => {
                    tx.wake_after(HOLD_DURATION);
                    handle.fade_in(FADE_IN);
                }
                HudTransition::Update => {
                    tx.wake_after(HOLD_DURATION);
                    handle.show();
                }
            }
        });
    }

    pub fn set_enabled(&self, enabled: bool) {
        let handle = self.handle.clone();
        self.handle.update(move |overlay, _| {
            if overlay.flow.set_enabled(enabled) {
                handle.fade_out(FADE_OUT);
            }
        });
    }
}

fn overlay_poll(overlay: &mut HudOverlay, _clock: f64, tx: &mut Tx) -> bool {
    let now_ms = spaceman::journal::unix_millis();
    if overlay.flow.tick(now_ms) {
        if let Some(handle) = &overlay.handle {
            handle.fade_out(FADE_OUT);
        }
        return true;
    }
    if let Some(remaining_ms) = overlay.flow.remaining_ms(now_ms) {
        tx.wake_after(Duration::from_millis(remaining_ms));
    }
    false
}

fn overlay_view(overlay: &HudOverlay, cx: &mut Cx<HudOverlay>) -> El {
    let p = Paint::new(true, cx.env().scale);
    let Some(event) = overlay.flow.current() else {
        return el().key("hud-empty").w_full().h_full();
    };
    row()
        .key("hud-root")
        .w_full()
        .h_full()
        .center()
        .child(pill(event, p))
}

#[must_use]
pub fn labels(event: &UiHudEvent) -> (String, String) {
    let ws = event.workspace;
    let count_label = match event.windows {
        0 => "Empty".to_owned(),
        1 => "1 window".to_owned(),
        n => format!("{n} windows"),
    };
    match event.kind {
        UiHudKind::Workspace => {
            let detail = match (&event.preview, event.windows) {
                (UiHudLayout::Fullscreen, 0) => "Fullscreen".to_owned(),
                (UiHudLayout::Fullscreen, _) => format!("Fullscreen · {count_label}"),
                _ => count_label,
            };
            (format!("Workspace {ws}"), detail)
        }
        UiHudKind::Moved => (format!("Moved to Workspace {ws}"), count_label),
        UiHudKind::Layout { axis } => {
            let split = match axis {
                Axis::Horizontal => "Horizontal split",
                Axis::Vertical => "Vertical split",
            };
            (format!("Workspace {ws}"), split.to_owned())
        }
        UiHudKind::Fullscreen { enabled } => (
            format!("Workspace {ws}"),
            if enabled {
                "Fullscreen layout".to_owned()
            } else {
                "Split layout".to_owned()
            },
        ),
    }
}

/// Renders the HUD capsule itself (shared by the live `OverlaySpec` and Playground scenes).
pub fn pill(event: &UiHudEvent, p: Paint) -> El {
    let (headline, detail) = labels(event);
    let dark = Paint::new(true, 1.0 / p.hair);
    let mut capsule = row()
        .key("hud-pill")
        .h(44.0)
        .pl(12.0)
        .pr(16.0)
        .gap(12.0)
        .center_y()
        .radius(22.0)
        .fill(srgb(0x161618))
        .border(p.hair, srgb(0x303036))
        .child(mini_layout(&event.preview, p.hair))
        .child(
            row()
                .key("copy")
                .gap(8.0)
                .center_y()
                .child(
                    text(headline)
                        .key("headline")
                        .font(TextStyle::new(13.0, 620.0).lh(16.0))
                        .color(srgb(0xf4f4f6)),
                )
                .child(
                    text("·")
                        .key("dot")
                        .font(TextStyle::new(13.0, 500.0).lh(16.0))
                        .color(srgb(0x6c6c76)),
                )
                .child(
                    text(detail)
                        .key("detail")
                        .font(TextStyle::new(13.0, 460.0).lh(16.0))
                        .color(srgb(0xa4a4ae)),
                ),
        );
    for shadow in &dark.k.elevation.overlay {
        capsule = capsule.box_shadow(*shadow);
    }
    capsule
}

fn mini_layout(layout: &UiHudLayout, hair: f32) -> El {
    const OUTER_W: f32 = 30.0;
    const OUTER_H: f32 = 20.0;
    const INSET: f32 = 2.5;
    let inner_w = OUTER_W - 2.0 * hair - INSET * 2.0;
    let inner_h = OUTER_H - 2.0 * hair - INSET * 2.0;

    let mut screen = el()
        .key("mini-screen")
        .w(OUTER_W)
        .h(OUTER_H)
        .radius(4.5)
        .fill(rgba(0xffffff, 0.05))
        .border(hair, rgba(0xffffff, 0.22));

    match layout {
        UiHudLayout::Fullscreen => {
            screen = screen.child(
                el().key("fullscreen")
                    .pin(Some(INSET), Some(INSET), Some(INSET), Some(INSET))
                    .radius(2.2)
                    .fill(rgba(0xffffff, 0.86)),
            );
        }
        UiHudLayout::Split(tiles) if tiles.is_empty() => {
            screen = screen.child(
                row()
                    .key("empty-wrap")
                    .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
                    .center()
                    .child(
                        el().key("empty")
                            .w(3.0)
                            .h(3.0)
                            .radius(1.5)
                            .fill(rgba(0xffffff, 0.28)),
                    ),
            );
        }
        UiHudLayout::Split(tiles) => {
            for (index, rect) in tiles.iter().enumerate() {
                let (x, y, w, h) = scale_tile(*rect, INSET, inner_w, inner_h, 0.6, 2.0);
                screen = screen.child(
                    el().key(format!("tile{index}"))
                        .absolute(x, y)
                        .w(w)
                        .h(h)
                        .radius(1.8)
                        .fill(rgba(0xffffff, 0.80)),
                );
            }
        }
    }
    screen
}

/// Full-window Playground preview stage showing how the non-activating HUD sits above a desktop.
pub fn stage(event: &UiHudEvent, p: Paint) -> El {
    const STAGE_W: f32 = 560.0;
    const STAGE_H: f32 = 340.0;
    const PAD: f32 = 16.0;
    let inner_w = STAGE_W - 2.0 * p.hair - PAD * 2.0;
    let inner_h = STAGE_H - 2.0 * p.hair - PAD * 2.0;

    let mut mock = col()
        .key("desktop-mock")
        .w(STAGE_W)
        .h(STAGE_H)
        .radius(p.k.radius.xl)
        .fill(p.k.c.bg_base)
        .border(p.hair, p.k.c.border_weak)
        .pb(28.0)
        .center_y();

    match &event.preview {
        UiHudLayout::Fullscreen => {
            mock = mock.child(
                el().key("stage-fullscreen")
                    .pin(Some(PAD), Some(PAD), Some(PAD), Some(PAD))
                    .radius(p.k.radius.md)
                    .fill(p.k.c.layer_01)
                    .border(p.hair, p.k.c.border_weak),
            );
        }
        UiHudLayout::Split(tiles) => {
            for (index, rect) in tiles.iter().enumerate() {
                let (x, y, w, h) = scale_tile(*rect, PAD, inner_w, inner_h, 4.0, 12.0);
                mock = mock.child(
                    el().key(format!("stage-tile{index}"))
                        .absolute(x, y)
                        .w(w)
                        .h(h)
                        .radius(p.k.radius.md)
                        .fill(p.k.c.layer_01)
                        .border(p.hair, p.k.c.border_weak),
                );
            }
        }
    }

    mock = mock
        .child(el().key("screen-spacer").grow(1.0))
        .child(pill(event, p));

    col()
        .key("hud-stage")
        .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
        .fill(p.k.c.bg_deep)
        .padding(32.0)
        .center()
        .child(mock)
}

fn scale_tile(
    rect: Rect,
    inset: f32,
    inner_w: f32,
    inner_h: f32,
    gap: f32,
    min_side: f32,
) -> (f32, f32, f32, f32) {
    let pw = HUD_PREVIEW_RECT.width.max(1) as f32;
    let ph = HUD_PREVIEW_RECT.height.max(1) as f32;
    (
        inset + (rect.x as f32 / pw) * inner_w + gap,
        inset + (rect.y as f32 / ph) * inner_h + gap,
        ((rect.width as f32 / pw) * inner_w - gap * 2.0).max(min_side),
        ((rect.height as f32 / ph) * inner_h - gap * 2.0).max(min_side),
    )
}

const fn tile_rect(x: i32, y: i32, width: i32, height: i32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn hud_scene(
    ui: &mut crate::Ui,
    workspace: WorkspaceId,
    kind: UiHudKind,
    preview: UiHudLayout,
    windows: usize,
) {
    reset_fixture(ui, Pane::Rules);
    ui.hud_preview = Some(UiHudEvent {
        workspace,
        kind,
        preview,
        windows,
    });
}

pub(crate) const SCENES: &[Scene] = &[
    ("hud-switch", |ui| {
        hud_scene(
            ui,
            WorkspaceId::TWO,
            UiHudKind::Workspace,
            UiHudLayout::Split(vec![
                tile_rect(0, 0, 860, 1100),
                tile_rect(860, 0, 640, 550),
                tile_rect(860, 550, 640, 550),
            ]),
            3,
        );
    }),
    ("hud-moved", |ui| {
        hud_scene(
            ui,
            WorkspaceId::FOUR,
            UiHudKind::Moved,
            UiHudLayout::Split(vec![
                tile_rect(0, 0, 750, 1100),
                tile_rect(750, 0, 750, 1100),
            ]),
            2,
        );
    }),
    ("hud-fullscreen", |ui| {
        hud_scene(
            ui,
            WorkspaceId::ONE,
            UiHudKind::Fullscreen { enabled: true },
            UiHudLayout::Fullscreen,
            2,
        );
    }),
];

fn srgb(hex: u32) -> Color {
    rgba(hex, 1.0)
}

fn rgba(hex: u32, alpha: f32) -> Color {
    let channel = |shift: u32| rune::color::srgb_decode(((hex >> shift) & 0xff) as f32 / 255.0);
    Color {
        r: channel(16),
        g: channel(8),
        b: channel(0),
        a: alpha,
    }
}
