//! Non-activating Workspace HUD overlay: a capsule near the bottom of each display showing its
//! row of workspaces, with a marker on the one you are on, when you switch workspaces, move a
//! window or change a layout. Never includes window titles or steals focus.
//!
//! Each display gets its own panel from a fixed pool, at a fixed size, only ordered in and out.
//! The capsule's entrance, exit and content changes are drawn inside it (`pill.rs`), so every
//! one of them can turn around mid-flight.

mod grow;
mod icons;
#[cfg(test)]
mod motion_tests;
mod pill;
mod stage;
#[cfg(test)]
mod tests;

use std::{
    cell::Cell,
    sync::{Arc, Mutex},
    time::Duration,
};

use rune::{
    Align, Cx, DisplayTarget, El, KeyPolicy, NativeDisplayId, OverlayHandle, OverlayInput,
    OverlayOptions, OverlaySpec, ScreenEdge, Tx, row,
};
use spaceman::{
    WorkspaceId,
    app::{UiHudDisplay, UiHudEvent},
    config::HudStyle,
};

pub use pill::{HudView, Icons};
pub(crate) use stage::{HudPreview, SCENES, stage, triggers};

use crate::{apps, controls::Paint};

pub const HOLD_MS: u64 = 1_150;
/// How long the capsule's exit gets before the panel is ordered out. The exit spring is under
/// 1/255 of the way from rest by then, so the cut removes nothing that shows.
pub const LEAVE_MS: u64 = 220;
/// A freshly shown HUD draws its marker where the switch started for this long, then sends it
/// on: long enough for the panel's first frame to have drawn it there.
const FRESH_MS: u64 = 50;
/// Waiting on that first frame, how soon to look again.
const DRAW_RETRY_MS: u64 = 16;
/// While a panel is up, how often it checks its display is still connected.
const DISPLAY_CHECK_MS: u64 = 250;
/// Only the first show fades the panel itself: nothing has been drawn to enter from yet.
const FIRST_FADE: Duration = Duration::from_millis(150);
/// Room on every side of the capsule for its shadow, which the panel's edge would cut, and
/// for ten or so workspaces before the row scrolls.
const OVERLAY_SIZE: (f32, f32) = (860.0, pill::PILL_HEIGHT + 2.0 * pill::SHADOW_REACH);
/// Keeps the capsule 60 pt above the bottom of the display.
const OVERLAY_MARGIN: f32 = 60.0 - pill::SHADOW_REACH;
/// One panel per display, up to this many displays.
const PANEL_NAMES: [&str; 4] = [
    "Spaceman HUD",
    "Spaceman HUD 2",
    "Spaceman HUD 3",
    "Spaceman HUD 4",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HudTransition {
    /// The HUD is off.
    Ignored,
    /// It was hidden: order the panel in.
    Show,
    /// It was up or on its way out: the capsule carries on from where it is.
    Update,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HudTick {
    Idle,
    /// The fresh capsule has drawn: the marker sets off for the visible workspace.
    Arrive,
    /// The hold ran out: the capsule starts to leave.
    Leave,
    /// The exit has played: order the panel out.
    Gone,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Stage {
    Hidden,
    Shown { hide_at_ms: u64 },
    Leaving { gone_at_ms: u64 },
}

/// Pure HUD visibility and deadline state machine for one display's capsule.
#[derive(Clone, Debug)]
pub struct HudFlow {
    enabled: bool,
    stage: Stage,
    current: Option<UiHudDisplay>,
    /// Where a fresh show's marker starts: the workspace the switch left on this display.
    from: Option<WorkspaceId>,
    fresh: bool,
    fresh_until_ms: u64,
    /// The view has been built since the HUD was shown, so its marker has been drawn at `from`.
    drawn: Cell<bool>,
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
            stage: Stage::Hidden,
            current: None,
            from: None,
            fresh: true,
            fresh_until_ms: 0,
            drawn: Cell::new(false),
        }
    }

    /// What the capsule draws now; the last row stays through the exit so it never blanks.
    #[must_use]
    pub fn view(&self, still: bool) -> Option<HudView<'_>> {
        self.current.as_ref().map(|row| HudView {
            row,
            shown: matches!(self.stage, Stage::Shown { .. }),
            fresh: self.fresh,
            focus: if self.fresh {
                self.from.or(row.visible)
            } else {
                row.visible
            },
            still,
        })
    }

    /// The view was built: a fresh marker has been drawn where it starts.
    pub fn mark_drawn(&self) {
        self.drawn.set(true);
    }

    #[must_use]
    pub const fn is_up(&self) -> bool {
        !matches!(self.stage, Stage::Hidden)
    }

    pub fn trigger(&mut self, row: UiHudDisplay, now_ms: u64) -> HudTransition {
        if !self.enabled {
            return HudTransition::Ignored;
        }
        let hidden = self.stage == Stage::Hidden;
        if hidden {
            self.fresh = true;
            self.fresh_until_ms = now_ms.saturating_add(FRESH_MS);
            self.from = row.from;
            self.drawn.set(false);
        } else if self.fresh {
            // Another event before the marker set off: it still starts where the first left.
            self.from = self.from.or(row.from);
        }
        self.stage = Stage::Shown {
            hide_at_ms: now_ms.saturating_add(HOLD_MS),
        };
        self.current = Some(row);
        if hidden {
            HudTransition::Show
        } else {
            HudTransition::Update
        }
    }

    /// Advances the timers to `now_ms`.
    pub fn tick(&mut self, now_ms: u64) -> HudTick {
        if self.fresh && self.is_up() && now_ms >= self.fresh_until_ms && self.drawn.get() {
            self.fresh = false;
            return HudTick::Arrive;
        }
        match self.stage {
            Stage::Shown { hide_at_ms } if now_ms >= hide_at_ms => {
                self.leave(now_ms);
                HudTick::Leave
            }
            Stage::Leaving { gone_at_ms } if now_ms >= gone_at_ms => {
                self.stage = Stage::Hidden;
                HudTick::Gone
            }
            _ => HudTick::Idle,
        }
    }

    /// Until the next deadline, while one is pending.
    #[must_use]
    pub fn remaining_ms(&self, now_ms: u64) -> Option<u64> {
        let until = match self.stage {
            Stage::Hidden => return None,
            Stage::Shown { hide_at_ms: until } | Stage::Leaving { gone_at_ms: until } => until,
        };
        let mut remaining = until.saturating_sub(now_ms);
        if self.fresh {
            let arrive = if now_ms < self.fresh_until_ms {
                self.fresh_until_ms - now_ms
            } else {
                DRAW_RETRY_MS
            };
            remaining = remaining.min(arrive);
        }
        Some(remaining.max(1))
    }

    /// Updates `enabled`; returns `true` if a shown HUD starts to leave now.
    pub fn set_enabled(&mut self, enabled: bool, now_ms: u64) -> bool {
        self.enabled = enabled;
        if !enabled && matches!(self.stage, Stage::Shown { .. }) {
            self.leave(now_ms);
            return true;
        }
        false
    }

    /// Gone at once: its display went away or another event left it out.
    pub fn dismiss(&mut self) {
        self.stage = Stage::Hidden;
    }

    fn leave(&mut self, now_ms: u64) {
        self.stage = Stage::Leaving {
            gone_at_ms: now_ms.saturating_add(LEAVE_MS),
        };
    }
}

