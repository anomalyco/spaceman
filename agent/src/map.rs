//! Rules as a workspace map: a tile per workspace showing the apps that open there, and the
//! apps that float below. Dragging an app to another tile, or to the float box, rewrites its
//! rule. Everything sits at computed positions, so a drag knows where the pointer is from the
//! dragged icon's own spot.

use rune::{Cursor, El, PointerPress, Tx, col, el, row, text};
use rune_kit::{Menu as KitMenu, MenuItem, Placement};
use spaceman::WorkspaceId;

use crate::{
    Menu, Ui,
    apps::{self, App},
    controls::{self, Paint},
    rules::{Doc, Place, Rule},
};

const COLS: usize = 5;
const TILE_W: f32 = 108.0;
const TILE_H: f32 = 100.0;
const GAP: f32 = 10.0;
pub const MAP_W: f32 = COLS as f32 * TILE_W + (COLS - 1) as f32 * GAP;
const HEAD_H: f32 = 20.0;
const HEAD_GAP: f32 = 10.0;
const GRID_TOP: f32 = HEAD_H + HEAD_GAP;
const SECTION_GAP: f32 = 24.0;
/// Inside a tile: where its icons start, their size and spacing, and how many fit.
const TILE_PAD: f32 = 12.0;
const ICONS_TOP: f32 = 38.0;
const ICON: f32 = 24.0;
const ICON_GAP: f32 = 6.0;
const ICONS_PER_ROW: usize = 3;
const ICONS_SHOWN: usize = 6;
/// The float box and its chips.
const BOX_PAD: f32 = 10.0;
const CHIP_W: f32 = 128.0;
const CHIP_H: f32 = 32.0;
const CHIP_GAP: f32 = 8.0;
const CHIP_ICON: f32 = 20.0;
const CHIPS_PER_ROW: usize = 4;
/// Pointer travel before a press on an app becomes a drag.
const SLOP: f32 = 3.0;

/// An app being dragged: its rule, where it started and how far the pointer has gone.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drag {
    pub rule: usize,
    /// The dragged item's top-left in the map, and its size.
    origin: (f32, f32),
    size: (f32, f32),
    /// Where in the item the press landed.
    grab: (f32, f32),
    delta: (f32, f32),
    moved: bool,
    target: Option<Place>,
}

impl Drag {
    /// Slack mid-drag over workspace 4, for stills.
    pub fn fixture(doc: &Doc) -> Option<Drag> {
        let layout = Layout::new(doc);
        let rule = doc
            .rules()
            .into_iter()
            .find(|rule| rule.bundle_id.as_deref() == Some("com.tinyspeck.slackmacgap"))?;
        let (origin, size) = layout.spot(&rule)?;
        let grab = (size.0 / 2.0, size.1 / 2.0);
        let to = layout.tile_origin(3);
        let delta = (
            to.0 + TILE_W / 2.0 - origin.0 - grab.0,
            to.1 + TILE_H / 2.0 - origin.1 - grab.1,
        );
        Some(Drag {
            rule: rule.index,
            origin,
            size,
            grab,
            delta,
            moved: true,
            target: layout.target_at(to.0 + TILE_W / 2.0, to.1 + TILE_H / 2.0),
        })
    }
}

/// Where everything in the map goes, from the rules.
struct Layout {
    tiles: Vec<WorkspaceId>,
    /// Rules by tile, in file order.
    routed: Vec<Vec<Rule>>,
    floating: Vec<Rule>,
}

impl Layout {
    fn new(doc: &Doc) -> Layout {
        let rules = doc.rules();
        let mut tiles: Vec<WorkspaceId> =
            (1..=9).filter_map(|n| WorkspaceId::new(n).ok()).collect();
        let mut extra: Vec<WorkspaceId> = rules
            .iter()
            .filter_map(|rule| match rule.place {
                Place::Workspace(workspace) => Some(workspace),
                _ => None,
            })
            .chain(WorkspaceId::all().filter(|workspace| doc.monitor(*workspace).is_some()))
            .filter(|workspace| !tiles.contains(workspace))
            .collect();
        extra.sort();
        extra.dedup();
        tiles.extend(extra);
        let mut routed = vec![Vec::new(); tiles.len()];
        let mut floating = Vec::new();
        for rule in rules {
            match rule.place {
                Place::Workspace(workspace) => {
                    if let Some(index) = tiles.iter().position(|tile| *tile == workspace) {
                        routed[index].push(rule);
                    }
                }
                Place::Float | Place::Ignore => floating.push(rule),
            }
        }
        Layout {
            tiles,
            routed,
            floating,
        }
    }

