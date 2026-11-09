//! Non-activating Workspace HUD overlay: a compact capsule near the bottom of
//! the primary display showing workspace switches, cross-workspace window
//! moves, and layout toggles with a miniature live split/fullscreen diagram.
//! Never includes window titles or steals focus.
//!
//! The panel is a fixed size and only ordered in and out. The capsule's entrance, exit and
//! content changes are drawn inside it (`pill.rs`), so every one of them can turn around
//! mid-flight.

#[cfg(test)]
mod motion_tests;
mod pill;
mod stage;
#[cfg(test)]
mod tests;

use std::time::Duration;

use rune::{
    Align, Cx, El, KeyPolicy, OverlayHandle, OverlayInput, OverlayOptions, OverlaySpec, ScreenEdge,
    Tx, row,
};
use spaceman::{WorkspaceId, app::UiHudEvent};

pub use pill::HudView;
pub(crate) use stage::{HudPreview, SCENES, stage, triggers};

use crate::controls::Paint;

pub const HOLD_MS: u64 = 1_150;
/// How long the capsule's exit gets before the panel is ordered out. The exit spring is under
/// 1/255 of the way from rest by then, so the cut removes nothing that shows.
pub const LEAVE_MS: u64 = 220;
/// Events this close behind the one that showed the HUD arrive before it has drawn, so they
/// land at rest with it instead of rolling in from what the HUD showed last time.
const FRESH_MS: u64 = 50;
/// Only the first show fades the panel itself: nothing has been drawn to enter from yet.
const FIRST_FADE: Duration = Duration::from_millis(150);
/// Room on every side of the capsule for its shadow, which the panel's edge would cut.
const OVERLAY_SIZE: (f32, f32) = (460.0, pill::PILL_HEIGHT + 2.0 * pill::SHADOW_REACH);
/// Keeps the capsule 60 pt above the bottom of the display.
const OVERLAY_MARGIN: f32 = 60.0 - pill::SHADOW_REACH;

/// Which way a switch went among the workspaces, in their order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Travel {
    /// The same workspace, or the HUD was hidden.
    Stay,
    Forward,
    Back,
}

impl Travel {
    fn between(from: WorkspaceId, to: WorkspaceId) -> Self {
        match to.cmp(&from) {
            std::cmp::Ordering::Greater => Self::Forward,
            std::cmp::Ordering::Less => Self::Back,
            std::cmp::Ordering::Equal => Self::Stay,
        }
    }
}

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

/// Pure HUD visibility and deadline state machine.
#[derive(Clone, Debug)]
pub struct HudFlow {
    enabled: bool,
    stage: Stage,
    current: Option<UiHudEvent>,
    travel: Travel,
    fresh: bool,
    fresh_until_ms: u64,
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
            travel: Travel::Stay,
            fresh: true,
            fresh_until_ms: 0,
        }
    }

    /// What the capsule draws now; the last event stays through the exit so it never blanks.
    #[must_use]
    pub fn view(&self, scope: &'static str, still: bool) -> Option<HudView<'_>> {
        self.current.as_ref().map(|event| HudView {
            event,
            shown: matches!(self.stage, Stage::Shown { .. }),
            fresh: self.fresh,
            travel: self.travel,
            still,
            scope,
        })
    }

    pub fn trigger(&mut self, event: UiHudEvent, now_ms: u64) -> HudTransition {
        if !self.enabled {
            return HudTransition::Ignored;
        }
        let hidden = self.stage == Stage::Hidden;
        if hidden {
            self.fresh_until_ms = now_ms.saturating_add(FRESH_MS);
        }
        self.fresh = hidden || now_ms < self.fresh_until_ms;
        self.travel = match &self.current {
            Some(previous) if !self.fresh => Travel::between(previous.workspace, event.workspace),
            _ => Travel::Stay,
        };
        self.stage = Stage::Shown {
            hide_at_ms: now_ms.saturating_add(HOLD_MS),
        };
        self.current = Some(event);
        if hidden {
            HudTransition::Show
        } else {
            HudTransition::Update
        }
    }

    /// Advances the timers to `now_ms`.
    pub fn tick(&mut self, now_ms: u64) -> HudTick {
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
        match self.stage {
            Stage::Hidden => None,
            Stage::Shown { hide_at_ms: until } | Stage::Leaving { gone_at_ms: until } => {
                Some(until.saturating_sub(now_ms).max(1))
            }
        }
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

    fn leave(&mut self, now_ms: u64) {
        self.stage = Stage::Leaving {
            gone_at_ms: now_ms.saturating_add(LEAVE_MS),
        };
    }
}

struct HudOverlay {
    flow: HudFlow,
    /// Reduce Motion as of the last event; the panel never takes focus, so Rune doesn't
    /// re-read it for us.
    reduce_motion: bool,
    /// The panel has been shown once, so its view has a drawn state to animate from.
    primed: bool,
    handle: Option<OverlayHandle<HudOverlay>>,
}

impl HudOverlay {
    const fn new(enabled: bool) -> Self {
        Self {
            flow: HudFlow::new(enabled),
            reduce_motion: false,
            primed: false,
            handle: None,
        }
    }
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
    // Showing must not settle the view: the capsule enters from the pose it left in.
    .animate_reveal()
    .poll(overlay_poll);

    let handle = app.overlay(spec, HudOverlay::new(enabled));
    let inner = handle.clone();
    handle.update(move |overlay, _| {
        overlay.handle = Some(inner);
    });
    HudDriver { handle }
}

impl HudDriver {
    /// Call on the main thread, where the Reduce Motion setting is read.
    pub fn trigger(&self, event: UiHudEvent) {
        let handle = self.handle.clone();
        let reduce_motion = reduce_motion();
        self.handle.update_if(move |overlay, tx| {
            let now_ms = spaceman::journal::unix_millis();
            match overlay.flow.trigger(event, now_ms) {
                HudTransition::Ignored => return false,
                HudTransition::Show if overlay.primed => handle.show(),
                HudTransition::Show => {
                    overlay.primed = true;
                    handle.fade_in(FIRST_FADE);
                }
                HudTransition::Update => {}
            }
            overlay.reduce_motion = reduce_motion;
            tx.wake_after(Duration::from_millis(HOLD_MS));
            true
        });
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.handle.update_if(move |overlay, tx| {
            let left = overlay
                .flow
                .set_enabled(enabled, spaceman::journal::unix_millis());
            if left {
                tx.wake_after(Duration::from_millis(LEAVE_MS));
            }
            left
        });
    }
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
    let tick = overlay.flow.tick(now_ms);
    if tick == HudTick::Gone
        && let Some(handle) = &overlay.handle
    {
        handle.hide();
    }
    if let Some(remaining_ms) = overlay.flow.remaining_ms(now_ms) {
        tx.wake_after(Duration::from_millis(remaining_ms));
    }
    tick == HudTick::Leave
}

fn overlay_view(overlay: &HudOverlay, cx: &mut Cx<HudOverlay>) -> El {
    if overlay.reduce_motion {
        cx.reduced_motion(true);
    }
    let p = Paint::new(true, cx.env().scale);
    row().key("hud-root").w_full().h_full().center().children(
        overlay
            .flow
            .view("hud", cx.is_reduced_motion())
            .map(|view| pill::slot(&view, p)),
    )
}
