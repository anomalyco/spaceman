//! The 4-step Rune setup tour view: Welcome, Permissions, Starter Rules, and Done.

use rune::{
    Color, El, Icon, Shadow, Spring, SpringPair, TextStyle, Transition, Tx, col, el, icon, row,
    text,
};
use rune_kit::{
    Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, SegmentedControl, glyphs,
};
use spaceman::{WorkspaceId, app::UiLaunchAtLogin};

use super::{Input, StarterApp, Step, Tour, flow::View, input};
use crate::{
    Ui,
    controls::{self, ButtonKind, DESCRIPTION, Paint},
    rules::Place,
};

const WIDTH: f32 = 520.0;
const FOOTER_SLOT: f32 = 176.0;
const LEAD: TextStyle = TextStyle::new(15.0, 400.0).lh(22.0);
const STEP_ENTER: Spring = Spring::new(0.3, 0.0);
const STEP_EXIT: Spring = Spring::new(0.25, 0.0);

static ACCESSIBILITY_ICON: Icon = Icon {
    name: "spaceman-accessibility",
    view: 16.0,
    body: include_str!("../../assets/workspaces.svg"),
};

fn title_style(dark: bool) -> TextStyle {
    TextStyle::new(26.0, if dark { 540.0 } else { 600.0 })
        .lh(32.0)
        .tracking(-0.3)
}

pub fn overlay(ui: &Ui, p: Paint) -> Option<El> {
    let tour = ui.tour.as_ref()?;
    let view = tour.flow.view();
    let step_body = match view.step {
        Step::Welcome => welcome(p, &tour.conflicts),
        Step::Setup => setup(ui, p, &view),
        Step::Rules => rules_step(ui, p, &tour.starter_apps),
        Step::Done => done(p),
    };
    let page = col()
        .key(format!("step-{}", view.step.key()))
        .w(WIDTH)
        .transition(step_transition(view.direction))
        .child(step_body);
    let top = row()
        .window_drag()
        .key("tour-top")
        .w_full()
        .h(48.0)
        .px(20.0)
        .center_y()
        .justify_end()
        .children(view.manual.then(|| {
            IconButton::new(&rune_theme_opencode::icons::CLOSE)
                .variant(IconButtonVariant::Ghost)
                .control_size(ControlSize::Large)
                .label("Close setup tour")
                .on_press(|ui: &mut Ui| input(ui, Input::Dismiss))
                .build(p.k)
                .key("tour-close")
        }));
    Some(
        col()
            .key("tour")
            .role("dialog")
            .label("Spaceman setup")
            .absolute(0.0, 0.0)
            .w_full()
            .h_full()
            .fill(p.k.c.bg_deep)
            .transition(Transition::blur_replace())
            .on_press(|_: &mut Ui| {})
            .child(top)
            .child(
                col().key("tour-page").grow(1.0).shrink().center().child(
                    col()
                        .key("tour-column")
                        .w(WIDTH)
                        .motion(STEP_ENTER)
                        .child(page),
                ),
            )
            .child(
                col()
                    .key("tour-bottom")
                    .pt(12.0)
                    .pb(28.0)
                    .center()
                    .child(footer(p, &view)),
            ),
    )
}

fn step_transition(direction: i8) -> Transition {
    if direction == 0 {
        return Transition::blur_replace();
    }
    let sign = if direction > 0 { 1.0 } else { -1.0 };
    let enter = Transition::offset(sign * 24.0, 0.0).combined(Transition::opacity());
    let exit = Transition::offset(-sign * 24.0, 0.0).combined(Transition::opacity());
    Transition::asymmetric(enter, exit).springs(SpringPair::new(STEP_ENTER, STEP_EXIT))
}

