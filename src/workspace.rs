use std::{
    collections::{HashMap, HashSet},
    fmt,
    str::FromStr,
};

use serde::{Deserialize, Serialize};

use crate::{Engine, PhysicalWindowId};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WorkspaceId(u8);

impl WorkspaceId {
    pub const NUMBERED_COUNT: usize = 10;
    pub const COUNT: usize = Self::NUMBERED_COUNT + 26;
    pub const ONE: Self = Self(1);
    pub const SIX: Self = Self(6);

    pub fn new(value: u8) -> Result<Self, String> {
        (1..=Self::NUMBERED_COUNT as u8)
            .contains(&value)
            .then_some(Self(value))
            .ok_or_else(|| format!("workspace must be 1 through 10 or A through Z, got {value}"))
    }

    pub fn named(letter: char) -> Result<Self, String> {
        let letter = letter.to_ascii_uppercase();
        letter
            .is_ascii_uppercase()
            .then_some(Self(11 + (letter as u8 - b'A')))
            .ok_or_else(|| format!("workspace must be 1 through 10 or A through Z, got {letter:?}"))
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        value.parse::<u8>().ok().map(Self::new).unwrap_or_else(|| {
            let mut characters = value.chars();
            match (characters.next(), characters.next()) {
                (Some(letter), None) => Self::named(letter),
                _ => Err(format!(
                    "workspace must be 1 through 10 or A through Z, got {value:?}"
                )),
            }
        })
    }

    #[must_use]
    pub const fn number(self) -> Option<u8> {
        if self.0 <= Self::NUMBERED_COUNT as u8 {
            Some(self.0)
        } else {
            None
        }
    }

    #[must_use]
    pub const fn letter(self) -> Option<char> {
        if self.0 > Self::NUMBERED_COUNT as u8 {
            Some((b'A' + self.0 - 11) as char)
        } else {
            None
        }
    }

    pub fn all() -> impl Iterator<Item = Self> {
        (0..Self::COUNT).map(Self::from_index)
    }

    pub fn numbered() -> impl Iterator<Item = Self> {
        Self::all().take(Self::NUMBERED_COUNT)
    }

    pub(crate) fn from_index(index: usize) -> Self {
        Self(u8::try_from(index + 1).expect("workspace index fits u8"))
    }

    #[must_use]
    pub(crate) const fn index(self) -> usize {
        (self.0 - 1) as usize
    }
}

impl fmt::Display for WorkspaceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.number() {
            Some(number) => write!(formatter, "{number}"),
            None => write!(
                formatter,
                "{}",
                self.letter().expect("named workspace has a letter")
            ),
        }
    }
}

impl FromStr for WorkspaceId {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl Serialize for WorkspaceId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.number() {
            Some(number) => serializer.serialize_u8(number),
            None => serializer.serialize_char(self.letter().expect("named workspace has a letter")),
        }
    }
}

impl<'de> Deserialize<'de> for WorkspaceId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Value {
            Number(u8),
            Name(String),
        }
        match Value::deserialize(deserializer)? {
            Value::Number(value) => Self::new(value),
            Value::Name(value) => Self::parse(&value),
        }
        .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum DisplaySelector {
    Main,
    Secondary,
    Preferred(String),
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct WorkspaceState {
    active: Vec<(DisplaySelector, WorkspaceId)>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    previous: Vec<(DisplaySelector, WorkspaceId)>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    axes: Vec<crate::Axis>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    layouts: Vec<crate::layout_tree::LayoutTree>,
    slots: Vec<Vec<crate::Slot>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    floating: Vec<(crate::PhysicalWindowId, WorkspaceId)>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    last_focused: Vec<Option<crate::SlotId>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    fullscreen_layout: Vec<bool>,
    /// Legacy per-window maximize intent. A populated entry migrates the
    /// workspace into fullscreen layout mode.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    maximized: Vec<Option<crate::SlotId>>,
}

impl WorkspaceState {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.slots.len() > WorkspaceId::COUNT
            || self.layouts.len() > WorkspaceId::COUNT
            || self.axes.len() > WorkspaceId::COUNT
            || self.last_focused.len() > WorkspaceId::COUNT
            || self.fullscreen_layout.len() > WorkspaceId::COUNT
            || self.maximized.len() > WorkspaceId::COUNT
        {
            return Err("workspace state contains more than 36 workspaces".into());
        }
        for (index, slots) in self.slots.iter().enumerate() {
            let next_slot_id = slots
                .iter()
                .map(|slot| slot.id.0)
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or("workspace slot ID cannot be incremented")?;
            crate::Model {
                workspace: crate::model::Workspace {
                    axis: self.axes.get(index).copied().unwrap_or_default(),
                    slots: slots.clone(),
                },
                continuity: HashMap::new(),
                next_slot_id,
            }
            .validate()
            .map_err(|error| format!("workspace {}: {error}", index + 1))?;
            if let Some(layout) = self.layouts.get(index) {
                layout
                    .validate(slots)
                    .map_err(|error| format!("workspace {}: {error}", index + 1))?;
            }
        }
        if self.layouts.len() > self.slots.len() {
            return Err("workspace layouts have no matching slot state".into());
        }
        for (name, intents) in [
            ("last-focused", &self.last_focused),
            ("maximized", &self.maximized),
        ] {
            for (index, slot) in intents.iter().enumerate() {
                if let Some(slot) = slot
                    && !self
                        .slots
                        .get(index)
                        .is_some_and(|slots| slots.iter().any(|candidate| candidate.id == *slot))
                {
                    return Err(format!(
                        "workspace {} {name} slot {} does not exist",
                        index + 1,
                        slot.0
                    ));
                }
            }
        }
        let mut tiled = HashSet::new();
        for window in self
            .slots
            .iter()
            .flatten()
            .map(crate::Slot::identity_window)
        {
            if !tiled.insert(window) {
                return Err("tiled window is owned by multiple workspaces".into());
            }
        }
        let mut floating = HashSet::new();
        for (window, _) in &self.floating {
            if tiled.contains(window) {
                return Err("window is both tiled and floating".into());
            }
            if !floating.insert(window) {
                return Err("floating window is owned by multiple workspaces".into());
            }
        }
        Ok(())
    }

    pub(crate) fn named_workspaces_in_use(&self) -> HashSet<WorkspaceId> {
        let mut used = self
            .active
            .iter()
            .chain(&self.previous)
            .map(|(_, workspace)| *workspace)
            .chain(self.floating.iter().map(|(_, workspace)| *workspace))
            .filter(|workspace| workspace.letter().is_some())
            .collect::<HashSet<_>>();
        for (index, slots) in self.slots.iter().enumerate() {
            let workspace = WorkspaceId::from_index(index);
            if workspace.letter().is_some() && !slots.is_empty() {
                used.insert(workspace);
            }
        }
        used
    }
}

