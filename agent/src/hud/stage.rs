//! The HUD in the Playground: the capsule over a mock desktop, with triggers that drive the
//! same flow and view as the live overlay, so its motion can be tried without switching a
//! real workspace.

use rune::{El, Play, Playground, col, el};
use spaceman::{
    Axis, Rect, WorkspaceId,
    app::{UiHudEvent, UiHudKind, UiHudLayout},
};

use super::{HudFlow, LEAVE_MS, pill};
use crate::{
    Pane, Ui,
    controls::Paint,
    playground::{Scene, reset_fixture},
};

/// The workspaces the triggers step through.
const DEMO_WORKSPACES: u8 = 5;
/// Between the switches of a burst: quick presses, each landing while the last still moves.
const BURST_GAP: f64 = 0.16;

/// The stage's HUD: a flow on the Playground's logical clock.
#[derive(Clone, Debug)]
pub struct HudPreview {
    flow: HudFlow,
}

impl HudPreview {
    fn showing(event: UiHudEvent) -> Self {
        let mut flow = HudFlow::new(true);
        flow.trigger(event, 0);
        Self { flow }
    }

    fn workspace(&self) -> u8 {
        self.flow
            .view("", false)
            .and_then(|view| view.event.workspace.number())
            .unwrap_or(1)
    }
}

/// Full-window Playground preview stage showing how the non-activating HUD sits above a desktop.
pub fn stage(preview: &HudPreview, p: Paint, reduce_motion: bool) -> El {
    const STAGE_W: f32 = 560.0;
    const STAGE_H: f32 = 340.0;
    const PAD: f32 = 16.0;
    let inner_w = STAGE_W - 2.0 * p.hair - PAD * 2.0;
    let inner_h = STAGE_H - 2.0 * p.hair - PAD * 2.0;

    let view = preview.flow.view("hud-stage", reduce_motion);
    // The windows keep to their own layer, so one fading out never draws over the capsule.
    let mut windows = el()
        .key("stage-windows")
        .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0));
    match view.as_ref().map(|view| &view.event.preview) {
        Some(UiHudLayout::Fullscreen) => {
            windows = windows.child(stage_window(0, (PAD, PAD, inner_w, inner_h), p));
        }
        Some(UiHudLayout::Split(tiles)) => {
            for (index, rect) in tiles.iter().enumerate() {
                let frame = pill::scale_tile(*rect, PAD, inner_w, inner_h, 4.0, 12.0);
                windows = windows.child(stage_window(index, frame, p));
            }
        }
        None => {}
    }

    let mock = col()
        .key("desktop-mock")
        .w(STAGE_W)
        .h(STAGE_H)
        .radius(p.k.radius.xl)
        .fill(p.k.c.bg_base)
        .border(p.hair, p.k.c.border_weak)
        .pb(28.0)
        .center_y()
        .child(windows)
        .child(el().key("screen-spacer").grow(1.0))
        .children(view.map(|view| pill::slot(&view, p)));

    col()
        .key("hud-stage")
        .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
        .fill(p.k.c.bg_deep)
        .padding(32.0)
        .center()
        .child(mock)
}

fn stage_window(index: usize, (x, y, w, h): (f32, f32, f32, f32), p: Paint) -> El {
    el().key(format!("stage-tile{index}"))
        .absolute(x, y)
        .w(w)
        .h(h)
        .radius(p.k.radius.md)
        .fill(p.k.c.layer_01)
        .border(p.hair, p.k.c.border_weak)
}

/// The motion triggers of the workspace-switch scene.
pub fn triggers(playground: Playground<Ui>) -> Playground<Ui> {
    const SCENE: &str = "HUD/hud-switch";
    playground
        .trigger(SCENE, 'r', "Switch 1, 2, 3 quickly", |ui, play| {
            burst(ui, play, &[1, 2, 3]);
        })
        .trigger(SCENE, 'n', "Next workspace", |ui, play| step(ui, play, 1))
        .trigger(SCENE, 'p', "Previous workspace", |ui, play| {
            step(ui, play, DEMO_WORKSPACES - 1);
        })
        .trigger(SCENE, 'b', "Back and forth", |ui, play| {
            let here = ui.hud_preview.as_ref().map_or(1, HudPreview::workspace);
            let there = if here == 1 { 3 } else { 1 };
            burst(ui, play, &[there, here]);
        })
        .trigger(
            SCENE,
            'm',
            "Move a window to the next workspace",
            |ui, play| {
                let here = ui.hud_preview.as_ref().map_or(1, HudPreview::workspace);
                play.clear();
                let moved = UiHudEvent {
                    kind: UiHudKind::Moved,
                    ..demo(here % DEMO_WORKSPACES + 1)
                };
                show(ui, play, moved);
            },
        )
        .trigger(SCENE, 'l', "Change this workspace's layout", relayout)
        .trigger(SCENE, 'h', "Hide, then show", hide_then_show)
}

fn millis(seconds: f64) -> u64 {
    (seconds * 1_000.0).round() as u64
}

