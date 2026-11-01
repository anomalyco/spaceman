//! The window: Hex's settings shell. A sidebar of panes under the traffic lights, with the
//! version and an update check at its foot, beside a raised panel holding the pane.

use rune::{Anim, Cx, El, Icon, ScrollSpec, Tx, col, el, icon, row, text};

use crate::{
    Pane, Ui, activity,
    agent::Update,
    controls::{self, Paint},
    hud, map, onboarding, panes,
};

static GENERAL_ICON: Icon = Icon {
    name: "spaceman-general",
    view: 16.0,
    body: include_str!("../assets/general.svg"),
};
static RULES_ICON: Icon = Icon {
    name: "spaceman-rules",
    view: 16.0,
    body: include_str!("../assets/rules.svg"),
};
static WORKSPACES_ICON: Icon = Icon {
    name: "spaceman-workspaces",
    view: 16.0,
    body: include_str!("../assets/workspaces.svg"),
};
static ACTIVITY_ICON: Icon = Icon {
    name: "spaceman-activity",
    view: 24.0,
    body: include_str!("../assets/activity.svg"),
};

fn pane_icon(pane: Pane) -> &'static Icon {
    match pane {
        Pane::General => &GENERAL_ICON,
        Pane::Rules => &RULES_ICON,
        Pane::Workspaces => &WORKSPACES_ICON,
        Pane::Shortcuts => &rune_theme_opencode::icons::KEYBOARD,
        Pane::Activity => &ACTIVITY_ICON,
    }
}

/// The traffic lights sit over the sidebar, so the navigation starts below them.
const SIDEBAR_TOP: f32 = 52.0;
/// OpenCode Desktop's settings navigation: 28 pt rows with a 4 pt gap.
const NAV_ROW_HEIGHT: f32 = 28.0;
const NAV_ROW_GAP: f32 = 4.0;
pub const SIDEBAR_WIDTH: f32 = 220.0;
/// The panel's inset from the window's top, right and bottom.
pub const PANEL_INSET: f32 = 8.0;
pub const HEADER_HEIGHT: f32 = 56.0;
pub const CONTENT_PADDING: f32 = 24.0;

pub fn view(ui: &Ui, cx: &mut Cx<Ui>) -> El {
    let p = Paint::new(cx.system_dark(), cx.env().scale);
    // Menus mount above the page (rune-kit overlays).
    let mut layer = Vec::new();
    let (trailing, body) = match ui.pane {
        Pane::General => (None, panes::general(ui, p)),
        Pane::Rules => (None, map::pane(ui, p, &mut layer)),
        Pane::Workspaces => (None, panes::workspaces(ui, p)),
        Pane::Shortcuts => (None, panes::shortcuts(p)),
        Pane::Activity => (Some(activity::header_actions(ui, p)), activity::pane(ui, p)),
    };
    let panel = col()
        .key("panel")
        .grow(1.0)
        .shrink()
        .mt(PANEL_INSET)
        .mr(PANEL_INSET)
        .mb(PANEL_INSET)
        .radius(p.k.radius.lg)
        .fill(p.k.c.bg_base)
        .border(p.hair, p.k.c.border_weak)
        .padding(p.hair)
        .clip_soft(0.0)
        .child(pane_header(p, ui.pane.title(), trailing))
        .children(error_banner(ui, p))
        .child(body);
    let window = row()
        .key("window")
        .role("window")
        .fill(p.k.c.bg_deep)
        .hover_motion_within(Anim::INSTANT, Anim::INSTANT)
        .child(sidebar(ui, p))
        .child(panel);
    if let Some(tour) = onboarding::overlay(ui, p) {
        layer.push(tour);
    }
    if let Some(hud_preview) = &ui.hud_preview {
        layer.push(hud::stage(hud_preview, p));
    }
    rune_kit::root(window, layer)
}

fn sidebar(ui: &Ui, p: Paint) -> El {
    let mut nav = col().key("nav").role("list").focus_group().focus_follows();
    for pane in Pane::ALL {
        nav = nav.child(nav_slot(p, pane, ui.pane == pane));
    }
    col()
        .window_drag()
        .key("sidebar")
        .role("navigation")
        .w(SIDEBAR_WIDTH)
        .h_full()
        .pt(SIDEBAR_TOP - NAV_ROW_GAP / 2.0)
        .pb(16.0)
        .pl(10.0)
        .pr(6.0)
        .child(nav)
        .child(el().key("spacer").grow(1.0))
        .child(
            text(format!("Version {}", env!("CARGO_PKG_VERSION")))
                .key("version")
                .font(p.k.ty.small)
                .color(p.faint())
                .pl(6.0),
        )
        .child(update_status(ui, p))
}

