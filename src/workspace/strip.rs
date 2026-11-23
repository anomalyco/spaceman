//! Per-workspace Scrolling Strip (a Lab) state: the workspace's own switch, its column widths
//! and viewport anchor. Whether the Lab itself is on is the runtime's question.

use super::{WorkspaceId, Workspaces};
use crate::strip::StripState;

impl Workspaces {
    /// The workspace's own Scrolling Strip switch, whether or not the Lab is on.
    pub(crate) fn uses_strip_layout(&self, workspace: WorkspaceId) -> bool {
        self.strip[workspace.index()].enabled
    }

    /// Turning the Scrolling Strip on turns Fullscreen Layout off. Unlike Fullscreen Layout it
    /// survives an empty workspace, so the next window opens into the strip.
    pub(crate) fn set_strip_layout(&mut self, workspace: WorkspaceId, enabled: bool) {
        self.strip[workspace.index()].enabled = enabled;
        if enabled {
            self.fullscreen_layout[workspace.index()] = false;
        }
    }

    pub(crate) fn strip(&self, workspace: WorkspaceId) -> &StripState {
        &self.strip[workspace.index()]
    }

    pub(crate) fn strip_mut(&mut self, workspace: WorkspaceId) -> &mut StripState {
        &mut self.strip[workspace.index()]
    }

    /// The slot focus last rested on in `workspace`.
    pub(crate) fn last_focused_slot(&self, workspace: WorkspaceId) -> Option<crate::SlotId> {
        self.last_focused[workspace.index()]
    }

    /// Restores persisted strip state; widths and anchors of slots that no longer exist go.
    pub(super) fn restore_strip(&mut self, workspace: WorkspaceId, strip: StripState) {
        let enabled = strip.enabled;
        self.strip[workspace.index()] = strip;
        self.set_strip_layout(workspace, enabled);
        self.prune_intents(workspace);
    }

    /// Drops widths and the anchor of slots the workspace no longer has.
    pub(super) fn prune_strip(&mut self, workspace: WorkspaceId) {
        let index = workspace.index();
        let engine = &self.engines[index];
        self.strip[index].retain_slots(|slot| engine.model().slot(slot).is_some());
    }
}
