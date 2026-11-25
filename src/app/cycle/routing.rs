//! Routing before reconciliation: a newly seen window bound for another workspace (a `route`
//! rule, the setup tour's arrangement, a detached tab, a remembered placement on a hidden
//! workspace) joins that workspace directly, before any display's reconciliation runs. It is
//! never inserted into the workspace on screen and then pulled back out, so a park that fails
//! leaves it unowned (and routed again next cycle) instead of tiled on the wrong workspace.
//!
//! Floats bound for another workspace move onto that workspace's display here too: on screen
//! now when it shows, or parked with a recovery frame on its display when it is hidden.

use super::affinity::{FloatHome, FrozenAffinityPolicy, FrozenWindowAffinity};
use super::*;
use crate::journal::PlacementSource;

impl<B: WorkspaceBackend> WorkspaceRuntime<B> {
    pub(super) fn route_before_reconcile(
        &mut self,
        policy: &mut FrozenAffinityPolicy,
        observation: &crate::NativeObservation,
        visible: &[ResolvedWorkspace],
        outputs: &macos::DisplayOutputs,
        failed: &mut usize,
    ) {
        let frames = live::observed_frames(observation);
        let mut routes: Vec<_> = policy
            .windows
            .iter()
            .filter_map(|(window, affinity)| match *affinity {
                FrozenWindowAffinity::ExplicitRoute(target)
                | FrozenWindowAffinity::DetachedTab(target) => {
                    Some((window.clone(), target, *affinity))
                }
                // A recall to a workspace on screen is reconciled there directly.
                FrozenWindowAffinity::Inferred(target)
                    if !policy.visible_workspaces.contains(&target) =>
                {
                    Some((window.clone(), target, *affinity))
                }
                _ => None,
            })
            .collect();
        routes.sort_by(|left, right| left.0.cmp(&right.0));
        for (window, target, affinity) in routes {
            // Routed from the managed display it opened on, like any adoption.
            let Some(source) = frames.get(&window).and_then(|frame| {
                visible.iter().find(|resolved| {
                    self.scope.manages_selector(&resolved.selector)
                        && resolved.output.contains_center(*frame)
                })
            }) else {
                continue;
            };
            if source.workspace == target || self.workspaces.workspace_of(&window).is_some() {
                continue;
            }
            let target_output = output_for(&self.workspaces, target, outputs);
            if !policy.visible_workspaces.contains(&target) {
                let outcome =
                    self.park_window_across(&window, source.output, target_output, outputs);
                if outcome != macos::ApplyOutcome::Applied {
                    if outcome != macos::ApplyOutcome::Gone {
                        log_event!("route {window:?} -> workspace {target}: park failed; retrying");
                        *failed += 1;
                    }
                    // Unowned until a later cycle routes it again.
                    policy.windows.insert(window, FrozenWindowAffinity::Ignored);
                    continue;
                }
            }
            let anchor = self.insertion_anchor(target);
            let canvas = self.layout_settings(target_output).canvas();
            self.workspaces.adopt_tile(
                window.clone(),
                target,
                self.config.layout(),
                canvas,
                anchor,
            );
            log_event!("route {window:?} -> workspace {target} ({affinity:?})");
            match affinity {
                FrozenWindowAffinity::ExplicitRoute(_) => {
                    self.note_placement(&window, PlacementSource::Rule, target);
                }
                FrozenWindowAffinity::Inferred(_) => {
                    self.note_placement(&window, PlacementSource::Recall, target);
                }
                _ => {}
            }
            self.exit_fullscreen_if_split(target);
            self.clear_fullscreen_restore_if_inactive(target);
            policy
                .windows
                .insert(window, FrozenWindowAffinity::Owned(target));
        }
        for home in std::mem::take(&mut policy.float_homes) {
            if !self.place_float_home(&home, &policy.visible_workspaces, outputs) {
                *failed += 1;
            }
        }
    }

    /// Where a window routed to `workspace` joins its tiles: next to its last focused tile in
    /// a dwindle layout, at the end in columns.
    fn insertion_anchor(&self, workspace: WorkspaceId) -> Option<crate::SlotId> {
        match self.config.layout() {
            crate::config::LayoutMode::Dwindle => self
                .workspaces
                .preferred_focus(workspace)
                .and_then(|window| self.workspaces.engine(workspace).slot_for_window(&window)),
            crate::config::LayoutMode::Columns => None,
        }
    }

    /// Brings a float its observation sent to another workspace onto that workspace's display:
    /// to its remembered frame or the same place on that display when the workspace shows,
    /// else parked with its recovery frame there (#415, #416).
    fn place_float_home(
        &mut self,
        home: &FloatHome,
        visible: &HashSet<WorkspaceId>,
        outputs: &macos::DisplayOutputs,
    ) -> bool {
        let displays: Vec<_> = outputs.frames().collect();
        let Some(from) = Self::display_for_frame(home.observed, &displays) else {
            return true;
        };
        let to = output_for(&self.workspaces, home.workspace, outputs);
        if !visible.contains(&home.workspace) {
            return matches!(
                self.park_window_across(&home.window, from, to, outputs),
                macos::ApplyOutcome::Applied | macos::ApplyOutcome::Gone
            );
        }
        match home.remembered.filter(|frame| to.contains_center(*frame)) {
            Some(frame) if !frame.position_approximately_equals(home.observed, 1) => {
                let outcome = self.backend.set_position(&home.window, frame);
                log_event!(
                    "float home: {:?} to {:?} ({:?})",
                    home.window,
                    frame,
                    outcome
                );
                matches!(
                    outcome,
                    macos::ApplyOutcome::Applied | macos::ApplyOutcome::Gone
                )
            }
            Some(_) => true,
            None if from == to => true,
            None => self.carry_floating_window(&home.window, from, to),
        }
    }
}