    fn grid_rows(&self) -> usize {
        self.tiles.len().div_ceil(COLS)
    }

    fn grid_h(&self) -> f32 {
        let rows = self.grid_rows() as f32;
        rows * TILE_H + (rows - 1.0).max(0.0) * GAP
    }

    fn float_head_top(&self) -> f32 {
        GRID_TOP + self.grid_h() + SECTION_GAP
    }

    fn box_top(&self) -> f32 {
        self.float_head_top() + HEAD_H + HEAD_GAP
    }

    /// The float box holds its rules and the Add chip.
    fn box_h(&self) -> f32 {
        let rows = (self.floating.len() + 1).div_ceil(CHIPS_PER_ROW) as f32;
        2.0 * BOX_PAD + rows * CHIP_H + (rows - 1.0) * CHIP_GAP
    }

    fn height(&self) -> f32 {
        self.box_top() + self.box_h()
    }

    fn tile_origin(&self, index: usize) -> (f32, f32) {
        let (col, row) = (index % COLS, index / COLS);
        (
            col as f32 * (TILE_W + GAP),
            GRID_TOP + row as f32 * (TILE_H + GAP),
        )
    }

    /// The `slot`th icon in a tile, from the tile's top-left.
    fn icon_offset(slot: usize) -> (f32, f32) {
        let (col, row) = (slot % ICONS_PER_ROW, slot / ICONS_PER_ROW);
        (
            TILE_PAD + col as f32 * (ICON + ICON_GAP),
            ICONS_TOP + row as f32 * (ICON + ICON_GAP),
        )
    }

    fn chip_origin(&self, slot: usize) -> (f32, f32) {
        let (col, row) = (slot % CHIPS_PER_ROW, slot / CHIPS_PER_ROW);
        (
            BOX_PAD + col as f32 * (CHIP_W + CHIP_GAP),
            self.box_top() + BOX_PAD + row as f32 * (CHIP_H + CHIP_GAP),
        )
    }

    /// A rule's item: its top-left in the map and its size.
    fn spot(&self, rule: &Rule) -> Option<((f32, f32), (f32, f32))> {
        if let Some(slot) = self.floating.iter().position(|r| r.index == rule.index) {
            return Some((self.chip_origin(slot), (CHIP_W, CHIP_H)));
        }
        self.routed.iter().enumerate().find_map(|(tile, rules)| {
            let slot = rules.iter().position(|r| r.index == rule.index)?;
            let (x, y) = self.tile_origin(tile);
            let (dx, dy) = Self::icon_offset(slot);
            Some(((x + dx, y + dy), (ICON, ICON)))
        })
    }

    /// The place under a point in the map.
    fn target_at(&self, x: f32, y: f32) -> Option<Place> {
        if (0.0..=MAP_W).contains(&x)
            && (GRID_TOP - GAP..=GRID_TOP + self.grid_h() + GAP).contains(&y)
        {
            let col = ((x + GAP / 2.0) / (TILE_W + GAP))
                .floor()
                .clamp(0.0, (COLS - 1) as f32);
            let row = ((y - GRID_TOP + GAP / 2.0) / (TILE_H + GAP))
                .floor()
                .clamp(0.0, (self.grid_rows() - 1) as f32);
            let index = row as usize * COLS + col as usize;
            return self.tiles.get(index).map(|tile| Place::Workspace(*tile));
        }
        let top = self.float_head_top();
        ((0.0..=MAP_W).contains(&x) && (top..=self.box_top() + self.box_h() + GAP).contains(&y))
            .then_some(Place::Float)
    }
}

