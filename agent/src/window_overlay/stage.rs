//! The window overlay in the Playground and in headless snaps: a mock desktop of light and
//! dark windows under the real dim and card, so their look can be judged on both without
//! holding a chord over a real desktop. Drawn only: nothing here presses or hovers.

use std::collections::HashMap;

use rune::{El, col, el, oklch, row};
use spaceman::{
    AppInstanceId, Direction, PhysicalWindowId, Rect, WorkspaceId,
    app::{UiWindowOverlayDisplay, UiWindowOverlayWindow, UiWindowOverlayWorkspace},
};

use super::{Control, Hover, Scene, Wiring, dim, layers, look};
use crate::{
    Pane, Ui, apps,
    hud::Icons,
    playground::{Scene as PlaygroundScene, reset_fixture},
};

/// The mock display, in points.
const SCREEN: Rect = Rect {
    x: 0,
    y: 0,
    width: 1440,
    height: 900,
};
const MENU_BAR: i32 = 28;
const GAP: i32 = 8;

const TERMINAL: &str = "com.apple.Terminal";
const SAFARI: &str = "com.apple.Safari";
const NOTES: &str = "com.apple.Notes";
const CALCULATOR: &str = "com.apple.calculator";
const MAIL: &str = "com.apple.mail";
const MESSAGES: &str = "com.apple.MobileSMS";
const MUSIC: &str = "com.apple.Music";
const PREVIEW: &str = "com.apple.Preview";

/// A display's overlay over mock windows.
#[derive(Clone, Debug)]
pub struct OverlayPreview {
    display: UiWindowOverlayDisplay,
    /// Which windows the mock desktop draws light, by window ID.
    light: Vec<u32>,
    icons: Icons,
    names: HashMap<String, String>,
    hover: Hover,
}

fn id(window_id: u32) -> PhysicalWindowId {
    PhysicalWindowId {
        app: AppInstanceId {
            pid: 100 + window_id as i32,
            launched_at: 1,
        },
        window_id,
    }
}

fn window(window_id: u32, bundle: &str, frame: (i32, i32, i32, i32)) -> UiWindowOverlayWindow {
    UiWindowOverlayWindow {
        window: id(window_id),
        bundle_id: Some(bundle.to_owned()),
        frame: Rect {
            x: frame.0,
            y: frame.1,
            width: frame.2,
            height: frame.3,
        },
        workspace: WorkspaceId::ONE,
        floating: false,
        pinned: false,
        focused: false,
        swaps: Vec::new(),
    }
}

/// Ten workspaces, the first five on this display: 1 is the windows shown, 2 and 3 hold other
/// apps and 4 is the next empty one. Display 2 shows 6 and holds 8; the rest are empty.
fn workspaces(shown: &[UiWindowOverlayWindow]) -> Vec<UiWindowOverlayWorkspace> {
    WorkspaceId::numbered()
        .zip(1..)
        .map(|(workspace, number)| {
            let apps: Vec<&str> = match number {
                1 => shown
                    .iter()
                    .filter(|window| !window.pinned)
                    .filter_map(|window| window.bundle_id.as_deref())
                    .collect(),
                2 => vec![MAIL, MESSAGES],
                3 => vec![MUSIC],
                6 => vec![PREVIEW, MAIL, MESSAGES, MUSIC],
                8 => vec![NOTES],
                _ => Vec::new(),
            };
            UiWindowOverlayWorkspace {
                workspace,
                windows: if number == 6 { 5 } else { apps.len() },
                apps: apps.into_iter().map(str::to_owned).collect(),
                on_this_display: number <= 5,
                display: if number <= 5 { 1 } else { 2 },
                visible: number == 1 || number == 6,
            }
        })
        .collect()
}

impl OverlayPreview {
    fn new(windows: Vec<UiWindowOverlayWindow>, light: &[u32], hover: Hover) -> Self {
        let display = UiWindowOverlayDisplay {
            id: "stage".to_owned(),
            screen: SCREEN,
            workspace: WorkspaceId::ONE,
            fullscreen: false,
            workspaces: workspaces(&windows),
            windows,
        };
        let mut catalog = apps::Catalog::default();
        let (icons, names) = catalog.icons_and_names(super::bundles(&display));
        Self {
            display,
            light: light.to_vec(),
            icons,
            names,
            hover,
        }
    }
}