fn footer(p: Paint, view: &View) -> El {
    let (label, enabled, forward) = match view.step {
        Step::Welcome => ("Get started", true, true),
        Step::Setup => ("Continue", view.ready, true),
        Step::Rules => ("Continue", true, true),
        Step::Done => ("Start using Spaceman", true, false),
    };
    let mut btn = Button::new(label)
        .variant(ButtonVariant::Contrast)
        .control_size(ControlSize::Large);
    if forward {
        btn = btn.label_with(move |font, ink| {
            row()
                .gap(6.0)
                .center_y()
                .child(text(label).key("label").font(font).color(ink))
                .child(icon(&glyphs::ARROW_RIGHT, 16.0).key("arrow").color(ink))
        });
    }
    let primary = if enabled {
        btn.on_press(|ui: &mut Ui| input(ui, Input::Continue))
            .build(p.k)
            .px(12.0)
            .radius(p.k.radius.md)
            .key("tour-continue")
            .default_button()
    } else {
        btn.disabled(true)
            .build(p.k)
            .px(12.0)
            .radius(p.k.radius.md)
            .opacity(if p.k.dark { 0.3 } else { 1.0 })
            .key("tour-continue")
    };
    let back = view.can_go_back.then(|| {
        controls::button(p, "tour-back", "Back", ButtonKind::Secondary, |ui, _| {
            input(ui, Input::Back);
        })
    });
    row()
        .key("tour-footer")
        .w(WIDTH)
        .h(32.0)
        .center_y()
        .child(row().key("start").w(FOOTER_SLOT).children(back))
        .child(
            row()
                .key("middle")
                .grow(1.0)
                .w(0.0)
                .center()
                .children(step_dots(p, view)),
        )
        .child(row().key("end").w(FOOTER_SLOT).justify_end().child(primary))
}

fn step_dots(p: Paint, view: &View) -> Option<El> {
    if view.step == Step::Done || view.count < 2 {
        return None;
    }
    let mut dots = row().key("step-dots").gap(6.0).center_y();
    for index in 0..view.count {
        let current = index == view.index;
        dots = dots.child(
            el().key(format!("dot{index}"))
                .size(if current { 16.0 } else { 6.0 }, 6.0)
                .radius(3.0)
                .fill(if current { p.text() } else { p.k.c.border_base })
                .motion(STEP_ENTER),
        );
    }
    Some(dots)
}

fn title_block(p: Paint, heading: &str, body: Option<&str>) -> El {
    col()
        .key("title-block")
        .gap(10.0)
        .center()
        .child(
            text(heading)
                .key("title")
                .font(title_style(p.k.dark))
                .color(p.text()),
        )
        .children(body.map(|body| {
            text(body)
                .key("body")
                .font(LEAD)
                .color(p.muted())
                .align(rune::Align::Center)
                .wrap()
                .w(440.0)
        }))
}

fn welcome(p: Paint, conflicts: &[String]) -> El {
    let mut column = col()
        .key("welcome")
        .gap(28.0)
        .child(welcome_hero(p))
        .child(title_block(
            p,
            "Welcome to Spaceman",
            Some(
                "Fast macOS workspaces and automatic tiling, with every window restored if you pause or quit.",
            ),
        ));
    if let Some(banner) = conflict_banner(p, conflicts) {
        column = column.child(banner);
    }
    column
}

/// Spaceman's split-tile mark inside a concentric halo ring.
fn welcome_hero(p: Paint) -> El {
    let c = &p.k.c;
    let badge_ink = rune::oklch(0.21, 0.012, 265.0);
    let tile_ink = rune::oklch(0.97, 0.012, 270.0);
    let mut badge = el()
        .key("mark-badge")
        .size(92.0, 92.0)
        .radius(22.0)
        .fill(badge_ink)
        .border(p.hair, Color::WHITE.alpha(0.10))
        .box_shadow(Shadow {
            y: 2.0,
            blur: 4.0,
            color: Color::BLACK.alpha(if p.k.dark { 0.45 } else { 0.16 }),
            ..Shadow::default()
        })
        .box_shadow(Shadow {
            y: 12.0,
            blur: 28.0,
            color: Color::BLACK.alpha(if p.k.dark { 0.55 } else { 0.18 }),
            ..Shadow::default()
        })
        .child(
            el().key("tile-master")
                .size(34.0, 44.0)
                .radius(7.0)
                .fill(tile_ink)
                .absolute(17.0, 24.0),
        )
        .child(
            el().key("tile-top")
                .size(22.0, 20.5)
                .radius(5.5)
                .fill(tile_ink.alpha(0.88))
                .absolute(53.0, 24.0),
        )
        .child(
            el().key("tile-bottom")
                .size(22.0, 20.5)
                .radius(5.5)
                .fill(tile_ink.alpha(0.66))
                .absolute(53.0, 47.5),
        );
    if !p.k.dark {
        badge = badge.border(p.hair, Color::BLACK.alpha(0.12));
    }
    row().key("hero").w_full().h(132.0).center().child(
        row()
            .key("halo")
            .padding(16.0)
            .radius(38.0)
            .border(p.hair, c.border_base.alpha(0.35))
            .child(badge),
    )
}