struct HudOverlay {
    flow: HudFlow,
    style: HudStyle,
    icons: Icons,
    /// Reduce Motion as of the last event; the panel never takes focus, so Rune doesn't
    /// re-read it for us.
    reduce_motion: bool,
    /// The panel has been shown once, so its view has a drawn state to animate from.
    primed: bool,
    /// The display the panel is on, to hide it if that display goes away.
    display: Option<u32>,
    handle: Option<OverlayHandle<HudOverlay>>,
}

impl HudOverlay {
    fn new(enabled: bool, style: HudStyle) -> Self {
        Self {
            flow: HudFlow::new(enabled),
            style,
            icons: Icons::new(),
            reduce_motion: false,
            primed: false,
            display: None,
            handle: None,
        }
    }
}

struct Panel {
    handle: OverlayHandle<HudOverlay>,
    /// The display the panel is assigned to, by its `UiHudDisplay::id`.
    display: Option<String>,
}

struct Panels {
    panels: Vec<Panel>,
    apps: apps::Catalog,
}

#[derive(Clone)]
pub struct HudDriver {
    panels: Arc<Mutex<Panels>>,
}

pub fn install<S: 'static>(app: &mut rune::App<S>, enabled: bool, style: HudStyle) -> HudDriver {
    let panels = PANEL_NAMES
        .iter()
        .map(|&name| {
            let spec = OverlaySpec {
                name,
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
            // Showing must not settle the view: the capsule enters from the pose it left in.
            .animate_reveal()
            .poll(overlay_poll);
            let handle = app.overlay(spec, HudOverlay::new(enabled, style));
            let inner = handle.clone();
            handle.update(move |overlay, _| {
                overlay.handle = Some(inner);
            });
            Panel {
                handle,
                display: None,
            }
        })
        .collect();
    HudDriver {
        panels: Arc::new(Mutex::new(Panels {
            panels,
            apps: apps::Catalog::default(),
        })),
    }
}

