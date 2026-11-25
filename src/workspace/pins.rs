//! Pinned floats: floating windows shown on every workspace of their display.
//!
//! A pinned window keeps its owning workspace (for focus, memory and the HUD) but is never
//! parked by a workspace switch, so it stays where it is while other workspaces come and go.
//! A pin comes from a `pin` rule (recomputed on every observation) or from `spaceman ctl pin`
//! (persisted with the workspace state). Only a floating window can be pinned: forgetting or
//! tiling the float drops its pin.

use super::{PhysicalWindowId, Workspaces};

impl Workspaces {
    /// Whether `window` shows on every workspace, by hand or by a `pin` rule.
    #[must_use]
    pub(crate) fn is_pinned(&self, window: &PhysicalWindowId) -> bool {
        self.pinned.contains(window) || self.rule_pinned.contains(window)
    }

    /// Whether `window` was pinned by hand, as opposed to by a `pin` rule.
    #[must_use]
    pub(crate) fn is_pinned_by_hand(&self, window: &PhysicalWindowId) -> bool {
        self.pinned.contains(window)
    }

    /// Pin a floating window by hand. Returns whether it is pinned (a tiled or unowned window
    /// cannot be).
    pub(crate) fn pin(&mut self, window: PhysicalWindowId) -> bool {
        if !self.floating_owners.contains_key(&window) {
            return false;
        }
        self.pinned.insert(window);
        true
    }

    /// Drop a hand pin. Returns whether one was dropped.
    pub(crate) fn unpin(&mut self, window: &PhysicalWindowId) -> bool {
        self.pinned.remove(window)
    }

    /// Record whether a `pin` rule matches `window` on this observation.
    pub(crate) fn set_rule_pinned(&mut self, window: &PhysicalWindowId, pinned: bool) {
        if pinned && self.floating_owners.contains_key(window) {
            self.rule_pinned.insert(window.clone());
        } else {
            self.rule_pinned.remove(window);
        }
    }
}