fn conflict_banner(p: Paint, conflicts: &[String]) -> Option<El> {
    if conflicts.is_empty() {
        return None;
    }
    let names = match conflicts {
        [only] => format!("{only} is running"),
        [first, second] => format!("{first} and {second} are running"),
        [first, rest @ ..] => format!("{first} and {} others are running", rest.len()),
        [] => unreachable!(),
    };
    let w = &p.k.c.warning;
    Some(
        col()
            .key("conflict-banner")
            .px(14.0)
            .py(11.0)
            .gap(3.0)
            .radius(p.k.radius.lg)
            .fill(w.bg)
            .border(p.hair, w.border)
            .child(
                row()
                    .key("title-row")
                    .gap(8.0)
                    .center_y()
                    .child(controls::status_dot(w.fg))
                    .child(text(names).key("title").font(p.k.ty.ui_medium).color(w.fg)),
            )
            .child(
                text(
                    "Quit other window managers so they don't compete over window positions or Option shortcuts.",
                )
                .key("detail")
                .font(DESCRIPTION)
                .color(p.muted())
                .pl(16.0)
                .wrap(),
            ),
    )
}

fn setup(ui: &Ui, p: Paint, view: &View) -> El {
    let waiting = view.accessibility == controls::PermissionControl::Waiting;
    let access_desc = if view.ready {
        (
            "Moves windows and listens for Option shortcuts.".to_owned(),
            p.muted(),
        )
    } else if waiting {
        (
            "Drag Spaceman into the list, or switch it on in System Settings.".to_owned(),
            p.warning(),
        )
    } else {
        (
            "Needed to move windows and register Option shortcuts.".to_owned(),
            p.muted(),
        )
    };
    let access_kind = if view.ready {
        ButtonKind::Secondary
    } else {
        ButtonKind::Primary
    };
    let access_row = controls::icon_row(
        p,
        "setup-accessibility",
        &ACCESSIBILITY_ICON,
        "Accessibility",
        Some(access_desc),
        controls::permission_control(p, "setup-grant", view.accessibility, access_kind, |ui| {
            input(ui, Input::GrantAccessibility);
        }),
    );

    let login_desc = match ui.agent.as_ref().map(|agent| &agent.launch_at_login) {
        Some(UiLaunchAtLogin::NeedsApproval) => (
            "Approve Spaceman in System Settings › General › Login Items.".to_owned(),
            p.warning(),
        ),
        Some(UiLaunchAtLogin::Failed(error)) => (error.clone(), p.danger()),
        _ => (
            "Start in the menu bar when you sign in.".to_owned(),
            p.muted(),
        ),
    };
    let login_row = controls::icon_toggle_row(
        p,
        "setup-login",
        &glyphs::SPARKLE,
        "Launch at Login",
        Some(login_desc),
        view.launch_at_login,
        |ui| input(ui, Input::ToggleLaunchAtLogin),
    );

    col()
        .key("setup")
        .gap(24.0)
        .child(title_block(p, "Give Spaceman window access", None))
        .child(controls::card(p, "setup-card", vec![access_row, login_row]))
}