impl HudDriver {
    /// Call on the main thread, where displays, app icons and Reduce Motion are read.
    pub fn trigger(&self, event: UiHudEvent) {
        let reduce_motion = reduce_motion();
        let connected = display_ids();
        let Ok(mut panels) = self.panels.lock() else {
            return;
        };
        let icons = panels.icons_for(&event);
        let mut rows: Vec<(UiHudDisplay, DisplayTarget, Option<u32>)> = event
            .displays
            .iter()
            .filter_map(|row| {
                let id = *connected.get(&row.id)?;
                Some((row.clone(), native(id), Some(id)))
            })
            .take(PANEL_NAMES.len())
            .collect();
        if rows.is_empty()
            && let Some(row) = event.display_of(event.workspace)
        {
            // Displays couldn't be matched up: the acting display's row on the main one.
            rows.push((row.clone(), DisplayTarget::Primary, None));
        }
        panels.assign(&rows);
        for (row, target, display) in rows {
            let Some(panel) = panels
                .panels
                .iter()
                .find(|panel| panel.display.as_deref() == Some(row.id.as_str()))
            else {
                continue;
            };
            let icons = icons.clone();
            panel.handle.update_if(move |overlay, tx| {
                if overlay.display != display || display.is_none() {
                    if let Some(handle) = &overlay.handle {
                        handle.set_display(target);
                    }
                    overlay.display = display;
                }
                let now_ms = spaceman::journal::unix_millis();
                let handle = overlay.handle.clone();
                match overlay.flow.trigger(row, now_ms) {
                    HudTransition::Ignored => return false,
                    HudTransition::Show if overlay.primed => {
                        if let Some(handle) = handle {
                            handle.show();
                        }
                    }
                    HudTransition::Show => {
                        overlay.primed = true;
                        if let Some(handle) = handle {
                            handle.fade_in(FIRST_FADE);
                        }
                    }
                    HudTransition::Update => {}
                }
                overlay.icons.extend(icons);
                overlay.reduce_motion = reduce_motion;
                if let Some(remaining) = overlay.flow.remaining_ms(now_ms) {
                    tx.wake_after(Duration::from_millis(remaining));
                }
                true
            });
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.each(move |overlay, tx| {
            let left = overlay
                .flow
                .set_enabled(enabled, spaceman::journal::unix_millis());
            if left {
                tx.wake_after(Duration::from_millis(LEAVE_MS));
            }
            left
        });
    }

    pub fn set_style(&self, style: HudStyle) {
        self.each(move |overlay, _| {
            let changed = overlay.style != style;
            overlay.style = style;
            changed
        });
    }

    fn each(&self, f: impl Fn(&mut HudOverlay, &mut Tx) -> bool + Clone + Send + 'static) {
        let Ok(panels) = self.panels.lock() else {
            return;
        };
        for panel in &panels.panels {
            let f = f.clone();
            panel.handle.update_if(move |overlay, tx| f(overlay, tx));
        }
    }
}

impl Panels {
    /// Every app in `event` that has an icon here, looked up once each.
    fn icons_for(&mut self, event: &UiHudEvent) -> Icons {
        let mut icons = Icons::new();
        for workspace in event.displays.iter().flat_map(|row| &row.workspaces) {
            let tiles = workspace
                .tiles
                .iter()
                .filter_map(|tile| tile.bundle.as_ref());
            for bundle in tiles.chain(&workspace.floating) {
                if !icons.contains_key(bundle)
                    && let Some(pixels) = self.apps.get(bundle).icon
                {
                    icons.insert(bundle.clone(), pixels);
                }
            }
        }
        icons
    }

