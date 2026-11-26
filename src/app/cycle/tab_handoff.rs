//! A native-tab switch caught mid-handoff under a route-every-window rule (#706).
//!
//! Routing runs before any display is reconciled, so a rule that routes every new window
//! decides on one snapshot. An incoming native tab can show up at its tile's frame while the
//! outgoing tab is still listed, or before focus reaches it. That snapshot can't prove a
//! replacement yet, and routing it would send the tab to the rule's workspace and retire
//! its tile. Such a window is held instead, its app left unreconciled, for a bounded settle;
//! a window still unexplained after that is routed as new.

use super::*;

/// How long an incoming tab at its tile's frame may wait for the evidence of a replacement.
pub(in crate::app) const TAB_HANDOFF_SETTLE: Duration = Duration::from_secs(1);

impl<B: WorkspaceBackend> WorkspaceRuntime<B> {
    /// When `window`, unbound and at the frame of one of its app's tiles on the visible
    /// workspace it opened over, is held mid-handoff: the deadline its hold ends.
    pub(super) fn hold_tab_handoff(
        &mut self,
        window: &crate::ObservedWindow,
        windows: &[crate::ObservedWindow],
        visible: &[ResolvedWorkspace],
    ) -> Option<Instant> {
        let resolved = visible
            .iter()
            .find(|resolved| resolved.screen.contains_center(window.frame))?;
        let pending = self
            .workspaces
            .engine(resolved.workspace)
            .native_tab_handoff_pending(&self.layout_settings(resolved.output), window, windows);
        if !pending {
            return None;
        }
        let now = self.backend.now();
        let since = *self
            .tab_handoffs
            .entry(window.id.clone())
            .or_insert_with(|| {
                log_event!(
                    "hold {:?}: a native tab mid-handoff at workspace {}'s tile",
                    window.id,
                    resolved.workspace
                );
                now
            });
        let deadline = since + TAB_HANDOFF_SETTLE;
        (now < deadline).then_some(deadline)
    }
}