impl DisplaySelector {
    #[must_use]
    pub fn parse(value: &str) -> Self {
        match value {
            "main" => Self::Main,
            "secondary" => Self::Secondary,
            value => Self::Preferred(value.to_owned()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Workspaces {
    engines: [Engine; WorkspaceId::COUNT],
    assignments: [DisplaySelector; WorkspaceId::COUNT],
    active: HashMap<DisplaySelector, WorkspaceId>,
    previous: HashMap<DisplaySelector, WorkspaceId>,
    floating_owners: HashMap<PhysicalWindowId, WorkspaceId>,
    enabled: HashSet<WorkspaceId>,
    last_focused: [Option<crate::SlotId>; WorkspaceId::COUNT],
    fullscreen_layout: [bool; WorkspaceId::COUNT],
}

impl Default for Workspaces {
    fn default() -> Self {
        Self {
            engines: std::array::from_fn(|_| Engine::default()),
            assignments: std::array::from_fn(|index| {
                if (5..WorkspaceId::NUMBERED_COUNT).contains(&index) {
                    DisplaySelector::Secondary
                } else {
                    DisplaySelector::Main
                }
            }),
            active: HashMap::from([
                (DisplaySelector::Main, WorkspaceId::ONE),
                (DisplaySelector::Secondary, WorkspaceId::SIX),
            ]),
            previous: HashMap::new(),
            floating_owners: HashMap::new(),
            enabled: WorkspaceId::numbered().collect(),
            last_focused: std::array::from_fn(|_| None),
            fullscreen_layout: [false; WorkspaceId::COUNT],
        }
    }
}

impl Workspaces {
    #[must_use]
    pub(crate) fn state(&self) -> WorkspaceState {
        let workspace_count = self
            .engines
            .iter()
            .rposition(|engine| !engine.model().workspace.slots.is_empty())
            .map_or(WorkspaceId::NUMBERED_COUNT, |index| index + 1)
            .max(WorkspaceId::NUMBERED_COUNT);
        WorkspaceState {
            active: sorted(self.active.clone()),
            previous: sorted(self.previous.clone()),
            axes: Vec::new(),
            layouts: self
                .engines
                .iter()
                .take(workspace_count)
                .map(|engine| engine.layout().clone())
                .collect(),
            slots: self
                .engines
                .iter()
                .take(workspace_count)
                .map(|engine| engine.model().workspace.slots.clone())
                .collect(),
            floating: sorted(self.floating_owners.clone()),
            last_focused: trimmed(&self.last_focused, Option::is_some),
            fullscreen_layout: trimmed(&self.fullscreen_layout, |enabled| *enabled),
            maximized: Vec::new(),
        }
    }

    pub(crate) fn restore(&mut self, state: WorkspaceState) {
        self.enabled.extend(state.named_workspaces_in_use());
        let WorkspaceState {
            active,
            previous,
            axes,
            layouts,
            slots,
            floating,
            last_focused,
            fullscreen_layout,
            maximized,
        } = state;
        for (index, slots) in slots.into_iter().take(WorkspaceId::COUNT).enumerate() {
            let workspace = WorkspaceId::from_index(index);
            for mut slot in slots {
                if let crate::BindingState::Unresolved { since, .. } = &mut slot.binding {
                    // Reconciliation cycles are process-local, so restart the
                    // bounded settling window when restoring persisted state.
                    *since = 0;
                }
                self.engine_mut(workspace).insert_slot(slot);
            }
            let layout = layouts.get(index).cloned().unwrap_or_else(|| {
                crate::layout_tree::LayoutTree::weighted_flat(
                    axes.get(index).copied().unwrap_or_default(),
                    self.engine(workspace).model().workspace.slots.iter(),
                )
            });
            self.engine_mut(workspace).restore_layout(layout);
        }
        for (selector, workspace) in active {
            if self.assignment(workspace) == &selector {
                self.active.insert(selector, workspace);
            }
        }
        for (selector, workspace) in previous {
            if self.assignment(workspace) == &selector
                && self.active_for(&selector) != Some(workspace)
            {
                self.previous.insert(selector, workspace);
            }
        }
        for (window, workspace) in floating {
            if self.workspace_of(&window).is_none() {
                self.floating_owners.insert(window, workspace);
            }
        }
        for index in 0..WorkspaceId::COUNT {
            let workspace = WorkspaceId::from_index(index);
            let valid_slot = |slots: &[Option<crate::SlotId>]| {
                slots
                    .get(index)
                    .copied()
                    .flatten()
                    .filter(|slot| self.has_slot(workspace, *slot))
            };
            let legacy_maximized = valid_slot(&maximized);
            self.last_focused[index] = legacy_maximized.or_else(|| valid_slot(&last_focused));
            self.set_fullscreen_layout(
                workspace,
                fullscreen_layout.get(index).copied().unwrap_or(false)
                    || legacy_maximized.is_some(),
            );
        }
    }

    #[must_use]
    pub fn assignment(&self, workspace: WorkspaceId) -> &DisplaySelector {
        &self.assignments[workspace.index()]
    }

    #[must_use]
    pub fn active_for(&self, display: &DisplaySelector) -> Option<WorkspaceId> {
        self.active.get(display).copied()
    }

    pub fn active_lanes(&self) -> impl Iterator<Item = (&DisplaySelector, WorkspaceId)> {
        self.active
            .iter()
            .map(|(display, workspace)| (display, *workspace))
    }

    pub fn set_assignment(&mut self, workspace: WorkspaceId, display: DisplaySelector) {
        self.enable(workspace);
        let previous = self.assignments[workspace.index()].clone();
        if previous == display {
            return;
        }
        self.assignments[workspace.index()] = display.clone();
        self.active.entry(display).or_insert(workspace);
        if self.active.get(&previous) == Some(&workspace) {
            if let Some(replacement) = WorkspaceId::all().find(|candidate| {
                self.is_enabled(*candidate) && self.assignment(*candidate) == &previous
            }) {
                self.active.insert(previous, replacement);
            } else {
                self.active.remove(&previous);
            }
        }
    }

    pub fn enable(&mut self, workspace: WorkspaceId) {
        self.enabled.insert(workspace);
    }

    #[must_use]
    pub fn is_enabled(&self, workspace: WorkspaceId) -> bool {
        self.enabled.contains(&workspace)
    }

    pub fn switch(&mut self, workspace: WorkspaceId) -> WorkspaceId {
        let display = self.assignment(workspace).clone();
        let active = self.active_for(&display).unwrap_or(workspace);
        if active != workspace {
            self.previous.insert(display.clone(), active);
            self.active.insert(display, workspace);
        }
        active
    }

    #[must_use]
    pub fn previous_for(&self, display: &DisplaySelector) -> Option<WorkspaceId> {
        self.previous.get(display).copied()
    }

    #[must_use]
    pub fn engine(&self, workspace: WorkspaceId) -> &Engine {
        &self.engines[workspace.index()]
    }

    pub fn engine_mut(&mut self, workspace: WorkspaceId) -> &mut Engine {
        &mut self.engines[workspace.index()]
    }

    pub fn move_window(
        &mut self,
        window: &PhysicalWindowId,
        target: WorkspaceId,
    ) -> Option<WorkspaceId> {
        let source = self.workspace_of(window)?;
        let source_index = source.index();
        let target_index = target.index();
        if source_index == target_index {
            return None;
        }
        let (source_engine, target_engine) = if source_index < target_index {
            let (before, after) = self.engines.split_at_mut(target_index);
            (&mut before[source_index], &mut after[0])
        } else {
            let (before, after) = self.engines.split_at_mut(source_index);
            (&mut after[0], &mut before[target_index])
        };
        let slot = source_engine.take_slot(window)?;
        target_engine.insert_slot(slot);
        self.prune_intents(source);
        Some(source)
    }

    #[must_use]
    pub fn workspace_of(&self, window: &PhysicalWindowId) -> Option<WorkspaceId> {
        self.engines
            .iter()
            .position(|engine| {
                engine
                    .model()
                    .workspace
                    .slots
                    .iter()
                    .any(|slot| slot.identity_window() == window)
            })
            .map(WorkspaceId::from_index)
    }

    #[must_use]
    pub(crate) fn floating_workspace_of(&self, window: &PhysicalWindowId) -> Option<WorkspaceId> {
        self.floating_owners.get(window).copied()
    }

    #[must_use]
    pub(crate) fn workspace_of_any(&self, window: &PhysicalWindowId) -> Option<WorkspaceId> {
        self.workspace_of(window)
            .or_else(|| self.floating_workspace_of(window))
    }

    pub(crate) fn own_floating(&mut self, window: PhysicalWindowId, workspace: WorkspaceId) {
        self.floating_owners.insert(window, workspace);
    }

    pub(crate) fn windows(&self, workspace: WorkspaceId) -> Vec<PhysicalWindowId> {
        let mut windows = self.engine(workspace).bound_windows();
        windows.extend(
            self.floating_entries()
                .filter(|(_, owner)| *owner == workspace)
                .map(|(window, _)| window.clone()),
        );
        windows
    }

    pub(crate) fn app_instances(
        &self,
        workspace: WorkspaceId,
    ) -> impl Iterator<Item = &crate::AppInstanceId> {
        self.engine(workspace)
            .model()
            .workspace
            .slots
            .iter()
            .map(|slot| &slot.identity_window().app)
    }

    pub(crate) fn app_owns_windows(&self, app: &crate::AppInstanceId) -> bool {
        WorkspaceId::all().any(|workspace| self.app_instances(workspace).any(|owner| owner == app))
            || self.floating_owners.keys().any(|window| &window.app == app)
    }

    pub(crate) fn forget_app(
        &mut self,
        workspace: WorkspaceId,
        app: &crate::AppInstanceId,
    ) -> bool {
        let removed = self.engine_mut(workspace).forget_app(app);
        if removed {
            self.prune_intents(workspace);
        }
        removed
    }

    pub(crate) fn floating_windows_for_app(
        &self,
        app: &crate::AppInstanceId,
    ) -> Vec<PhysicalWindowId> {
        self.floating_entries()
            .filter(|(window, _)| &window.app == app)
            .map(|(window, _)| window.clone())
            .collect()
    }

    pub(crate) fn floating_entries(
        &self,
    ) -> impl Iterator<Item = (&PhysicalWindowId, WorkspaceId)> {
        self.floating_owners
            .iter()
            .map(|(window, workspace)| (window, *workspace))
    }

    pub(crate) fn forget_floating(&mut self, window: &PhysicalWindowId) {
        self.floating_owners.remove(window);
    }

    #[must_use]
    pub fn workspace_for_app(&self, app: &crate::AppInstanceId) -> Option<WorkspaceId> {
        let mut matches = WorkspaceId::all().filter(|workspace| {
            self.is_enabled(*workspace)
                && self
                    .engine(*workspace)
                    .model()
                    .workspace
                    .slots
                    .iter()
                    .any(|slot| slot.belongs_to(app))
        });
        let workspace = matches.next()?;
        matches.next().is_none().then_some(workspace)
    }

    /// Candidate evidence is reserved, not owned: only reconciliation may bind
    /// it. More than one reserving workspace means adoption must be deferred.
    pub(crate) fn replacement_workspaces<'a>(
        &'a self,
        window: &'a PhysicalWindowId,
    ) -> impl Iterator<Item = WorkspaceId> + 'a {
        WorkspaceId::all().filter(|workspace| {
            self.engine(*workspace)
                .model()
                .workspace
                .slots
                .iter()
                .any(|slot| {
                    matches!(&slot.binding, crate::BindingState::Unresolved { candidates, .. }
                    if candidates.contains(window))
                })
        })
    }

    pub fn forget_window(&mut self, workspace: WorkspaceId, window: &PhysicalWindowId) {
        self.engine_mut(workspace).take_slot(window);
        self.prune_intents(workspace);
    }

    pub(crate) fn remember_focus(&mut self, workspace: WorkspaceId, window: &PhysicalWindowId) {
        self.last_focused[workspace.index()] = self.engine(workspace).slot_for_window(window);
    }

    pub(crate) fn preferred_focus(&self, workspace: WorkspaceId) -> Option<PhysicalWindowId> {
        let slot = self.last_focused[workspace.index()]?;
        self.engine(workspace)
            .model()
            .slot(slot)?
            .bound_window()
            .cloned()
    }

    pub(crate) fn uses_fullscreen_layout(&self, workspace: WorkspaceId) -> bool {
        self.fullscreen_layout[workspace.index()]
    }

    pub(crate) fn set_fullscreen_layout(&mut self, workspace: WorkspaceId, enabled: bool) {
        self.fullscreen_layout[workspace.index()] =
            enabled && !self.engine(workspace).model().workspace.slots.is_empty();
    }

    pub(crate) fn prune_intents(&mut self, workspace: WorkspaceId) {
        let index = workspace.index();
        self.last_focused[index] =
            self.last_focused[index].filter(|slot| self.has_slot(workspace, *slot));
        self.set_fullscreen_layout(workspace, self.fullscreen_layout[index]);
    }

    fn has_slot(&self, workspace: WorkspaceId, slot: crate::SlotId) -> bool {
        self.engine(workspace).model().slot(slot).is_some()
    }

    pub(crate) fn inactive_windows_except(
        &self,
        visible: &HashSet<WorkspaceId>,
    ) -> Vec<(WorkspaceId, PhysicalWindowId)> {
        WorkspaceId::all()
            .filter(|workspace| self.is_enabled(*workspace) && !visible.contains(workspace))
            .flat_map(|workspace| {
                self.windows(workspace)
                    .into_iter()
                    .map(move |window| (workspace, window))
            })
            .collect()
    }
}

fn sorted<T: Ord>(items: impl IntoIterator<Item = T>) -> Vec<T> {
    let mut items: Vec<_> = items.into_iter().collect();
    items.sort();
    items
}

fn trimmed<T: Clone>(items: &[T; WorkspaceId::COUNT], keep: impl Fn(&T) -> bool) -> Vec<T> {
    let length = items.iter().rposition(keep).map_or(0, |index| index + 1);
    items[..length].to_vec()
}

#[cfg(test)]
mod tests {
    use crate::{AppInstanceId, BindingState, Slot, SlotId};

    use super::*;

    fn window(id: u32) -> PhysicalWindowId {
        PhysicalWindowId {
            app: AppInstanceId {
                pid: 1,
                launched_at: 1,
            },
            window_id: id,
        }
    }

    #[test]
    fn unresolved_affinity_reserves_candidates_without_owning_them() {
        let old = window(10);
        let replacement = window(11);
        let other = window(20);
        let mut workspaces = Workspaces::default();
        workspaces.engine_mut(WorkspaceId::ONE).insert_slot(Slot {
            id: SlotId(1),
            weight: 1,
            binding: BindingState::Unresolved {
                previous: old.clone(),
                candidates: vec![replacement.clone()],
                since: 1,
            },
        });
        assert_eq!(
            workspaces.workspace_for_app(&old.app),
            Some(WorkspaceId::ONE)
        );
        assert_eq!(workspaces.workspace_of(&replacement), None);
        assert_eq!(
            workspaces
                .replacement_workspaces(&replacement)
                .collect::<Vec<_>>(),
            vec![WorkspaceId::ONE]
        );
        workspaces.engine_mut(WorkspaceId::SIX).insert_slot(Slot {
            id: SlotId(1),
            weight: 1,
            binding: BindingState::Bound(other),
        });
        assert_eq!(workspaces.workspace_for_app(&old.app), None);
        assert!(workspaces.state().validate().is_ok());
    }

    #[test]
    fn workspace_range_and_default_assignment_match_aerospace() {
        assert!(WorkspaceId::new(0).is_err());
        assert!(WorkspaceId::new(11).is_err());
        assert_eq!(
            WorkspaceId::parse("d").unwrap(),
            WorkspaceId::named('D').unwrap()
        );
        assert_eq!(
            Workspaces::default().assignment(WorkspaceId::new(5).unwrap()),
            &DisplaySelector::Main
        );
        assert_eq!(
            Workspaces::default().assignment(WorkspaceId::new(6).unwrap()),
            &DisplaySelector::Secondary
        );
        assert!(!Workspaces::default().is_enabled(WorkspaceId::named('D').unwrap()));
        assert_eq!(
            Workspaces::default().state().slots.len(),
            WorkspaceId::NUMBERED_COUNT
        );
    }

    #[test]
    fn workspace_identity_serializes_numbers_compatibly_and_names_as_letters() {
        assert_eq!(
            serde_json::to_value(WorkspaceId::ONE).unwrap(),
            serde_json::json!(1)
        );
        assert_eq!(
            serde_json::to_value(WorkspaceId::named('D').unwrap()).unwrap(),
            serde_json::json!("D")
        );
        assert_eq!(
            serde_json::from_value::<WorkspaceId>(serde_json::json!("D")).unwrap(),
            WorkspaceId::named('D').unwrap()
        );
    }

    #[test]
    fn switching_one_display_preserves_the_other() {
        let mut workspaces = Workspaces::default();
        workspaces.switch(WorkspaceId::new(3).unwrap());
        assert_eq!(
            workspaces.active_for(&DisplaySelector::Main),
            WorkspaceId::new(3).ok()
        );
        assert_eq!(
            workspaces.active_for(&DisplaySelector::Secondary),
            Some(WorkspaceId::SIX)
        );
    }

    #[test]
    fn switching_to_a_named_workspace_preserves_the_other_display() {
        let mut workspaces = Workspaces::default();
        let named = WorkspaceId::named('D').unwrap();
        workspaces.enable(named);

        workspaces.switch(named);

        assert_eq!(workspaces.active_for(&DisplaySelector::Main), Some(named));
        assert_eq!(
            workspaces.active_for(&DisplaySelector::Secondary),
            Some(WorkspaceId::SIX)
        );
    }

    #[test]
    fn unconfigured_named_workspaces_are_not_display_fallbacks() {
        let mut workspaces = Workspaces::default();
        for workspace in WorkspaceId::numbered().take(5) {
            workspaces.set_assignment(workspace, DisplaySelector::Secondary);
        }

        assert_eq!(workspaces.active_for(&DisplaySelector::Main), None);
    }

    #[test]
    fn back_and_forth_history_is_independent_per_display() {
        let mut workspaces = Workspaces::default();
        let main_target = WorkspaceId::new(3).unwrap();
        let secondary_target = WorkspaceId::new(8).unwrap();

        workspaces.switch(main_target);
        workspaces.switch(secondary_target);

        assert_eq!(
            workspaces.previous_for(&DisplaySelector::Main),
            Some(WorkspaceId::ONE)
        );
        assert_eq!(
            workspaces.previous_for(&DisplaySelector::Secondary),
            Some(WorkspaceId::SIX)
        );
        workspaces.switch(WorkspaceId::ONE);
        assert_eq!(
            workspaces.previous_for(&DisplaySelector::Main),
            Some(main_target)
        );
        assert_eq!(
            workspaces.previous_for(&DisplaySelector::Secondary),
            Some(WorkspaceId::SIX)
        );
    }

    #[test]
    fn switching_to_the_active_workspace_preserves_history() {
        let mut workspaces = Workspaces::default();
        let target = WorkspaceId::new(3).unwrap();
        workspaces.switch(target);

        workspaces.switch(target);

        assert_eq!(
            workspaces.previous_for(&DisplaySelector::Main),
            Some(WorkspaceId::ONE)
        );
    }

    #[test]
    fn persisted_state_restores_active_workspaces_and_tile_order() {
        let first = window(10);
        let second = window(20);
        let mut workspaces = Workspaces::default();
        workspaces.engine_mut(WorkspaceId::ONE).insert_slot(Slot {
            id: SlotId(1),
            weight: 1,
            binding: BindingState::Bound(first.clone()),
        });
        workspaces.engine_mut(WorkspaceId::ONE).insert_slot(Slot {
            id: SlotId(2),
            weight: 1,
            binding: BindingState::Bound(second.clone()),
        });
        workspaces.engine_mut(WorkspaceId::ONE).toggle_axis();
        let third = WorkspaceId::new(3).unwrap();
        workspaces.switch(third);

        let encoded = serde_json::to_vec(&workspaces.state()).unwrap();
        let state = serde_json::from_slice(&encoded).unwrap();
        let mut restored = Workspaces::default();
        restored.restore(state);

        assert_eq!(restored.active_for(&DisplaySelector::Main), Some(third));
        assert_eq!(
            restored.previous_for(&DisplaySelector::Main),
            Some(WorkspaceId::ONE)
        );
        assert_eq!(
            restored.engine(WorkspaceId::ONE).model().workspace.axis,
            crate::Axis::Vertical
        );
        assert_eq!(
            restored.engine(WorkspaceId::ONE).bound_windows(),
            vec![first.clone(), second]
        );
        assert_eq!(restored.workspace_of(&first), Some(WorkspaceId::ONE));
    }

    #[test]
    fn persisted_state_restores_focus_and_fullscreen_layout() {
        let first = window(10);
        let second = window(20);
        let mut workspaces = Workspaces::default();
        for (id, window) in [(1, &first), (2, &second)] {
            workspaces.engine_mut(WorkspaceId::ONE).insert_slot(Slot {
                id: SlotId(id),
                weight: 1,
                binding: BindingState::Bound(window.clone()),
            });
        }
        workspaces.remember_focus(WorkspaceId::ONE, &second);
        workspaces.set_fullscreen_layout(WorkspaceId::ONE, true);

        let encoded = serde_json::to_value(workspaces.state()).unwrap();
        assert_eq!(encoded["fullscreen_layout"], serde_json::json!([true]));
        assert!(encoded.get("maximized").is_none());
        let state: WorkspaceState = serde_json::from_value(encoded).unwrap();
        state.validate().unwrap();
        let mut restored = Workspaces::default();
        restored.restore(state);

        assert_eq!(
            restored.preferred_focus(WorkspaceId::ONE),
            Some(second.clone())
        );
        assert!(restored.uses_fullscreen_layout(WorkspaceId::ONE));
    }

    #[test]
    fn persisted_empty_workspace_cannot_restore_fullscreen_layout() {
        let mut state = Workspaces::default().state();
        state.fullscreen_layout = vec![true];
        let mut restored = Workspaces::default();

        restored.restore(state);

        assert!(!restored.uses_fullscreen_layout(WorkspaceId::ONE));
    }

    #[test]
    fn fullscreen_layout_survives_until_the_final_slot_is_removed() {
        let first = window(10);
        let second = window(20);
        let mut workspaces = Workspaces::default();
        for (id, window) in [(1, &first), (2, &second)] {
            workspaces.engine_mut(WorkspaceId::ONE).insert_slot(Slot {
                id: SlotId(id),
                weight: 1,
                binding: BindingState::Bound(window.clone()),
            });
        }
        workspaces.set_fullscreen_layout(WorkspaceId::ONE, true);

        workspaces.forget_window(WorkspaceId::ONE, &first);
        assert!(workspaces.uses_fullscreen_layout(WorkspaceId::ONE));
        workspaces.forget_window(WorkspaceId::ONE, &second);

        assert!(!workspaces.uses_fullscreen_layout(WorkspaceId::ONE));
    }

    #[test]
    fn legacy_maximize_intent_migrates_to_fullscreen_layout() {
        let first = window(10);
        let state = serde_json::json!({
            "active": [],
            "slots": [[
                {
                    "id": 7,
                    "weight": 1,
                    "binding": {
                        "state": "bound",
                        "app": { "pid": 1, "launched_at": 1 },
                        "window_id": 10
                    }
                },
                {
                    "id": 8,
                    "weight": 1,
                    "binding": {
                        "state": "bound",
                        "app": { "pid": 1, "launched_at": 1 },
                        "window_id": 20
                    }
                }
            ]],
            "last_focused": [8],
            "maximized": [7]
        });
        let mut workspaces = Workspaces::default();

        workspaces.restore(serde_json::from_value(state).unwrap());

        assert!(workspaces.uses_fullscreen_layout(WorkspaceId::ONE));
        assert_eq!(workspaces.preferred_focus(WorkspaceId::ONE), Some(first));
    }

    #[test]
    fn persisted_state_round_trips_a_named_workspace() {
        let named = WorkspaceId::named('D').unwrap();
        let owned = window(40);
        let mut workspaces = Workspaces::default();
        workspaces.enable(named);
        workspaces.engine_mut(named).insert_slot(Slot {
            id: SlotId(1),
            weight: 1,
            binding: BindingState::Bound(owned.clone()),
        });
        workspaces.switch(named);

        let encoded = serde_json::to_value(workspaces.state()).unwrap();
        let state: WorkspaceState = serde_json::from_value(encoded).unwrap();
        state.validate().unwrap();
        let mut restored = Workspaces::default();
        restored.restore(state);

        assert_eq!(restored.active_for(&DisplaySelector::Main), Some(named));
        assert_eq!(restored.workspace_of(&owned), Some(named));
        assert!(restored.is_enabled(named));
    }

    #[test]
    fn persisted_state_restores_floating_window_ownership() {
        let floating = window(30);
        let workspace = WorkspaceId::new(3).unwrap();
        let mut workspaces = Workspaces::default();
        workspaces.own_floating(floating.clone(), workspace);

        let encoded = serde_json::to_vec(&workspaces.state()).unwrap();
        let state: WorkspaceState = serde_json::from_slice(&encoded).unwrap();
        state.validate().unwrap();
        let mut restored = Workspaces::default();
        restored.restore(state);

        assert_eq!(restored.floating_workspace_of(&floating), Some(workspace));
        assert_eq!(restored.workspace_of(&floating), None);
    }

    #[test]
    fn nested_layout_round_trips_without_flattening() {
        let first = window(10);
        let second = window(20);
        let third = window(30);
        let mut workspaces = Workspaces::default();
        for (id, window) in [(1, &first), (2, &second), (3, &third)] {
            workspaces.engine_mut(WorkspaceId::ONE).insert_slot(Slot {
                id: SlotId(id),
                weight: 1,
                binding: BindingState::Bound(window.clone()),
            });
        }
        assert!(
            workspaces
                .engine_mut(WorkspaceId::ONE)
                .create_implicit_split(&second, crate::Direction::Up)
        );

        let expected = workspaces.engine(WorkspaceId::ONE).preview(crate::Rect {
            x: 0,
            y: 0,
            width: 1200,
            height: 800,
        });
        let encoded = serde_json::to_value(workspaces.state()).unwrap();
        assert!(encoded.get("layouts").is_some());
        assert!(encoded.get("axes").is_none());
        let mut restored = Workspaces::default();
        restored.restore(serde_json::from_value(encoded).unwrap());

        assert_eq!(
            restored.engine(WorkspaceId::ONE).preview(crate::Rect {
                x: 0,
                y: 0,
                width: 1200,
                height: 800,
            }),
            expected
        );
    }

    #[test]
    fn persisted_state_rejects_layout_with_wrong_slot_membership() {
        let mut state = Workspaces::default().state();
        state.slots[0].push(Slot {
            id: SlotId(1),
            weight: 1,
            binding: BindingState::Bound(window(10)),
        });

        assert!(state.validate().is_err());
    }

    #[test]
    fn persisted_state_rejects_zero_weight_slots() {
        let mut state = Workspaces::default().state();
        state.slots[0].push(Slot {
            id: SlotId(1),
            weight: 0,
            binding: BindingState::Bound(window(10)),
        });
        state.layouts[0] =
            crate::layout_tree::LayoutTree::flat(crate::Axis::Horizontal, [SlotId(1)]);

        assert!(state.validate().is_err());
    }

    #[test]
    fn persisted_state_rejects_a_window_owned_by_multiple_workspaces() {
        let mut state = Workspaces::default().state();
        state.slots[0].push(Slot {
            id: SlotId(1),
            weight: 1,
            binding: BindingState::Bound(window(10)),
        });
        state.slots[1].push(Slot {
            id: SlotId(1),
            weight: 1,
            binding: BindingState::Unresolved {
                previous: window(10),
                candidates: vec![window(20)],
                since: 1,
            },
        });
        for index in [0, 1] {
            state.layouts[index] =
                crate::layout_tree::LayoutTree::flat(crate::Axis::Horizontal, [SlotId(1)]);
        }

        assert_eq!(
            state.validate().unwrap_err(),
            "tiled window is owned by multiple workspaces"
        );
    }

    #[test]
    fn restoring_unresolved_state_restarts_its_settling_window() {
        let mut state = Workspaces::default().state();
        state.slots[0].push(Slot {
            id: SlotId(1),
            weight: 1,
            binding: BindingState::Unresolved {
                previous: window(10),
                candidates: vec![window(20)],
                since: 10_000,
            },
        });
        state.layouts[0] =
            crate::layout_tree::LayoutTree::flat(crate::Axis::Horizontal, [SlotId(1)]);
        let mut workspaces = Workspaces::default();

        workspaces.restore(state);

        assert!(matches!(
            workspaces.engine(WorkspaceId::ONE).model().workspace.slots[0].binding,
            BindingState::Unresolved { since: 0, .. }
        ));
    }

    #[test]
    fn persisted_state_rejects_intent_for_a_missing_slot() {
        let mut state = Workspaces::default().state();
        state.maximized = vec![Some(SlotId(99))];

        assert!(state.validate().is_err());
    }

    #[test]
    fn legacy_flat_state_migrates_to_a_root_split() {
        let first = window(10);
        let state = serde_json::json!({
            "active": [],
            "axes": ["vertical"],
            "slots": [[{
                "id": 7,
                "weight": 3,
                "binding": {
                    "state": "bound",
                    "app": { "pid": 1, "launched_at": 1 },
                    "window_id": 10
                }
            }]]
        });
        let mut workspaces = Workspaces::default();

        workspaces.restore(serde_json::from_value(state).unwrap());

        assert_eq!(
            workspaces.engine(WorkspaceId::ONE).root_axis(),
            crate::Axis::Vertical
        );
        assert_eq!(
            workspaces.engine(WorkspaceId::ONE).bound_windows(),
            vec![first]
        );
    }

    #[test]
    fn preferred_monitor_creates_an_independent_active_lane() {
        let mut workspaces = Workspaces::default();
        let ninth = WorkspaceId::new(9).unwrap();

        workspaces.set_assignment(ninth, DisplaySelector::Preferred("third-id".into()));

        assert_eq!(
            workspaces.active_for(&DisplaySelector::Main),
            Some(WorkspaceId::ONE)
        );
        assert_eq!(
            workspaces.active_for(&DisplaySelector::Secondary),
            Some(WorkspaceId::SIX)
        );
        assert_eq!(
            workspaces.active_for(&DisplaySelector::Preferred("third-id".into())),
            Some(ninth)
        );
    }

    #[test]
    fn moving_preserves_the_logical_slot() {
        let mut workspaces = Workspaces::default();
        let window = window(10);
        let slot = Slot {
            id: SlotId(7),
            weight: 3,
            binding: BindingState::Bound(window.clone()),
        };
        workspaces.engine_mut(WorkspaceId::ONE).insert_slot(slot);
        workspaces.set_fullscreen_layout(WorkspaceId::ONE, true);

        assert_eq!(
            workspaces.move_window(&window, WorkspaceId::new(3).unwrap()),
            Some(WorkspaceId::ONE)
        );
        assert!(
            workspaces
                .engine(WorkspaceId::ONE)
                .model()
                .workspace
                .slots
                .is_empty()
        );
        assert!(!workspaces.uses_fullscreen_layout(WorkspaceId::ONE));
        let moved = &workspaces
            .engine(WorkspaceId::new(3).unwrap())
            .model()
            .workspace
            .slots[0];
        assert_eq!((moved.id, moved.weight), (SlotId(7), 3));
        assert_eq!(moved.bound_window(), Some(&window));
    }

    #[test]
    fn moving_into_a_populated_workspace_avoids_slot_id_collisions() {
        let mut workspaces = Workspaces::default();
        let source = window(10);
        let existing = window(20);
        workspaces.engine_mut(WorkspaceId::ONE).insert_slot(Slot {
            id: SlotId(1),
            weight: 1,
            binding: BindingState::Bound(source.clone()),
        });
        let target = WorkspaceId::new(2).unwrap();
        workspaces.engine_mut(target).insert_slot(Slot {
            id: SlotId(1),
            weight: 1,
            binding: BindingState::Bound(existing),
        });

        workspaces.move_window(&source, target).unwrap();
        let ids: std::collections::HashSet<_> = workspaces
            .engine(target)
            .model()
            .workspace
            .slots
            .iter()
            .map(|slot| slot.id)
            .collect();
        assert_eq!(ids.len(), 2);
        workspaces.engine(target).model().validate().unwrap();
    }
}
