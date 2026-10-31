use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{
    AppInstanceId, AppObservation, BindingState, Model, NativeObservation, PhysicalWindowId, Rect,
    Slot, SlotId, layout_tree::LayoutTree,
};

const GHOSTTY_REPLACEMENT_SETTLE_CYCLES: u64 = 3;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReconciliationEvidence {
    #[serde(with = "crate::serde_map")]
    pub previous_placements: HashMap<SlotId, Rect>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Settings {
    pub output: Rect,
    #[serde(default = "default_frame_tolerance")]
    pub frame_tolerance: i32,
}

pub const DEFAULT_FRAME_TOLERANCE: i32 = 2;

impl Settings {
    #[must_use]
    pub const fn for_output(output: Rect) -> Self {
        Self {
            output,
            frame_tolerance: DEFAULT_FRAME_TOLERANCE,
        }
    }
}

const fn default_frame_tolerance() -> i32 {
    DEFAULT_FRAME_TOLERANCE
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Placement {
    pub slot: SlotId,
    pub window: Option<PhysicalWindowId>,
    pub frame: Rect,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Decision {
    WindowInserted {
        slot: SlotId,
        window: PhysicalWindowId,
    },
    WindowRemoved {
        slot: SlotId,
        window: PhysicalWindowId,
    },
    NativeTabReplaced {
        slot: SlotId,
        previous: PhysicalWindowId,
        current: PhysicalWindowId,
    },
    NativeTabAmbiguous {
        app: AppInstanceId,
    },
    ObservationUnavailable {
        app: AppInstanceId,
        error: String,
    },
    ApplicationTerminated {
        app: AppInstanceId,
    },
    MouseDownDeferred,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Reconciliation {
    pub model: Model,
    pub placements: Vec<Placement>,
    pub decisions: Vec<Decision>,
}

#[must_use]
pub fn reconcile(
    model: &Model,
    observation: &NativeObservation,
    evidence: &ReconciliationEvidence,
    settings: &Settings,
) -> Reconciliation {
    if observation.mouse_down {
        let mut model = model.clone();
        reset_settling(&mut model, observation.cycle, None);
        return Reconciliation {
            placements: flat_placements(
                settings.output,
                model.workspace.axis,
                &model.workspace.slots,
            ),
            model,
            decisions: vec![Decision::MouseDownDeferred],
        };
    }

    let mut next = model.clone();
    let mut decisions = Vec::new();

    for app_observation in &observation.applications {
        match app_observation {
            AppObservation::Unavailable { app, error } => {
                reset_settling(&mut next, observation.cycle, Some(app));
                decisions.push(Decision::ObservationUnavailable {
                    app: app.clone(),
                    error: error.clone(),
                });
            }
            AppObservation::Terminated { app } => {
                remove_app(&mut next, app, &mut decisions);
                next.continuity.remove(app);
                decisions.push(Decision::ApplicationTerminated { app: app.clone() });
            }
            AppObservation::Complete {
                app,
                focused,
                windows,
            } => {
                let context = AppReconcileContext {
                    focused: focused.as_ref(),
                    windows,
                    cycle: observation.cycle,
                    evidence,
                    settings,
                };
                if !reconcile_unresolved(&mut next, &app.id, &context, &mut decisions) {
                    let current: HashSet<_> =
                        windows.iter().map(|window| window.id.clone()).collect();
                    let existing: HashSet<_> = next
                        .workspace
                        .slots
                        .iter()
                        .filter(|slot| slot.belongs_to(&app.id))
                        .filter_map(|slot| slot.bound_window().cloned())
                        .collect();
                    let stale: Vec<_> = existing.difference(&current).cloned().collect();
                    let new: Vec<_> = current.difference(&existing).cloned().collect();

                    let replacement = app.bundle_id == "com.mitchellh.ghostty"
                        && replacement_candidate(&next, &stale, &new, &context);

                    if replacement {
                        let previous = &stale[0];
                        let current = &new[0];
                        if let Some(slot) = next
                            .workspace
                            .slots
                            .iter_mut()
                            .find(|slot| slot.bound_window() == Some(previous))
                        {
                            slot.binding = BindingState::Bound(current.clone());
                            decisions.push(Decision::NativeTabReplaced {
                                slot: slot.id,
                                previous: previous.clone(),
                                current: current.clone(),
                            });
                        }
                    } else if app.bundle_id == "com.mitchellh.ghostty"
                        && !stale.is_empty()
                        && !new.is_empty()
                    {
                        for slot in &mut next.workspace.slots {
                            let Some(previous) = slot
                                .bound_window()
                                .filter(|window| stale.contains(window))
                                .cloned()
                            else {
                                continue;
                            };
                            slot.binding = BindingState::Unresolved {
                                previous,
                                candidates: new.clone(),
                                since: observation.cycle,
                            };
                        }
                        decisions.push(Decision::NativeTabAmbiguous {
                            app: app.id.clone(),
                        });
                    } else {
                        remove_windows(&mut next, &stale, &mut decisions);
                        insert_windows(&mut next, &new, &mut decisions);
                    }
                }

                next.continuity
                    .entry(app.id.clone())
                    .or_default()
                    .previous_focused
                    .clone_from(focused);
            }
        }
    }

    let placements = flat_placements(settings.output, next.workspace.axis, &next.workspace.slots);
    Reconciliation {
        model: next,
        placements,
        decisions,
    }
}

struct AppReconcileContext<'a> {
    focused: Option<&'a PhysicalWindowId>,
    windows: &'a [crate::ObservedWindow],
    cycle: u64,
    evidence: &'a ReconciliationEvidence,
    settings: &'a Settings,
}

fn reconcile_unresolved(
    model: &mut Model,
    app: &AppInstanceId,
    context: &AppReconcileContext<'_>,
    decisions: &mut Vec<Decision>,
) -> bool {
    let unresolved: Vec<_> = model
        .workspace
        .slots
        .iter()
        .enumerate()
        .filter_map(|(index, slot)| {
            matches!(slot.binding, BindingState::Unresolved { .. })
                .then_some(index)
                .filter(|_| slot.belongs_to(app))
        })
        .collect();
    if unresolved.is_empty() {
        return false;
    }

    let bound: HashSet<_> = model
        .workspace
        .slots
        .iter()
        .filter_map(|slot| slot.bound_window().cloned())
        .collect();
    let candidates: Vec<_> = context
        .windows
        .iter()
        .filter(|window| !bound.contains(&window.id))
        .map(|window| window.id.clone())
        .collect();

    if unresolved.len() == 1 && candidates.len() == 1 && context.focused == candidates.first() {
        let index = unresolved[0];
        let slot = &model.workspace.slots[index];
        if candidate_matches_target(slot.id, &candidates[0], context) {
            let slot_id = slot.id;
            let previous = match &slot.binding {
                BindingState::Unresolved { previous, .. } => previous.clone(),
                BindingState::Bound(_) => unreachable!(),
            };
            model.workspace.slots[index].binding = BindingState::Bound(candidates[0].clone());
            decisions.push(Decision::NativeTabReplaced {
                slot: slot_id,
                previous,
                current: candidates[0].clone(),
            });
            return true;
        }
    }

    let expired = unresolved.iter().all(|index| {
        let BindingState::Unresolved { since, .. } = &model.workspace.slots[*index].binding else {
            unreachable!()
        };
        context.cycle.saturating_sub(*since) >= GHOSTTY_REPLACEMENT_SETTLE_CYCLES
    });
    if expired {
        model.workspace.slots.retain(|slot| match &slot.binding {
            BindingState::Unresolved { previous, .. } if slot.belongs_to(app) => {
                decisions.push(Decision::WindowRemoved {
                    slot: slot.id,
                    window: previous.clone(),
                });
                false
            }
            _ => true,
        });
        insert_windows(model, &candidates, decisions);
        return true;
    }

    for index in unresolved {
        if let BindingState::Unresolved {
            candidates: existing,
            ..
        } = &mut model.workspace.slots[index].binding
        {
            *existing = candidates.clone();
        }
    }
    decisions.push(Decision::NativeTabAmbiguous { app: app.clone() });
    true
}

fn reset_settling(model: &mut Model, cycle: u64, app: Option<&AppInstanceId>) {
    for slot in model
        .workspace
        .slots
        .iter_mut()
        .filter(|slot| app.is_none_or(|app| slot.belongs_to(app)))
    {
        if let BindingState::Unresolved { since, .. } = &mut slot.binding {
            *since = cycle;
        }
    }
}

fn replacement_candidate(
    model: &Model,
    stale: &[PhysicalWindowId],
    new: &[PhysicalWindowId],
    context: &AppReconcileContext<'_>,
) -> bool {
    if stale.len() != 1 || new.len() != 1 || context.focused != new.first() {
        return false;
    }
    if model
        .continuity
        .get(&stale[0].app)
        .and_then(|value| value.previous_focused.as_ref())
        != stale.first()
    {
        return false;
    }
    let Some(slot) = model
        .workspace
        .slots
        .iter()
        .find(|slot| slot.bound_window() == stale.first())
    else {
        return false;
    };
    candidate_matches_target(slot.id, &new[0], context)
}

fn candidate_matches_target(
    slot: SlotId,
    candidate: &PhysicalWindowId,
    context: &AppReconcileContext<'_>,
) -> bool {
    context
        .evidence
        .previous_placements
        .get(&slot)
        .is_some_and(|target| {
            context
                .windows
                .iter()
                .find(|window| &window.id == candidate)
                .is_some_and(|window| {
                    window
                        .frame
                        .approximately_equals(*target, context.settings.frame_tolerance)
                })
        })
}

fn remove_app(model: &mut Model, app: &AppInstanceId, decisions: &mut Vec<Decision>) {
    model.workspace.slots.retain(|slot| {
        if !slot.belongs_to(app) {
            return true;
        }
        if let Some(window) = slot.bound_window().cloned() {
            decisions.push(Decision::WindowRemoved {
                slot: slot.id,
                window,
            });
        }
        false
    });
}

fn remove_windows(model: &mut Model, windows: &[PhysicalWindowId], decisions: &mut Vec<Decision>) {
    model.workspace.slots.retain(|slot| {
        let Some(window) = slot
            .bound_window()
            .filter(|window| windows.contains(window))
            .cloned()
        else {
            return true;
        };
        decisions.push(Decision::WindowRemoved {
            slot: slot.id,
            window,
        });
        false
    });
}

fn insert_windows(model: &mut Model, windows: &[PhysicalWindowId], decisions: &mut Vec<Decision>) {
    let mut windows = windows.to_vec();
    windows.sort_by_key(|window| window.window_id);
    for window in windows {
        let slot_id = SlotId(model.next_slot_id);
        model.next_slot_id += 1;
        model.workspace.slots.push(Slot {
            id: slot_id,
            weight: 1,
            binding: BindingState::Bound(window.clone()),
        });
        decisions.push(Decision::WindowInserted {
            slot: slot_id,
            window,
        });
    }
}

fn flat_placements(output: Rect, axis: crate::Axis, slots: &[Slot]) -> Vec<Placement> {
    slots
        .iter()
        .zip(LayoutTree::weighted_flat(axis, slots.iter()).placements(output))
        .map(|(slot, (slot_id, frame))| Placement {
            slot: slot_id,
            window: slot.bound_window().cloned(),
            frame,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ObservedApplication, ObservedWindow};

    fn weighted_slot(id: u64, weight: u32) -> Slot {
        Slot {
            id: SlotId(id),
            weight,
            binding: BindingState::Bound(PhysicalWindowId {
                app: AppInstanceId {
                    pid: 1,
                    launched_at: 1,
                },
                window_id: id as u32,
            }),
        }
    }

    #[test]
    fn flat_placements_partition_without_gaps_or_rounding_loss() {
        let placements = flat_placements(
            Rect {
                x: 10,
                y: 20,
                width: 101,
                height: 50,
            },
            crate::Axis::Horizontal,
            &[
                weighted_slot(1, 1),
                weighted_slot(2, 1),
                weighted_slot(3, 1),
            ],
        );

        assert_eq!(placements[0].frame.width, 33);
        assert_eq!(placements[1].frame.x, 43);
        assert_eq!(placements[1].frame.width, 34);
        assert_eq!(placements[2].frame.x, 77);
        assert_eq!(placements[2].frame.width, 34);
        assert_eq!(placements[2].frame.x + placements[2].frame.width, 111);
    }

    #[test]
    fn flat_placements_reserve_unresolved_slots() {
        let mut unresolved = weighted_slot(2, 1);
        unresolved.binding = BindingState::Unresolved {
            previous: unresolved.bound_window().unwrap().clone(),
            candidates: Vec::new(),
            since: 4,
        };

        let placements = flat_placements(
            Rect {
                width: 100,
                height: 50,
                ..Rect::default()
            },
            crate::Axis::Horizontal,
            &[weighted_slot(1, 1), unresolved],
        );
        assert_eq!(placements.len(), 2);
        assert_eq!(placements[0].frame.width, 50);
        assert_eq!(placements[1].frame.width, 50);
        assert_eq!(placements[1].window, None);
    }

    #[test]
    fn flat_placements_preserve_cumulative_weight_rounding() {
        let placements = flat_placements(
            Rect {
                x: 10,
                y: 20,
                width: 50,
                height: 101,
            },
            crate::Axis::Vertical,
            &[
                weighted_slot(1, 2),
                weighted_slot(2, 3),
                weighted_slot(3, 5),
            ],
        );

        assert_eq!(placements[0].frame.height, 20);
        assert_eq!(placements[1].frame.y, 40);
        assert_eq!(placements[1].frame.height, 30);
        assert_eq!(placements[2].frame.y, 70);
        assert_eq!(placements[2].frame.height, 51);
        assert_eq!(placements[2].frame.y + placements[2].frame.height, 121);
    }

    const OUTPUT: Rect = Rect {
        x: 0,
        y: 0,
        width: 1000,
        height: 800,
    };

    fn app(pid: i32, launched_at: u64) -> AppInstanceId {
        AppInstanceId { pid, launched_at }
    }

    fn window(app: &AppInstanceId, window_id: u32, frame: Rect) -> ObservedWindow {
        ObservedWindow {
            id: PhysicalWindowId {
                app: app.clone(),
                window_id,
            },
            frame,
            resizable: true,
            title: None,
        }
    }

    fn complete(
        app: &AppInstanceId,
        bundle_id: &str,
        focused: Option<u32>,
        windows: Vec<ObservedWindow>,
    ) -> AppObservation {
        AppObservation::Complete {
            app: ObservedApplication {
                id: app.clone(),
                bundle_id: bundle_id.into(),
            },
            focused: focused.map(|window_id| PhysicalWindowId {
                app: app.clone(),
                window_id,
            }),
            windows,
        }
    }

    fn run(model: &Model, cycle: u64, applications: Vec<AppObservation>) -> Reconciliation {
        reconcile(
            model,
            &NativeObservation {
                cycle,
                mouse_down: false,
                applications,
            },
            &ReconciliationEvidence::default(),
            &Settings {
                output: OUTPUT,
                frame_tolerance: 2,
            },
        )
    }

    #[test]
    fn initial_observation_inserts_windows_in_id_order() {
        let app = app(10, 1);
        let result = run(
            &Model::default(),
            1,
            vec![complete(
                &app,
                "com.example.Editor",
                Some(4),
                vec![window(&app, 4, OUTPUT), window(&app, 2, OUTPUT)],
            )],
        );

        let ids: Vec<_> = result
            .model
            .workspace
            .slots
            .iter()
            .map(|slot| slot.bound_window().unwrap().window_id)
            .collect();
        assert_eq!(ids, vec![2, 4]);
        assert_eq!(result.placements.len(), 2);
    }

    #[test]
    fn ordinary_churn_preserves_existing_slot_and_appends_new_window() {
        let app = app(10, 1);
        let first = run(
            &Model::default(),
            1,
            vec![complete(
                &app,
                "com.example.Editor",
                Some(1),
                vec![window(&app, 1, OUTPUT), window(&app, 2, OUTPUT)],
            )],
        );
        let retained_slot = first.model.workspace.slots[1].id;
        let result = run(
            &first.model,
            2,
            vec![complete(
                &app,
                "com.example.Editor",
                Some(3),
                vec![window(&app, 2, OUTPUT), window(&app, 3, OUTPUT)],
            )],
        );

        assert_eq!(result.model.workspace.slots[0].id, retained_slot);
        assert_eq!(
            result.model.workspace.slots[0]
                .bound_window()
                .unwrap()
                .window_id,
            2
        );
        assert_eq!(
            result.model.workspace.slots[1]
                .bound_window()
                .unwrap()
                .window_id,
            3
        );
    }

    #[test]
    fn unavailable_app_is_preserved_while_another_app_reconciles() {
        let unavailable = app(10, 1);
        let available = app(20, 1);
        let initial = run(
            &Model::default(),
            1,
            vec![
                complete(
                    &unavailable,
                    "a",
                    Some(1),
                    vec![window(&unavailable, 1, OUTPUT)],
                ),
                complete(
                    &available,
                    "b",
                    Some(2),
                    vec![window(&available, 2, OUTPUT)],
                ),
            ],
        );
        let result = run(
            &initial.model,
            2,
            vec![
                AppObservation::Unavailable {
                    app: unavailable.clone(),
                    error: "timeout".into(),
                },
                complete(
                    &available,
                    "b",
                    Some(3),
                    vec![window(&available, 3, OUTPUT)],
                ),
            ],
        );

        assert!(result.model.workspace.slots.iter().any(|slot| {
            slot.bound_window()
                .is_some_and(|window| window.app == unavailable)
        }));
        assert!(result.model.workspace.slots.iter().any(|slot| {
            slot.bound_window()
                .is_some_and(|window| window.window_id == 3)
        }));
    }

    #[test]
    fn termination_only_removes_the_matching_process_incarnation() {
        let old = app(10, 1);
        let new = app(10, 2);
        let initial = run(
            &Model::default(),
            1,
            vec![
                complete(&old, "a", Some(1), vec![window(&old, 1, OUTPUT)]),
                complete(&new, "a", Some(2), vec![window(&new, 2, OUTPUT)]),
            ],
        );
        let result = run(
            &initial.model,
            2,
            vec![AppObservation::Terminated { app: old.clone() }],
        );

        assert_eq!(result.model.workspace.slots.len(), 1);
        assert_eq!(
            result.model.workspace.slots[0].bound_window().unwrap().app,
            new
        );
    }

    #[test]
    fn valid_ghostty_replacement_preserves_slot() {
        let ghostty = app(10, 1);
        let first = run(
            &Model::default(),
            1,
            vec![complete(
                &ghostty,
                "com.mitchellh.ghostty",
                Some(1),
                vec![window(&ghostty, 1, OUTPUT)],
            )],
        );
        let slot = first.model.workspace.slots[0].id;
        let evidence = ReconciliationEvidence {
            previous_placements: HashMap::from([(slot, OUTPUT)]),
        };
        let result = reconcile(
            &first.model,
            &NativeObservation {
                cycle: 2,
                mouse_down: false,
                applications: vec![complete(
                    &ghostty,
                    "com.mitchellh.ghostty",
                    Some(2),
                    vec![window(&ghostty, 2, OUTPUT)],
                )],
            },
            &evidence,
            &Settings {
                output: OUTPUT,
                frame_tolerance: 2,
            },
        );

        assert_eq!(result.model.workspace.slots.len(), 1);
        assert_eq!(result.model.workspace.slots[0].id, slot);
        assert_eq!(
            result.model.workspace.slots[0]
                .bound_window()
                .unwrap()
                .window_id,
            2
        );
        assert!(matches!(
            result.decisions[0],
            Decision::NativeTabReplaced { .. }
        ));
    }

    #[test]
    fn ambiguous_ghostty_replacement_reserves_the_slot_without_a_write() {
        let ghostty = app(10, 1);
        let first = run(
            &Model::default(),
            1,
            vec![complete(
                &ghostty,
                "com.mitchellh.ghostty",
                Some(1),
                vec![window(&ghostty, 1, OUTPUT)],
            )],
        );
        let result = run(
            &first.model,
            2,
            vec![complete(
                &ghostty,
                "com.mitchellh.ghostty",
                Some(2),
                vec![window(
                    &ghostty,
                    2,
                    Rect {
                        width: 500,
                        ..OUTPUT
                    },
                )],
            )],
        );

        assert!(matches!(
            result.model.workspace.slots[0].binding,
            BindingState::Unresolved { .. }
        ));
        assert_eq!(result.placements.len(), 1);
        assert_eq!(result.placements[0].window, None);
        assert_eq!(result.placements[0].frame, OUTPUT);

        let retried = run(
            &result.model,
            3,
            vec![complete(
                &ghostty,
                "com.mitchellh.ghostty",
                Some(2),
                vec![window(
                    &ghostty,
                    2,
                    Rect {
                        width: 500,
                        ..OUTPUT
                    },
                )],
            )],
        );
        assert_eq!(retried.model.workspace.slots.len(), 1);
        assert!(matches!(
            retried.model.workspace.slots[0].binding,
            BindingState::Unresolved { .. }
        ));
    }

    #[test]
    fn unresolved_ghostty_slot_expires_after_bounded_settling() {
        let ghostty = app(10, 1);
        let first = run(
            &Model::default(),
            1,
            vec![complete(
                &ghostty,
                "com.mitchellh.ghostty",
                Some(1),
                vec![window(&ghostty, 1, OUTPUT)],
            )],
        );
        let settling = run(
            &first.model,
            2,
            vec![complete(
                &ghostty,
                "com.mitchellh.ghostty",
                Some(2),
                vec![window(
                    &ghostty,
                    2,
                    Rect {
                        width: 500,
                        ..OUTPUT
                    },
                )],
            )],
        );

        let still_settling = run(
            &settling.model,
            4,
            vec![complete(
                &ghostty,
                "com.mitchellh.ghostty",
                None,
                Vec::new(),
            )],
        );
        assert_eq!(still_settling.model.workspace.slots.len(), 1);

        let expired = run(
            &still_settling.model,
            5,
            vec![complete(
                &ghostty,
                "com.mitchellh.ghostty",
                None,
                Vec::new(),
            )],
        );
        assert!(expired.model.workspace.slots.is_empty());
        assert!(matches!(
            expired.decisions.as_slice(),
            [Decision::WindowRemoved { .. }]
        ));
    }

    #[test]
    fn incomplete_observations_do_not_advance_ghostty_settling() {
        let ghostty = app(10, 1);
        let first = run(
            &Model::default(),
            1,
            vec![complete(
                &ghostty,
                "com.mitchellh.ghostty",
                Some(1),
                vec![window(&ghostty, 1, OUTPUT)],
            )],
        );
        let ambiguous = run(
            &first.model,
            2,
            vec![complete(
                &ghostty,
                "com.mitchellh.ghostty",
                Some(2),
                vec![window(
                    &ghostty,
                    2,
                    Rect {
                        width: 500,
                        ..OUTPUT
                    },
                )],
            )],
        );
        let mouse_down = reconcile(
            &ambiguous.model,
            &NativeObservation {
                cycle: 7,
                mouse_down: true,
                applications: Vec::new(),
            },
            &ReconciliationEvidence::default(),
            &Settings {
                output: OUTPUT,
                frame_tolerance: 2,
            },
        );
        let unavailable = run(
            &mouse_down.model,
            8,
            vec![AppObservation::Unavailable {
                app: ghostty.clone(),
                error: "timed out".into(),
            }],
        );
        let retried = run(
            &unavailable.model,
            9,
            vec![complete(
                &ghostty,
                "com.mitchellh.ghostty",
                Some(2),
                vec![window(
                    &ghostty,
                    2,
                    Rect {
                        width: 500,
                        ..OUTPUT
                    },
                )],
            )],
        );

        assert_eq!(retried.model.workspace.slots.len(), 1);
        assert!(matches!(
            retried.model.workspace.slots[0].binding,
            BindingState::Unresolved { .. }
        ));
    }

    #[test]
    fn mouse_down_defers_all_model_changes() {
        let app = app(10, 1);
        let model = Model::default();
        let result = reconcile(
            &model,
            &NativeObservation {
                cycle: 1,
                mouse_down: true,
                applications: vec![complete(&app, "a", Some(1), vec![window(&app, 1, OUTPUT)])],
            },
            &ReconciliationEvidence::default(),
            &Settings {
                output: OUTPUT,
                frame_tolerance: 2,
            },
        );

        assert_eq!(result.model, model);
        assert_eq!(result.decisions, vec![Decision::MouseDownDeferred]);
    }
}