pub fn pane(ui: &Ui, p: Paint, layer: &mut Vec<El>) -> El {
    let layout = Layout::new(&ui.doc);
    let active: Vec<String> = ui
        .agent
        .clone()
        .flatten()
        .map(|agent| agent.active)
        .unwrap_or_default();
    let target = ui
        .drag
        .filter(|drag| drag.moved)
        .and_then(|drag| drag.target);
    let mut map = el().key("map").w(MAP_W).h(layout.height()).child(
        controls::heading(
            p,
            "where",
            "Where apps open",
            Some("Drag an app to move it"),
        )
        .w(MAP_W)
        .px(0.0)
        .absolute(0.0, 0.0),
    );
    for (index, workspace) in layout.tiles.iter().copied().enumerate() {
        let (x, y) = layout.tile_origin(index);
        let is_active = active.contains(&workspace.to_string());
        let is_target = target == Some(Place::Workspace(workspace));
        map = map.child(
            tile(
                ui, p, &layout, index, workspace, is_active, is_target, layer,
            )
            .absolute(x, y),
        );
    }
    map = map
        .child(
            controls::heading(p, "float-heading", "Float, never tiled", None)
                .w(MAP_W)
                .px(0.0)
                .absolute(0.0, layout.float_head_top()),
        )
        .child(float_box(
            ui,
            p,
            &layout,
            target == Some(Place::Float),
            layer,
        ));
    if let Some(drag) = ui.drag.filter(|drag| drag.moved) {
        map = map.children(ghost(ui, p, &layout, drag));
    }
    col()
        .key("rules-body")
        .grow(1.0)
        .px(crate::view::CONTENT_PADDING)
        .pt(4.0)
        .child(map)
}

#[allow(clippy::too_many_arguments)]
fn tile(
    ui: &Ui,
    p: Paint,
    layout: &Layout,
    index: usize,
    workspace: WorkspaceId,
    active: bool,
    target: bool,
    layer: &mut Vec<El>,
) -> El {
    let c = &p.k.c;
    let rules = &layout.routed[index];
    let border = if target {
        (1.5, c.border_focus)
    } else if active {
        (1.5, c.border_strong)
    } else {
        (p.hair, c.border_weak)
    };
    let mut tile = el()
        .key(format!("tile-{workspace}"))
        .role("group")
        .label(format!("Workspace {workspace}"))
        .size(TILE_W, TILE_H)
        .radius(p.k.radius.lg)
        .fill(if target { c.layer_02 } else { c.layer_01 })
        .border(border.0, border.1)
        .hover_group()
        .child(
            text(workspace.to_string())
                .key("number")
                .font(p.k.ty.ui_medium)
                .color(if active { p.text() } else { p.muted() })
                .absolute(TILE_PAD, 10.0),
        )
        .child(add_button(ui, p, Place::Workspace(workspace), layer).absolute(TILE_W - 30.0, 8.0));
    let overflow = rules.len() > ICONS_SHOWN;
    for (slot, rule) in rules.iter().enumerate() {
        let (x, y) = Layout::icon_offset(slot);
        if overflow && slot == ICONS_SHOWN - 1 {
            tile = tile.child(
                el().key("more")
                    .size(ICON, ICON)
                    .radius(6.0)
                    .fill(c.layer_03)
                    .center()
                    .child(
                        text(format!("+{}", rules.len() - slot))
                            .font(p.k.ty.small_medium)
                            .color(p.muted()),
                    )
                    .absolute(x, y),
            );
            break;
        }
        let item = draggable(ui, p, layout, rule, app_icon(ui, p, rule, ICON), layer);
        tile = tile.child(item.absolute(x, y));
    }
    tile
}

