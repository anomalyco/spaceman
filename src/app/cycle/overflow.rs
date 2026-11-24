use super::*;

impl<B: WorkspaceBackend> WorkspaceRuntime<B> {
    /// `[labs] overflow`: once a display's active workspace holds that many
    /// tiles, a window that would open there opens instead on the next
    /// enabled, managed workspace of the same display with room, so a Mac
    /// with many windows (or one display standing in for two) spreads them out
    /// rather than slicing one workspace into unusable tiles. Windows a rule
    /// routes or memory recalls keep their placement; when every workspace of
    /// the display is full the window joins the active one as usual.
    pub(super) fn overflow_full_workspaces(
        &self,
        policy: &mut FrozenAffinityPolicy,
        observation: &crate::NativeObservation,
        visible: &[ResolvedWorkspace],
        outputs: &macos::DisplayOutputs,
    ) {
        let capacity = usize::from(self.config.labs().overflow);
        if capacity == 0 {
            return;
        }
        let mut tiles = HashMap::new();
        for application in &observation.applications {
            let crate::AppObservation::Complete { windows, .. } = application else {
                continue;
            };
            for window in windows {
                if policy.windows.get(&window.id) != Some(&FrozenWindowAffinity::CurrentDisplay)
                    || self.workspaces.workspace_of(&window.id).is_some()
                {
                    continue;
                }
                let Some(active) = visible
                    .iter()
                    .find(|resolved| resolved.output.contains_center(window.frame))
                else {
                    continue;
                };
                if self.tile_count(&mut tiles, active.workspace) < capacity {
                    *tiles.entry(active.workspace).or_default() += 1;
                    continue;
                }
                let Some(target) = self
                    .overflow_candidates(active, &policy.visible_workspaces, outputs)
                    .into_iter()
                    .find(|candidate| self.tile_count(&mut tiles, *candidate) < capacity)
                else {
                    continue;
                };
                *tiles.entry(target).or_default() += 1;
                log_event!(
                    "overflow {:?}: workspace {} is full -> workspace {}",
                    window.id,
                    active.workspace,
                    target
                );
                policy
                    .windows
                    .insert(window.id.clone(), FrozenWindowAffinity::Overflow(target));
            }
        }
    }

    /// The hidden workspaces that share `active`'s display, starting after it
    /// in workspace order and wrapping around.
    fn overflow_candidates(
        &self,
        active: &ResolvedWorkspace,
        visible: &HashSet<WorkspaceId>,
        outputs: &macos::DisplayOutputs,
    ) -> Vec<WorkspaceId> {
        let mut candidates: Vec<_> = WorkspaceId::all()
            .filter(|workspace| {
                let selector = self.workspaces.assignment(*workspace);
                !visible.contains(workspace)
                    && self.workspaces.is_enabled(*workspace)
                    && self.scope.manages_selector(selector)
                    && outputs.resolve(selector).id == active.display_id
            })
            .collect();
        candidates.sort_by_key(|workspace| {
            (
                workspace.index() < active.workspace.index(),
                workspace.index(),
            )
        });
        candidates
    }

    fn tile_count(&self, tiles: &mut HashMap<WorkspaceId, usize>, workspace: WorkspaceId) -> usize {
        *tiles.entry(workspace).or_insert_with(|| {
            self.workspaces
                .engine(workspace)
                .model()
                .workspace
                .slots
                .len()
        })
    }
}
