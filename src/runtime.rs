use crate::{
    Model, NativeObservation, PhysicalWindowId, Placement, Reconciliation, ReconciliationEvidence,
    Settings, Slot, layout_tree::LayoutTree, reconcile,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CycleTicket {
    pub cycle: u64,
    pub revision: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StaleCycle {
    pub ticket: CycleTicket,
    pub current_revision: u64,
}

#[derive(Clone, Debug, Default)]
pub struct Engine {
    model: Model,
    layout: LayoutTree,
    evidence: ReconciliationEvidence,
    revision: u64,
    next_cycle: u64,
    pending: Option<CycleTicket>,
}

impl Engine {
    pub fn invalidate(&mut self) {
        self.pending = None;
        self.revision = self.revision.saturating_add(1);
    }

    pub fn begin_cycle(&mut self) -> CycleTicket {
        self.next_cycle = self.next_cycle.saturating_add(1);
        let ticket = CycleTicket {
            cycle: self.next_cycle,
            revision: self.revision,
        };
        self.pending = Some(ticket);
        ticket
    }

    pub fn accept(
        &mut self,
        ticket: CycleTicket,
        observation: &NativeObservation,
        settings: &Settings,
    ) -> Result<Reconciliation, StaleCycle> {
        if self.pending != Some(ticket) || observation.cycle != ticket.cycle {
            return Err(StaleCycle {
                ticket,
                current_revision: self.revision,
            });
        }
        self.pending = None;

        let mut result = reconcile(&self.model, observation, &self.evidence, settings);
        self.layout
            .sync(result.model.workspace.axis, &result.model.workspace.slots);
        result.placements = placements_for(&self.layout, &result.model, settings.output);
        self.evidence.previous_placements = result
            .placements
            .iter()
            .map(|placement| (placement.slot, placement.frame))
            .collect();
        self.model = result.model.clone();
        Ok(result)
    }

    #[must_use]
    pub const fn model(&self) -> &Model {
        &self.model
    }

    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub fn bound_windows(&self) -> Vec<PhysicalWindowId> {
        self.layout
            .slots()
            .into_iter()
            .filter_map(|id| self.model.slot(id)?.bound_window().cloned())
            .collect()
    }

    #[must_use]
    pub fn placements(&self, settings: &Settings) -> Vec<Placement> {
        placements_for(&self.layout, &self.model, settings.output)
    }

    pub(crate) fn record_placement_target(&mut self, slot: crate::SlotId, frame: crate::Rect) {
        self.evidence.previous_placements.insert(slot, frame);
    }

    pub(crate) fn toggle_axis(&mut self) {
        self.layout.toggle_root_axis();
        self.commit_layout();
    }

    pub(crate) fn take_slot(&mut self, window: &PhysicalWindowId) -> Option<Slot> {
        let index = self
            .model
            .workspace
            .slots
            .iter()
            .position(|slot| slot.bound_window() == Some(window))?;
        let slot = self.model.workspace.slots.remove(index);
        self.layout
            .sync(self.model.workspace.axis, &self.model.workspace.slots);
        self.evidence.previous_placements.remove(&slot.id);
        self.invalidate();
        Some(slot)
    }

    /// Remove every slot, bound or unresolved, whose identity belongs to `app`.
    pub(crate) fn forget_app(&mut self, app: &crate::AppInstanceId) -> bool {
        let before = self.model.workspace.slots.len();
        self.model
            .workspace
            .slots
            .retain(|slot| !slot.belongs_to(app));
        self.model.continuity.remove(app);
        if self.model.workspace.slots.len() == before {
            return false;
        }
        let slots = &self.model.workspace.slots;
        self.evidence
            .previous_placements
            .retain(|id, _| slots.iter().any(|slot| slot.id == *id));
        self.layout
            .sync(self.model.workspace.axis, &self.model.workspace.slots);
        self.invalidate();
        true
    }

    pub(crate) fn insert_slot(&mut self, mut slot: Slot) {
        if self
            .model
            .workspace
            .slots
            .iter()
            .any(|existing| existing.id == slot.id)
        {
            slot.id = crate::SlotId(self.model.next_slot_id);
        }
        self.model.next_slot_id = self.model.next_slot_id.max(slot.id.0.saturating_add(1));
        self.model.workspace.slots.push(slot);
        self.layout
            .sync(self.model.workspace.axis, &self.model.workspace.slots);
        self.invalidate();
    }

    pub(crate) fn swap_windows(
        &mut self,
        first: &PhysicalWindowId,
        second: &PhysicalWindowId,
    ) -> bool {
        let first = self.slot_for_window(first);
        let second = self.slot_for_window(second);
        let Some((first, second)) = first.zip(second).filter(|(first, second)| first != second)
        else {
            return false;
        };
        if !self.layout.swap(first, second) {
            return false;
        }
        self.invalidate();
        true
    }

    pub(crate) fn resize_window(
        &mut self,
        window: &PhysicalWindowId,
        current: crate::Rect,
        output: crate::Rect,
    ) -> bool {
        self.edit_slot(window, |layout, slot| layout.resize(slot, current, output))
    }

    pub(crate) fn fit_minimums(
        &mut self,
        output: crate::Rect,
        minimums: &std::collections::HashMap<PhysicalWindowId, (i32, i32)>,
    ) -> bool {
        let slots = self
            .model
            .workspace
            .slots
            .iter()
            .filter_map(|slot| Some((slot.id, *minimums.get(slot.bound_window()?)?)))
            .collect();
        if !self.layout.fit_minimums(output, &slots) {
            return false;
        }
        self.commit_layout();
        true
    }

    pub(crate) fn resize_focused_tile(
        &mut self,
        window: &PhysicalWindowId,
        direction: crate::ResizeDirection,
        amount: i32,
        output: crate::Rect,
    ) -> bool {
        self.edit_slot(window, |layout, slot| {
            layout.resize_by(slot, direction, amount, output)
        })
    }

    pub(crate) fn balance_focused_parent(
        &mut self,
        window: &PhysicalWindowId,
        output: crate::Rect,
    ) -> bool {
        self.edit_slot(window, |layout, slot| layout.balance(slot, output))
    }

    pub(crate) fn create_implicit_split(
        &mut self,
        window: &PhysicalWindowId,
        direction: crate::Direction,
    ) -> bool {
        self.edit_slot(window, |layout, slot| {
            layout.create_implicit_split(slot, direction)
        })
    }

    pub(crate) fn move_window_relative(
        &mut self,
        moved: &PhysicalWindowId,
        target: &PhysicalWindowId,
        direction: crate::Direction,
    ) -> bool {
        let Some(target) = self.slot_for_window(target) else {
            return false;
        };
        self.edit_slot(moved, |layout, moved| {
            layout.move_relative(moved, target, direction)
        })
    }

    pub(crate) fn layout(&self) -> &LayoutTree {
        &self.layout
    }

    pub(crate) const fn root_axis(&self) -> crate::Axis {
        self.layout.root_axis()
    }

    pub(crate) fn preview(&self, output: crate::Rect) -> Vec<crate::Rect> {
        self.layout
            .placements(output)
            .into_iter()
            .map(|(_, frame)| frame)
            .collect()
    }

    pub(crate) fn restore_layout(&mut self, mut layout: LayoutTree) {
        layout.sync(self.model.workspace.axis, &self.model.workspace.slots);
        self.layout = layout;
        self.commit_layout();
    }

    pub(crate) fn slot_for_window(&self, window: &PhysicalWindowId) -> Option<crate::SlotId> {
        self.model
            .workspace
            .slots
            .iter()
            .find(|slot| slot.bound_window() == Some(window))
            .map(|slot| slot.id)
    }

    fn edit_slot(
        &mut self,
        window: &PhysicalWindowId,
        op: impl FnOnce(&mut LayoutTree, crate::SlotId) -> bool,
    ) -> bool {
        let Some(slot) = self.slot_for_window(window) else {
            return false;
        };
        if !op(&mut self.layout, slot) {
            return false;
        }
        self.commit_layout();
        true
    }

    fn commit_layout(&mut self) {
        self.model.workspace.axis = self.layout.root_axis();
        for slot in &mut self.model.workspace.slots {
            if let Some(weight) = self.layout.weight(slot.id) {
                slot.weight = weight;
            }
        }
        self.invalidate();
    }
}

fn placements_for(layout: &LayoutTree, model: &Model, output: crate::Rect) -> Vec<Placement> {
    layout
        .placements(output)
        .into_iter()
        .filter_map(|(id, frame)| {
            let slot = model.slot(id)?;
            Some(Placement {
                slot: id,
                window: slot.bound_window().cloned(),
                frame,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::{
        AppInstanceId, AppObservation, NativeObservation, ObservedApplication, ObservedWindow,
        PhysicalWindowId, Rect,
    };

    use super::*;

    fn slot(id: u64, weight: u32) -> Slot {
        Slot {
            id: crate::SlotId(id),
            weight,
            binding: crate::BindingState::Bound(PhysicalWindowId {
                app: AppInstanceId {
                    pid: 1,
                    launched_at: 1,
                },
                window_id: id as u32,
            }),
        }
    }

    #[test]
    fn structural_layout_edits_commit_axis_and_slot_weights() {
        let mut engine = Engine::default();
        let slots = [slot(1, 1), slot(2, 1), slot(3, 1)];
        let windows = slots
            .iter()
            .map(|slot| slot.bound_window().unwrap().clone())
            .collect::<Vec<_>>();
        for slot in slots {
            engine.insert_slot(slot);
        }
        let output = Rect {
            width: 1000,
            height: 800,
            ..Rect::default()
        };
        assert!(engine.resize_focused_tile(&windows[0], crate::ResizeDirection::Grow, 50, output));
        assert!(engine.create_implicit_split(&windows[1], crate::Direction::Up));
        assert_eq!(engine.model().workspace.axis, crate::Axis::Vertical);
        assert_eq!(engine.model().workspace.slots[1].weight, 1);

        assert!(engine.move_window_relative(&windows[2], &windows[0], crate::Direction::Right));
        assert_eq!(engine.model().workspace.slots[2].weight, 1);
    }

    fn observation(cycle: u64, window_id: u32) -> NativeObservation {
        let app = AppInstanceId {
            pid: 10,
            launched_at: 1,
        };
        let window = PhysicalWindowId {
            app: app.clone(),
            window_id,
        };
        NativeObservation {
            cycle,
            mouse_down: false,
            applications: vec![AppObservation::Complete {
                app: ObservedApplication {
                    id: app,
                    bundle_id: "com.example.Editor".into(),
                },
                focused: Some(window.clone()),
                windows: vec![ObservedWindow {
                    id: window,
                    frame: Rect {
                        width: 100,
                        height: 100,
                        ..Rect::default()
                    },
                    resizable: true,
                    title: None,
                }],
            }],
        }
    }

    fn settings() -> Settings {
        Settings {
            output: Rect {
                width: 100,
                height: 100,
                ..Rect::default()
            },
            frame_tolerance: 2,
        }
    }

    #[test]
    fn accepted_cycle_commits_model_and_evidence() {
        let mut engine = Engine::default();
        engine.invalidate();
        let ticket = engine.begin_cycle();
        let result = engine.accept(ticket, &observation(ticket.cycle, 1), &settings());

        assert_eq!(result.unwrap().model.workspace.slots.len(), 1);
        assert_eq!(engine.model().workspace.slots.len(), 1);
    }

    #[test]
    fn invalidation_rejects_an_in_flight_cycle_without_mutation() {
        let mut engine = Engine::default();
        engine.invalidate();
        let ticket = engine.begin_cycle();
        engine.invalidate();

        let error = engine
            .accept(ticket, &observation(ticket.cycle, 1), &settings())
            .unwrap_err();
        assert_eq!(error.current_revision, 2);
        assert!(engine.model().workspace.slots.is_empty());
    }

    #[test]
    fn mismatched_observation_cycle_is_rejected() {
        let mut engine = Engine::default();
        let ticket = engine.begin_cycle();

        assert!(
            engine
                .accept(ticket, &observation(ticket.cycle + 1, 1), &settings())
                .is_err()
        );
    }

    #[test]
    fn newer_cycle_supersedes_an_older_ticket_in_the_same_revision() {
        let mut engine = Engine::default();
        let older = engine.begin_cycle();
        let current = engine.begin_cycle();

        assert!(
            engine
                .accept(older, &observation(older.cycle, 1), &settings())
                .is_err()
        );
        assert!(
            engine
                .accept(current, &observation(current.cycle, 1), &settings())
                .is_ok()
        );
    }

    #[test]
    fn accepted_ticket_cannot_be_committed_twice() {
        let mut engine = Engine::default();
        let ticket = engine.begin_cycle();
        let observation = observation(ticket.cycle, 1);

        assert!(engine.accept(ticket, &observation, &settings()).is_ok());
        assert!(engine.accept(ticket, &observation, &settings()).is_err());
    }

    #[test]
    fn reordering_a_window_invalidates_pending_work() {
        let first = PhysicalWindowId {
            app: AppInstanceId {
                pid: 1,
                launched_at: 1,
            },
            window_id: 1,
        };
        let second = PhysicalWindowId {
            window_id: 2,
            ..first.clone()
        };
        let mut engine = Engine::default();
        engine.insert_slot(Slot {
            id: crate::SlotId(1),
            weight: 1,
            binding: crate::BindingState::Bound(first.clone()),
        });
        engine.insert_slot(Slot {
            id: crate::SlotId(2),
            weight: 1,
            binding: crate::BindingState::Bound(second.clone()),
        });
        let pending = engine.begin_cycle();

        assert!(engine.swap_windows(&second, &first));
        assert_eq!(engine.bound_windows(), vec![second, first]);
        assert!(
            engine
                .accept(pending, &observation(pending.cycle, 1), &settings())
                .is_err()
        );
    }

    #[test]
    fn taking_a_slot_invalidates_pending_work() {
        let window = PhysicalWindowId {
            app: AppInstanceId {
                pid: 1,
                launched_at: 1,
            },
            window_id: 1,
        };
        let mut engine = Engine::default();
        engine.insert_slot(Slot {
            id: crate::SlotId(1),
            weight: 1,
            binding: crate::BindingState::Bound(window.clone()),
        });
        let pending = engine.begin_cycle();

        assert!(engine.take_slot(&window).is_some());
        assert!(
            engine
                .accept(pending, &observation(pending.cycle, 1), &settings())
                .is_err()
        );
        assert!(engine.model().workspace.slots.is_empty());
    }

    #[test]
    fn inserting_a_slot_invalidates_pending_work() {
        let mut engine = Engine::default();
        let pending = engine.begin_cycle();
        engine.insert_slot(Slot {
            id: crate::SlotId(1),
            weight: 1,
            binding: crate::BindingState::Bound(PhysicalWindowId {
                app: AppInstanceId {
                    pid: 1,
                    launched_at: 1,
                },
                window_id: 1,
            }),
        });

        assert!(
            engine
                .accept(pending, &observation(pending.cycle, 1), &settings())
                .is_err()
        );
        assert_eq!(engine.model().workspace.slots.len(), 1);
    }
}