/// The float box: a chip per floating app, then Add.
fn float_box(ui: &Ui, p: Paint, layout: &Layout, target: bool, layer: &mut Vec<El>) -> El {
    let c = &p.k.c;
    let top = layout.box_top();
    let mut area = el()
        .key("float-box")
        .w(MAP_W)
        .h(layout.box_h())
        .radius(p.k.radius.lg)
        .fill(if target { c.layer_02 } else { c.layer_01 })
        .border(
            if target { 1.5 } else { p.hair },
            if target {
                c.border_focus
            } else {
                c.border_weak
            },
        )
        .absolute(0.0, top);
    for (slot, rule) in layout.floating.iter().enumerate() {
        let (x, y) = layout.chip_origin(slot);
        let chip = chip(ui, p, rule);
        area = area.child(draggable(ui, p, layout, rule, chip, layer).absolute(x, y - top));
    }
    let (x, y) = layout.chip_origin(layout.floating.len());
    area.child(add_chip(ui, p, layer).absolute(x, y - top))
}

fn chip(ui: &Ui, p: Paint, rule: &Rule) -> El {
    let c = &p.k.c;
    let name = rule_name(ui, rule);
    row()
        .size(CHIP_W, CHIP_H)
        .px(6.0)
        .gap(8.0)
        .center_y()
        .radius(p.k.radius.md)
        .fill(c.layer_02)
        .child(app_icon(ui, p, rule, CHIP_ICON))
        .child(
            text(if rule.place == Place::Ignore {
                format!("{name} · ignored")
            } else {
                name
            })
            .key("name")
            .font(p.k.ty.ui)
            .color(p.text())
            .truncate()
            .shrink(),
        )
}

fn add_chip(ui: &Ui, p: Paint, layer: &mut Vec<El>) -> El {
    let c = &p.k.c;
    let open = ui.menu == Some(Menu::Add(Place::Float));
    let trigger = row()
        .key("add-float")
        .role("button")
        .label("Add app")
        .size(CHIP_W, CHIP_H)
        .px(10.0)
        .gap(6.0)
        .center_y()
        .radius(p.k.radius.md)
        .border(p.hair, c.border_base)
        .hover(c.hover)
        .press(c.pressed)
        .focusable(true)
        .focus_ring(2.0, -2.0, c.border_focus)
        .on_press(|ui: &mut Ui| ui.open_add_menu(Place::Float))
        .child(rune::icon(&rune_theme_opencode::icons::PLUS, 14.0).color(c.icon_muted))
        .child(text("Add app").font(p.k.ty.ui).color(p.muted()));
    add_menu(ui, p, "add-float-menu", trigger, open, Place::Float, layer)
}

/// A tile's Add, shown while the pointer is over the tile or its menu is open.
fn add_button(ui: &Ui, p: Paint, place: Place, layer: &mut Vec<El>) -> El {
    let c = &p.k.c;
    let open = ui.menu == Some(Menu::Add(place));
    let trigger = el()
        .key("add")
        .role("button")
        .label("Add an app here")
        .size(22.0, 22.0)
        .center()
        .radius(p.k.radius.sm)
        .hover(c.hover)
        .press(c.pressed)
        .on_press(move |ui: &mut Ui| ui.open_add_menu(place))
        .child(rune::icon(&rune_theme_opencode::icons::PLUS, 14.0).color(c.icon_muted));
    let trigger = if open || ui.drag.is_some() {
        trigger.opacity(if open { 1.0 } else { 0.0 })
    } else {
        trigger.group_opacity(0.0, 1.0)
    };
    let key = match place {
        Place::Workspace(workspace) => format!("add-{workspace}-menu"),
        _ => "add-menu".into(),
    };
    add_menu(ui, p, &key, trigger, open, place, layer)
}

fn add_menu(
    ui: &Ui,
    p: Paint,
    key: &str,
    trigger: El,
    open: bool,
    place: Place,
    layer: &mut Vec<El>,
) -> El {
    let mut menu = KitMenu::new(key)
        .trigger(trigger)
        .placement(Placement::BottomStart)
        .gutter(4.0)
        .min_width(220.0)
        .open(open)
        .on_close(|ui: &mut Ui| ui.menu = None);
    if open {
        menu = menu.group_label("Running apps");
        if ui.running.is_empty() {
            menu = menu.item(MenuItem::new("Every running app has a rule").disabled(true));
        }
        for app in &ui.running {
            let bundle_id = app.bundle_id.clone();
            let lead = controls::icon_or_initial(p, ui.apps.cached(&bundle_id), &app.name, 16.0);
            menu = menu.item(MenuItem::new(app.name.clone()).lead(lead).on_select(
                move |ui: &mut Ui| {
                    let bundle_id = bundle_id.clone();
                    ui.menu = None;
                    ui.edit(move |doc| doc.add(&bundle_id, place));
                },
            ));
        }
    }
    menu.build(p.k, layer)
}

