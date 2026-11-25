//! Labs › `open_on_chosen_workspace`: after an explicit choice of an empty workspace (a
//! workspace shortcut, `ctl switch`, the menu or the HUD) with more than one display, the next
//! window the user opens joins that workspace, wherever macOS put it. Showing an empty
//! workspace focuses nothing, so macOS can open that window on the display focus is still on.
//!
//! The choice lasts until focus goes to a window Spaceman already placed (other than the one
//! focused when it was made), another command runs, or the workspace stops being empty and on
//! screen. A new window that takes focus is claimed only once a complete observation lists it,
//! and only when it is its app's one new window, so a window opened in the background or a
//! launch that restores several windows opens as it always has. A focused window no
//! observation lists (a launcher's panel), an ignored window, or one a rule or built-in default
//! floats is passed over without ending the choice, so the app it launches still gets it.
//! Rules, remembered placements and dialogs decide before the choice is consulted
//! (`super::cycle::affinity`).

use std::collections::HashSet;

use super::*;
use crate::config::DispositionPatch;

/// An empty workspace just chosen, awaiting the window the user opens next.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ChosenWorkspace {
    pub(super) workspace: WorkspaceId,
    /// The focus reported when it was chosen; reporting it still means nothing took focus.
    left: Option<crate::PhysicalWindowId>,
    /// The new window the system-wide focus reports this cycle, if it is one.
    focused_new: Option<crate::PhysicalWindowId>,
    /// The frontmost app this cycle. Before the system-wide focus names the new window, its
    /// app's own report in the snapshot can, once that app is frontmost.
    frontmost: Option<crate::AppInstanceId>,
    /// New windows that took focus but are not the user's next window (ignored, or floated by
    /// a rule or built-in default, such as a launcher): focusing them again keeps the choice.
    passed: HashSet<crate::PhysicalWindowId>,
}

impl<B: WorkspaceBackend> WorkspaceRuntime<B> {
    /// After an explicit switch or selection of `workspace`: remembers it when it is empty and
    /// on screen and another display could get the next window; otherwise forgets any choice.
    pub(super) fn note_chosen_workspace(
        &mut self,
        workspace: WorkspaceId,
        outputs: &macos::DisplayOutputs,
    ) {
        self.chosen_workspace = None;
        if !self.config.labs().open_on_chosen_workspace
            || outputs.frames().nth(1).is_none()
            || !self.workspaces.windows(workspace).is_empty()
            || !self.is_visible(workspace, outputs)
        {
            return;
        }
        let left = self.backend.focused_window();
        log_event!("chosen workspace: {workspace} awaits the next window (focus {left:?})");
        self.chosen_workspace = Some(ChosenWorkspace {
            workspace,
            left,
            focused_new: None,
            frontmost: None,
            passed: HashSet::new(),
        });
    }

    /// Each cycle, before window affinities freeze: keeps the choice while focus stays where
    /// it was or is on a new window, notes that window, and drops the choice once focus goes to
    /// a window Spaceman placed, or the workspace has a window or is not on screen.
    pub(super) fn follow_chosen_workspace(
        &mut self,
        focused: Option<&crate::PhysicalWindowId>,
        frontmost: Option<&crate::AppInstanceId>,
        visible: &[ResolvedWorkspace],
    ) {
        let Some(chosen) = self.chosen_workspace.as_ref() else {
            return;
        };
        let workspace = chosen.workspace;
        let mut focused_new = None;
        let reason = if !self.config.labs().open_on_chosen_workspace {
            Some("the Lab is off")
        } else if !visible
            .iter()
            .any(|resolved| resolved.workspace == workspace)
        {
            Some("it is no longer on screen")
        } else if self
            .workspaces
            .windows(workspace)
            .iter()
            .any(|window| !chosen.passed.contains(window))
        {
            // A launcher passed over on the way may float there; it is not the next window.
            Some("it has a window")
        } else {
            match focused {
                None => None,
                Some(window)
                    if chosen.left.as_ref() == Some(window) || chosen.passed.contains(window) =>
                {
                    None
                }
                Some(window) if self.is_unplaced(window) => {
                    focused_new = Some(window.clone());
                    None
                }
                Some(_) => Some("focus moved to another window"),
            }
        };
        if let Some(reason) = reason {
            log_event!("chosen workspace: dropped {workspace}: {reason}");
            self.chosen_workspace = None;
        } else if let Some(chosen) = &mut self.chosen_workspace {
            chosen.focused_new = focused_new;
            chosen.frontmost = frontmost.cloned();
        }
    }

    /// Whether `window` is new to Spaceman: no workspace, scratchpad or parking record has it.
    fn is_unplaced(&self, window: &crate::PhysicalWindowId) -> bool {
        self.workspaces.workspace_of_any(window).is_none()
            && self.workspaces.residency(window).is_none()
            && !self.parked.contains_key(window)
    }

    /// Before `app`'s window affinities freeze: the new window focus went to, once this complete
    /// observation lists it, settles the choice. That is the window the system-wide focus
    /// reports or, while that report lags the snapshot, the one the frontmost app itself
    /// reports focused (`focused`). An ignored window, or one a rule or built-in default
    /// floats, is passed over and the choice stays. Returns the window and workspace when it
    /// is the app's only new window in `windows`.
    pub(super) fn take_chosen_window(
        &mut self,
        app: &crate::ObservedApplication,
        focused: Option<&crate::PhysicalWindowId>,
        windows: &[crate::ObservedWindow],
    ) -> Option<(crate::PhysicalWindowId, WorkspaceId)> {
        let chosen = self.chosen_workspace.as_ref()?;
        let app_focused = focused
            .filter(|_| chosen.frontmost.as_ref() == Some(&app.id))
            .filter(|window| chosen.left.as_ref() != Some(*window));
        let observed = windows.iter().find(|window| {
            !chosen.passed.contains(&window.id)
                && self.is_unplaced(&window.id)
                && (chosen.focused_new.as_ref() == Some(&window.id)
                    || app_focused == Some(&window.id))
        })?;
        let rules = self.config.patch(&app.bundle_id, observed);
        let patch = rules.over(DispositionPatch::traits(observed.resizable, false));
        if patch.is_ignored() || rules.floats() {
            log_event!(
                "chosen workspace: passed over {:?}: a rule or default decides it",
                observed.id
            );
            if let Some(chosen) = &mut self.chosen_workspace {
                chosen.passed.insert(observed.id.clone());
            }
            return None;
        }
        let candidate = observed.id.clone();
        let chosen = self.chosen_workspace.take()?;
        // An ignored window is never placed, so it would look new on every cycle.
        let new = windows
            .iter()
            .filter(|window| {
                self.is_unplaced(&window.id)
                    && !self
                        .config
                        .patch(&app.bundle_id, window)
                        .over(DispositionPatch::traits(window.resizable, false))
                        .is_ignored()
            })
            .count();
        if new != 1 {
            log_event!(
                "chosen workspace: dropped {}: {candidate:?} opened with {new} new window(s)",
                chosen.workspace
            );
            return None;
        }
        Some((candidate, chosen.workspace))
    }
}