fn show(ui: &mut Ui, play: &mut Play<'_, Ui>, event: UiHudEvent) {
    if let Some(preview) = &mut ui.hud_preview {
        preview.flow.trigger(event, millis(play.now()));
    }
}

fn step(ui: &mut Ui, play: &mut Play<'_, Ui>, by: u8) {
    let here = ui.hud_preview.as_ref().map_or(1, HudPreview::workspace);
    play.clear();
    show(ui, play, demo((here - 1 + by) % DEMO_WORKSPACES + 1));
}

fn burst(ui: &mut Ui, play: &mut Play<'_, Ui>, workspaces: &[u8]) {
    play.clear();
    let Some((&first, rest)) = workspaces.split_first() else {
        return;
    };
    show(ui, play, demo(first));
    for (index, &workspace) in rest.iter().enumerate() {
        play.after(BURST_GAP * (index + 1) as f64, move |ui, play| {
            show(ui, play, demo(workspace));
        });
    }
}

/// Steps this workspace through its layouts: columns, rows, Fullscreen Layout, and back.
fn relayout(ui: &mut Ui, play: &mut Play<'_, Ui>) {
    let Some(view) = ui
        .hud_preview
        .as_ref()
        .and_then(|preview| preview.flow.view("", false))
    else {
        return;
    };
    let windows = view.event.windows.max(2);
    let (kind, preview) = match (&view.event.preview, view.event.kind) {
        (UiHudLayout::Fullscreen, _) => (
            UiHudKind::Fullscreen { enabled: false },
            split(windows, Axis::Horizontal),
        ),
        (
            UiHudLayout::Split(_),
            UiHudKind::Layout {
                axis: Axis::Vertical,
            },
        ) => (
            UiHudKind::Fullscreen { enabled: true },
            UiHudLayout::Fullscreen,
        ),
        (UiHudLayout::Split(_), _) => (
            UiHudKind::Layout {
                axis: Axis::Vertical,
            },
            split(windows, Axis::Vertical),
        ),
    };
    let event = UiHudEvent {
        workspace: view.event.workspace,
        kind,
        preview,
        windows,
    };
    play.clear();
    show(ui, play, event);
}

/// `windows` equal tiles side by side, or stacked for a vertical split.
fn split(windows: usize, axis: Axis) -> UiHudLayout {
    let count = windows.max(1) as i32;
    UiHudLayout::Split(
        (0..count)
            .map(|index| match axis {
                Axis::Horizontal => tile_rect(index * (1500 / count), 0, 1500 / count, 1100),
                Axis::Vertical => tile_rect(0, index * (1100 / count), 1500, 1100 / count),
            })
            .collect(),
    )
}

fn hide_then_show(ui: &mut Ui, play: &mut Play<'_, Ui>) {
    let now_ms = millis(play.now());
    let Some(preview) = &mut ui.hud_preview else {
        return;
    };
    let Some(event) = preview.flow.view("", false).map(|view| view.event.clone()) else {
        return;
    };
    preview.flow.set_enabled(false, now_ms);
    preview.flow.set_enabled(true, now_ms);
    play.clear();
    let leave = LEAVE_MS as f64 / 1_000.0;
    play.after(leave, |ui, play| {
        if let Some(preview) = &mut ui.hud_preview {
            preview.flow.tick(millis(play.now()));
        }
    });
    play.after(leave + 0.4, move |ui, play| show(ui, play, event));
}

/// A different layout for each demo workspace.
fn demo(workspace: u8) -> UiHudEvent {
    let (preview, windows) = match workspace {
        1 => (
            UiHudLayout::Split(vec![
                tile_rect(0, 0, 860, 1100),
                tile_rect(860, 0, 640, 550),
                tile_rect(860, 550, 640, 550),
            ]),
            3,
        ),
        2 => (split(2, Axis::Horizontal), 2),
        3 => (UiHudLayout::Fullscreen, 2),
        4 => (UiHudLayout::Split(Vec::new()), 0),
        _ => (split(1, Axis::Horizontal), 1),
    };
    UiHudEvent {
        workspace: WorkspaceId::new(workspace).unwrap_or(WorkspaceId::ONE),
        kind: UiHudKind::Workspace,
        preview,
        windows,
    }
}

const fn tile_rect(x: i32, y: i32, width: i32, height: i32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn hud_scene(ui: &mut Ui, event: UiHudEvent) {
    reset_fixture(ui, Pane::Rules);
    ui.hud_preview = Some(HudPreview::showing(event));
}

pub(crate) const SCENES: &[Scene] = &[
    ("hud-switch", |ui| hud_scene(ui, demo(1))),
    ("hud-moved", |ui| {
        hud_scene(
            ui,
            UiHudEvent {
                workspace: WorkspaceId::FOUR,
                kind: UiHudKind::Moved,
                preview: split(2, Axis::Horizontal),
                windows: 2,
            },
        );
    }),
    ("hud-fullscreen", |ui| {
        hud_scene(
            ui,
            UiHudEvent {
                workspace: WorkspaceId::ONE,
                kind: UiHudKind::Fullscreen { enabled: true },
                preview: UiHudLayout::Fullscreen,
                windows: 2,
            },
        );
    }),
];