/// `item` for `rule`, dragged to move it and right-clicked for its menu.
fn draggable(ui: &Ui, p: Paint, layout: &Layout, rule: &Rule, item: El, layer: &mut Vec<El>) -> El {
    let index = rule.index;
    let Some((origin, size)) = layout.spot(rule) else {
        return item;
    };
    let dragging = ui.drag.is_some_and(|drag| drag.rule == index && drag.moved);
    let tiles = layout.tiles.clone();
    let floating: Vec<usize> = layout.floating.iter().map(|r| r.index).collect();
    let routed: Vec<Vec<usize>> = layout
        .routed
        .iter()
        .map(|rules| rules.iter().map(|r| r.index).collect())
        .collect();
    let item = item
        .key(format!("rule-{index}"))
        .role("button")
        .label(rule_name(ui, rule))
        .help(rule_help(ui, rule))
        .cursor(Cursor::Grab)
        .opacity(if dragging { 0.3 } else { 1.0 })
        .on_drag(move |ui: &mut Ui, d: &mut rune::Drag, _: &mut Tx| {
            drag(ui, d, index, origin, size, |x, y| {
                let layout = Layout::from_parts(&tiles, &routed, &floating);
                layout.target_at(x, y)
            })
        })
        .on_secondary_press(move |ui: &mut Ui, press: PointerPress, _: &mut Tx| {
            ui.menu = Some(Menu::Rule(index, press));
        });
    let point = match ui.menu {
        Some(Menu::Rule(open, point)) if open == index => Some(point),
        _ => None,
    };
    let mut menu = rune_kit::context_menu(format!("rule-{index}-menu"), item, point)
        .on_close(|ui: &mut Ui| ui.menu = None);
    if point.is_some() {
        if rule.place != Place::Float {
            menu = menu.item(
                MenuItem::new("Float, never tiled").on_select(move |ui: &mut Ui| {
                    ui.menu = None;
                    ui.edit(|doc| doc.place(index, Place::Float));
                }),
            );
        }
        menu = menu.item(MenuItem::new("Remove rule").on_select(move |ui: &mut Ui| {
            ui.menu = None;
            ui.edit(|doc| doc.remove(index));
        }));
    }
    menu.build(p.k, layer)
}

impl Layout {
    /// A layout rebuilt from indices alone, for drag handlers, which can't hold rules.
    fn from_parts(tiles: &[WorkspaceId], routed: &[Vec<usize>], floating: &[usize]) -> Layout {
        let rule = |index: usize| Rule {
            index,
            bundle_id: None,
            title: None,
            place: Place::Float,
        };
        Layout {
            tiles: tiles.to_vec(),
            routed: routed
                .iter()
                .map(|rules| rules.iter().copied().map(rule).collect())
                .collect(),
            floating: floating.iter().copied().map(rule).collect(),
        }
    }
}

fn drag(
    ui: &mut Ui,
    d: &rune::Drag,
    rule: usize,
    origin: (f32, f32),
    size: (f32, f32),
    target_at: impl Fn(f32, f32) -> Option<Place>,
) {
    match d.phase {
        rune::Phase::Down => {
            ui.menu = None;
            ui.drag = Some(Drag {
                rule,
                origin,
                size,
                grab: (d.start_x, d.start_y),
                delta: (0.0, 0.0),
                moved: false,
                target: None,
            });
        }
        rune::Phase::Move => {
            let Some(drag) = ui.drag.as_mut().filter(|drag| drag.rule == rule) else {
                return;
            };
            drag.delta = (d.dx, d.dy);
            drag.moved |= d.dx.abs().max(d.dy.abs()) >= SLOP;
            drag.target = target_at(origin.0 + drag.grab.0 + d.dx, origin.1 + drag.grab.1 + d.dy);
        }
        rune::Phase::Up => {
            let Some(drag) = ui.drag.take().filter(|drag| drag.rule == rule) else {
                return;
            };
            if d.cancelled() || !drag.moved {
                return;
            }
            let current = ui
                .doc
                .rules()
                .into_iter()
                .find(|r| r.index == rule)
                .map(|r| r.place);
            let Some(target) = drag.target else {
                return;
            };
            let unchanged = current == Some(target)
                || (target == Place::Float && current == Some(Place::Ignore));
            if !unchanged {
                ui.edit(|doc| doc.place(rule, target));
            }
        }
    }
}