    /// Gives each display in `rows` a panel, and hides the panels of displays it leaves out.
    fn assign(&mut self, rows: &[(UiHudDisplay, DisplayTarget, Option<u32>)]) {
        let current: Vec<Option<String>> = self
            .panels
            .iter()
            .map(|panel| panel.display.clone())
            .collect();
        let wanted: Vec<&str> = rows.iter().map(|(row, _, _)| row.id.as_str()).collect();
        for (panel, display) in self.panels.iter_mut().zip(assign_panels(&current, &wanted)) {
            if panel.display.is_some() && panel.display != display {
                panel.handle.update_if(|overlay, _| {
                    overlay.flow.dismiss();
                    if let Some(handle) = &overlay.handle {
                        handle.hide();
                    }
                    false
                });
            }
            panel.display = display;
        }
    }
}

/// Which display each panel shows: every display it already showed and `wanted` still lists
/// stays put, the rest are freed, and new displays take free panels in order. Displays past
/// the pool get none.
#[must_use]
pub fn assign_panels(current: &[Option<String>], wanted: &[&str]) -> Vec<Option<String>> {
    let mut panels: Vec<Option<String>> = current
        .iter()
        .map(|display| display.clone().filter(|id| wanted.contains(&id.as_str())))
        .collect();
    for id in wanted {
        if panels.iter().any(|display| display.as_deref() == Some(*id)) {
            continue;
        }
        if let Some(free) = panels.iter_mut().find(|display| display.is_none()) {
            *free = Some((*id).to_owned());
        }
    }
    panels
}

const fn native(id: u32) -> DisplayTarget {
    DisplayTarget::Native(NativeDisplayId::MacOS(id))
}

#[cfg(target_os = "macos")]
fn display_ids() -> std::collections::HashMap<String, u32> {
    spaceman::macos::display_ids()
}

#[cfg(not(target_os = "macos"))]
fn display_ids() -> std::collections::HashMap<String, u32> {
    std::collections::HashMap::new()
}

#[cfg(target_os = "macos")]
fn reduce_motion() -> bool {
    objc2_app_kit::NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion()
}

#[cfg(not(target_os = "macos"))]
fn reduce_motion() -> bool {
    false
}

fn overlay_poll(overlay: &mut HudOverlay, _clock: f64, tx: &mut Tx) -> bool {
    let now_ms = spaceman::journal::unix_millis();
    if overlay.flow.is_up()
        && let Some(display) = overlay.display
        && !display_ids().values().any(|id| *id == display)
    {
        // Its display went away: Rune would move the panel to the main display, over that
        // display's own capsule.
        overlay.flow.dismiss();
        if let Some(handle) = &overlay.handle {
            handle.hide();
        }
        return false;
    }
    let tick = overlay.flow.tick(now_ms);
    if tick == HudTick::Gone
        && let Some(handle) = &overlay.handle
    {
        handle.hide();
    }
    if let Some(remaining_ms) = overlay.flow.remaining_ms(now_ms) {
        let check = if overlay.display.is_some() {
            DISPLAY_CHECK_MS
        } else {
            u64::MAX
        };
        tx.wake_after(Duration::from_millis(remaining_ms.min(check)));
    }
    matches!(tick, HudTick::Arrive | HudTick::Leave)
}

fn overlay_view(overlay: &HudOverlay, cx: &mut Cx<HudOverlay>) -> El {
    if overlay.reduce_motion {
        cx.reduced_motion(true);
    }
    let p = Paint::new(true, cx.env().scale);
    overlay.flow.mark_drawn();
    row().key("hud-root").w_full().h_full().center().children(
        overlay
            .flow
            .view(cx.is_reduced_motion())
            .map(|view| pill::slot(&view, overlay.style, &overlay.icons, OVERLAY_SIZE.0, p)),
    )
}