/// A main window and two stacked beside it, with a small float over them.
fn desk(hover: Hover) -> OverlayPreview {
    let top = MENU_BAR + GAP;
    let (left_w, right_x) = (708, GAP + 708 + GAP);
    let right_w = SCREEN.width - right_x - GAP;
    let half = (SCREEN.height - top - GAP - GAP) / 2;
    let mut terminal = window(1, TERMINAL, (GAP, top, left_w, SCREEN.height - top - GAP));
    terminal.focused = true;
    terminal.swaps = vec![Direction::Right];
    let mut safari = window(2, SAFARI, (right_x, top, right_w, half));
    safari.swaps = vec![Direction::Left, Direction::Down];
    let mut notes = window(3, NOTES, (right_x, top + half + GAP, right_w, half));
    notes.swaps = vec![Direction::Left, Direction::Up];
    let mut calculator = window(4, CALCULATOR, (560, 500, 250, 340));
    calculator.floating = true;
    OverlayPreview::new(vec![terminal, safari, notes, calculator], &[2, 3], hover)
}

/// Five columns, each narrower than a full card, the last two of one app.
fn columns() -> OverlayPreview {
    let top = MENU_BAR + GAP;
    let width = (SCREEN.width - 6 * GAP) / 5;
    let apps = [TERMINAL, SAFARI, NOTES, MAIL, MAIL];
    let windows = apps
        .iter()
        .zip(0..)
        .map(|(bundle, column)| {
            let x = GAP + column * (width + GAP);
            let mut tile = window(
                column as u32 + 1,
                bundle,
                (x, top, width, SCREEN.height - top - GAP),
            );
            tile.focused = column == 1;
            tile.swaps = [Direction::Left, Direction::Right]
                .into_iter()
                .filter(|side| match side {
                    Direction::Left => column > 0,
                    _ => column < 4,
                })
                .collect();
            tile
        })
        .collect();
    OverlayPreview::new(windows, &[2, 3, 4], Hover::on_control(&id(3), Control::Pin))
}

/// Fullscreen Layout's one tile under a pinned float, with two floats stacked in one place.
/// `hover` picks the window: the tile's dim is cut round all three floats.
fn fullscreen(hover: Hover) -> OverlayPreview {
    let top = MENU_BAR + GAP;
    let mut safari = window(
        1,
        SAFARI,
        (GAP, top, SCREEN.width - 2 * GAP, SCREEN.height - top - GAP),
    );
    safari.focused = true;
    let mut music = window(2, MUSIC, (880, 140, 420, 300));
    music.floating = true;
    music.pinned = true;
    let mut first = window(3, CALCULATOR, (180, 430, 400, 320));
    first.floating = true;
    let mut second = window(4, NOTES, (200, 450, 400, 320));
    second.floating = true;
    let mut preview = OverlayPreview::new(vec![safari, music, first, second], &[1, 4], hover);
    preview.display.fullscreen = true;
    preview
}

/// The mock desktop, then the overlay's own layers over it.
pub fn stage(preview: &OverlayPreview, hair: f32, reduce_motion: bool) -> El {
    let mut desktop = el()
        .key("overlay-stage")
        .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
        .fill(oklch(0.24, 0.05, 275.0))
        .gradient(oklch(0.30, 0.07, 255.0), oklch(0.17, 0.05, 300.0))
        .child(menu_bar());
    for window in &preview.display.windows {
        let light = preview.light.contains(&window.window.window_id);
        desktop = desktop.child(mock_window(window, light, hair));
    }
    desktop.children(layers(&Scene {
        display: &preview.display,
        icons: &preview.icons,
        names: &preview.names,
        hover: &preview.hover,
        elsewhere: false,
        still: reduce_motion,
        wiring: Wiring { hair, live: false },
    }))
}