/// The dragged item, following the pointer above everything else in the map.
fn ghost(ui: &Ui, p: Paint, layout: &Layout, drag: Drag) -> Option<El> {
    let rule = layout
        .routed
        .iter()
        .flatten()
        .chain(&layout.floating)
        .find(|rule| rule.index == drag.rule)?;
    let item = if drag.size == (CHIP_W, CHIP_H) {
        chip(ui, p, rule)
    } else {
        app_icon(ui, p, rule, ICON)
    };
    let mut ghost = item
        .key("ghost")
        .cursor(Cursor::Grabbing)
        .absolute(drag.origin.0 + drag.delta.0, drag.origin.1 + drag.delta.1);
    for shadow in &p.k.elevation.floating {
        ghost = ghost.box_shadow(*shadow);
    }
    Some(ghost)
}

fn rule_app<'a>(ui: &'a Ui, rule: &Rule) -> Option<&'a App> {
    ui.apps.cached(rule.bundle_id.as_deref()?)
}

fn rule_name(ui: &Ui, rule: &Rule) -> String {
    match (rule_app(ui, rule), &rule.bundle_id, &rule.title) {
        (Some(app), _, _) => app.name.clone(),
        (None, Some(bundle_id), _) => apps::fallback_name(bundle_id),
        (None, None, Some(title)) => title.clone(),
        (None, None, None) => "Any app".into(),
    }
}

fn rule_help(ui: &Ui, rule: &Rule) -> String {
    let name = rule_name(ui, rule);
    match &rule.title {
        Some(title) => format!("{name}, windows titled “{title}”"),
        None => name,
    }
}

fn app_icon(ui: &Ui, p: Paint, rule: &Rule, size: f32) -> El {
    let icon = controls::icon_or_initial(p, rule_app(ui, rule), &rule_name(ui, rule), size);
    if rule.title.is_none() {
        return icon;
    }
    // A rule for some of an app's windows wears a dot.
    let c = &p.k.c;
    icon.child(
        el().key("title-dot")
            .size(8.0, 8.0)
            .radius(4.0)
            .fill(c.icon_accent)
            .border(1.5, c.layer_01)
            .absolute(size - 6.0, -2.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_tests_tiles_and_the_float_box() {
        let doc = Doc::parse(include_str!("../fixtures/config.toml"));
        let layout = Layout::new(&doc);
        assert_eq!(layout.tiles.len(), 10, "1–9 and M");
        let ws = |name| Some(Place::Workspace(WorkspaceId::parse(name).unwrap()));
        assert_eq!(layout.target_at(5.0, GRID_TOP + 5.0), ws("1"));
        let (x, y) = layout.tile_origin(3);
        assert_eq!(layout.target_at(x + 50.0, y + 50.0), ws("4"));
        let (x, y) = layout.tile_origin(9);
        assert_eq!(layout.target_at(x + 50.0, y + 50.0), ws("M"));
        assert_eq!(
            layout.target_at(20.0, layout.box_top() + 10.0),
            Some(Place::Float)
        );
        assert_eq!(layout.target_at(20.0, layout.height() + 100.0), None);
    }

    #[test]
    fn every_rule_has_a_spot() {
        let doc = Doc::parse(include_str!("../fixtures/config.toml"));
        let layout = Layout::new(&doc);
        for rule in doc.rules() {
            assert!(layout.spot(&rule).is_some(), "{rule:?}");
        }
    }
}