/// Under the version: a manual check, or what the updater is doing.
fn update_status(ui: &Ui, p: Paint) -> El {
    let c = &p.k.c;
    let update = ui
        .agent
        .clone()
        .flatten()
        .map_or(Update::Unavailable, |agent| agent.update);
    let label = match update {
        Update::Checking => "Checking for updates…",
        Update::UpToDate => "Spaceman is up to date",
        Update::Available => "Update available",
        Update::Idle | Update::Unavailable => "Check for updates",
    };
    let check = matches!(update, Update::Idle | Update::UpToDate | Update::Available);
    let mut status = row()
        .key("check-for-updates")
        .h(20.0)
        .mt(2.0)
        .px(6.0)
        .center_y()
        .radius(p.k.radius.sm)
        .child(
            text(label)
                .key("label")
                .font(p.k.ty.small)
                .color(c.text_muted)
                .truncate(),
        );
    if check {
        status = status
            .role("button")
            .label("Check for updates")
            .hover(c.hover)
            .press(c.pressed)
            .focusable(true)
            .focus_ring(2.0, -2.0, c.border_focus)
            .on_press(|ui: &mut Ui| ui.check_for_updates());
    }
    status
}

/// Slots tile the item column, each reaching halfway into the gaps around its item, so a press
/// anywhere in the column selects the nearest item.
fn nav_slot(p: Paint, pane: Pane, on: bool) -> El {
    col()
        .key(format!("{}-slot", pane.key()))
        .py(NAV_ROW_GAP / 2.0)
        .hover_group()
        .on_drag(move |ui: &mut Ui, d: &mut rune::Drag, _: &mut Tx| {
            if d.phase == rune::Phase::Down {
                ui.select_pane(pane);
            }
        })
        .on_press(move |ui: &mut Ui| ui.select_pane(pane))
        .child(nav_item(p, pane, on))
}

fn nav_item(p: Paint, pane: Pane, on: bool) -> El {
    let c = &p.k.c;
    let mut item = row()
        .key(pane.key())
        .role("button")
        .label(pane.title())
        .h(NAV_ROW_HEIGHT)
        .px(6.0)
        .gap(8.0)
        .center_y()
        .radius(p.k.radius.sm)
        .press(c.pressed)
        .on_press(move |ui: &mut Ui| ui.select_pane(pane))
        .child(
            el().key("hover")
                .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
                .radius(p.k.radius.sm)
                .fill(c.hover)
                .group_opacity(0.0, 1.0),
        )
        .child(icon(pane_icon(pane), 16.0).key("icon").color(if on {
            c.icon_base
        } else {
            c.icon_muted
        }))
        .child(
            text(pane.title())
                .key("label")
                .font(p.k.ty.ui)
                .color(if on { c.text_base } else { c.text_muted })
                .truncate(),
        );
    if on {
        item = item.fill(c.layer_02);
    }
    item.focusable(true)
        .focus_selected(on)
        .focus_ring(2.0, -2.0, c.border_focus)
}

/// The panel's title bar, which also moves the window.
fn pane_header(p: Paint, title: &str, trailing: Option<El>) -> El {
    row()
        .window_drag()
        .key("pane-header")
        .h(HEADER_HEIGHT)
        .px(CONTENT_PADDING)
        .gap(8.0)
        .center_y()
        .child(
            text(title)
                .key("pane-title")
                .font(p.k.ty.body_medium)
                .color(p.text())
                .shrink()
                .truncate(),
        )
        .child(el().key("header-spacer").grow(1.0))
        .children(trailing)
}

/// A save that failed, or a config file Spaceman can't read.
fn error_banner(ui: &Ui, p: Paint) -> Option<El> {
    let copy = ui.error.as_deref().or(ui.doc.error.as_deref())?;
    Some(
        row()
            .key("error")
            .mx(CONTENT_PADDING)
            .mb(8.0)
            .px(12.0)
            .py(8.0)
            .gap(8.0)
            .center_y()
            .radius(p.k.radius.md)
            .fill(p.k.c.danger.bg)
            .child(controls::status_dot(p.k.c.danger.fg))
            .child(
                text(copy)
                    .key("copy")
                    .font(controls::DESCRIPTION)
                    .color(p.text())
                    .wrap()
                    .grow(1.0)
                    .shrink(),
            ),
    )
}

/// A vertically scrolling pane body.
pub fn scroll_body(key: &str, content: El) -> El {
    col()
        .key(key)
        .grow(1.0)
        .shrink()
        .clip_fade(0.0, 4.0, 0.0, 24.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(
            col()
                .key("inset")
                .px(CONTENT_PADDING)
                .pt(4.0)
                .pb(24.0)
                .child(content),
        )
}