fn menu_bar() -> El {
    el().key("menu-bar")
        .absolute(0.0, 0.0)
        .size(SCREEN.width as f32, MENU_BAR as f32)
        .fill(look::black(0.35))
        .a11y_hidden()
}

/// A window of the mock desktop: a title bar with its three lights, and lines of "text" at
/// full contrast, the hardest thing for a card to sit on.
fn mock_window(window: &UiWindowOverlayWindow, light: bool, hair: f32) -> El {
    let frame = window.frame;
    let (fill, bar, ink, rim) = if light {
        (
            oklch(0.985, 0.0, 0.0),
            oklch(0.93, 0.004, 262.0),
            oklch(0.25, 0.01, 262.0),
            look::black(0.18),
        )
    } else {
        (
            oklch(0.19, 0.008, 262.0),
            oklch(0.24, 0.008, 262.0),
            oklch(0.90, 0.01, 262.0),
            look::white(0.12),
        )
    };
    let lights = [
        oklch(0.68, 0.19, 25.0),
        oklch(0.82, 0.16, 85.0),
        oklch(0.75, 0.19, 145.0),
    ];
    let title_bar = row()
        .key("title-bar")
        .h(36.0)
        .px(14.0)
        .gap(8.0)
        .center_y()
        .fill(bar)
        .children(lights.into_iter().zip(0..).map(|(color, index)| {
            el().key(format!("light-{index}"))
                .size(12.0, 12.0)
                .radius(6.0)
                .fill(color)
        }));
    let lines = (0..((frame.height - 80) / 22).max(0)).map(|index| {
        // Ragged right, like prose.
        let width = 0.92 - 0.11 * ((index * 7) % 5) as f32;
        el().key(format!("line-{index}"))
            .h(8.0)
            .w((frame.width as f32 - 48.0) * width)
            .radius(2.0)
            .fill(ink.alpha(if index % 6 == 0 { 0.9 } else { 0.55 }))
    });
    col()
        .key(format!("window-{}", window.window.window_id))
        .absolute(frame.x as f32, frame.y as f32)
        .size(frame.width as f32, frame.height as f32)
        .radius(dim::RADIUS)
        .clip_rounded(dim::RADIUS)
        .fill(fill)
        .border(hair, rim)
        .shadow(
            if window.floating { 18.0 } else { 6.0 },
            if window.floating { 48.0 } else { 20.0 },
            look::black(0.45),
        )
        .a11y_hidden()
        .child(title_bar)
        .child(col().key("page").padding(24.0).gap(14.0).children(lines))
}

fn scene(ui: &mut Ui, preview: OverlayPreview) {
    reset_fixture(ui, Pane::Labs);
    ui.overlay_preview = Some(preview);
}

pub(crate) const SCENES: &[PlaygroundScene] = &[
    // The pointer on no window: the focused one, a dark terminal, has the dim and the card.
    ("overlay", |ui| scene(ui, desk(Hover::default()))),
    // On a light window, off its card.
    ("overlay-hover", |ui| {
        scene(ui, desk(Hover::on_window(&id(2))));
    }),
    ("overlay-send", |ui| {
        let two = WorkspaceId::new(2).unwrap_or(WorkspaceId::ONE);
        scene(ui, desk(Hover::on_control(&id(2), Control::Send(two))));
    }),
    ("overlay-other-display", |ui| {
        let six = WorkspaceId::new(6).unwrap_or(WorkspaceId::ONE);
        scene(ui, desk(Hover::on_control(&id(4), Control::Send(six))));
    }),
    ("overlay-swap", |ui| {
        let swap = Control::Swap(Direction::Right);
        scene(ui, desk(Hover::on_control(&id(1), swap)));
    }),
    ("overlay-columns", |ui| scene(ui, columns())),
    // The tile under three floats, then the pinned float over it.
    ("overlay-fullscreen", |ui| {
        scene(ui, fullscreen(Hover::on_window(&id(1))));
    }),
    ("overlay-pinned", |ui| {
        scene(ui, fullscreen(Hover::on_control(&id(2), Control::Pin)));
    }),
];