fn rules_step(ui: &Ui, p: Paint, starter_apps: &[StarterApp]) -> El {
    let rows = starter_apps
        .iter()
        .enumerate()
        .map(|(index, app)| starter_rule_row(ui, p, index, app))
        .collect();
    col()
        .key("rules-step")
        .gap(20.0)
        .child(title_block(
            p,
            "Choose how apps open",
            Some("Windows tile automatically unless you pin an app to a workspace or float it."),
        ))
        .child(
            col()
                .key("rules-group")
                .gap(8.0)
                .child(controls::card(p, "rules-card", rows))
                .child(
                    text("Drag apps across all 36 workspaces anytime in Settings › Rules.")
                        .key("hint")
                        .font(DESCRIPTION)
                        .color(p.faint())
                        .px(16.0),
                ),
        )
}

fn starter_rule_row(ui: &Ui, p: Paint, index: usize, starter: &StarterApp) -> El {
    let current = ui.doc.app_place(&starter.bundle_id);

    let mut choices: Vec<(String, Option<Place>)> = vec![("Tile".into(), None)];
    for ws in (1..=3).filter_map(|n| WorkspaceId::new(n).ok()) {
        choices.push((ws.to_string(), Some(Place::Workspace(ws))));
    }
    if let Some(Place::Workspace(other)) = current
        && !matches!(other.number(), Some(1..=3))
    {
        choices[3] = (other.to_string(), Some(Place::Workspace(other)));
    }
    choices.push(("Float".into(), Some(Place::Float)));

    let selected = choices
        .iter()
        .position(|(_, place)| match (current, *place) {
            (Some(Place::Ignore), Some(Place::Float)) => true,
            (a, b) => a == b,
        })
        .unwrap_or(0);
    let labels: Vec<String> = choices.iter().map(|(label, _)| label.clone()).collect();
    let places: Vec<Option<Place>> = choices.into_iter().map(|(_, place)| place).collect();
    let bundle_id = starter.bundle_id.clone();

    let picker = SegmentedControl::new(&labels, Some(selected))
        .fit()
        .on_pick(move |ui: &mut Ui, picked, _: &mut Tx| {
            if let Some(&place) = places.get(picked) {
                let id = bundle_id.clone();
                ui.edit(move |doc| doc.set_app_place(&id, place));
            }
        })
        .build(p.k)
        .key("place");

    let cached = ui.apps.cached(&starter.bundle_id);
    let badge = controls::icon_or_initial(p, cached, &starter.name, 24.0);
    controls::badge_row(
        p,
        &format!("starter-{index}"),
        badge,
        &starter.name,
        None,
        picker,
    )
}

const DONE_SHORTCUTS: &[(&str, &[&[&str]])] = &[
    ("Switch workspace", &[&["⌥", "1…0"]]),
    ("Move window to workspace", &[&["⌥", "⇧", "1…0"]]),
    (
        "Focus or move a tile",
        &[&["⌥", "H J K L"], &["⌥", "⇧", "H J K L"]],
    ),
    ("Flip split or fullscreen", &[&["⌥", "/"], &["⌥", "⇧", "F"]]),
];

fn done(p: Paint) -> El {
    let c = &p.k.c;
    let art = row().key("done-art").w_full().h(96.0).center().child(
        row()
            .key("ring")
            .padding(12.0)
            .radius(44.0)
            .border(p.hair, c.border_base.alpha(0.35))
            .child(
                row()
                    .key("badge")
                    .size(64.0, 64.0)
                    .radius(32.0)
                    .center()
                    .fill(c.success.bg)
                    .child(
                        icon(p.k.glyph.check_small, 36.0)
                            .key("check")
                            .color(p.success()),
                    ),
            ),
    );
    let rows = controls::shortcut_rows(p, "done-shortcut", DONE_SHORTCUTS);
    col()
        .key("done")
        .gap(20.0)
        .child(art)
        .child(title_block(
            p,
            "You're all set",
            Some("Spaceman stays in the menu bar, ready for your shortcuts."),
        ))
        .child(controls::card(p, "done-shortcuts", rows))
}

pub fn apply_fixture(ui: &mut Ui, tour: Tour) {
    ui.tour = Some(tour);
}
