//! Durable, title-free memory of where each application's windows lived.
//!
//! Physical window identity (process, launch time, window number) does not
//! survive an application relaunch or a reboot, so it cannot carry workspace
//! ownership across one. This memory keys on the bundle identifier instead and
//! records, per application, each managed window's workspace, tiling kind, and
//! last on-screen frame. A relaunched application's new windows claim those
//! placements in order: exact frame matches first, then window creation order.
//!
//! Normally only a relaunch (an update or restart) recalls these placements;
//! an app the user opens mid-session opens where they are. Once the user moves one of an
//! app's windows back by hand right after Spaceman opened it on the active
//! workspace, that app is marked returning and every new instance of it
//! recalls its placements, however long it was closed.
//!
//! The memory is advisory. It never proves a window exists, never moves a
//! window on its own, and an invalid file is ignored rather than blocking
//! recovery.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{Rect, WorkspaceId};

/// Applications remembered at once; the least recently changed is evicted.
pub const MAX_APPS: usize = 128;
/// Windows remembered per application.
pub const MAX_WINDOWS_PER_APP: usize = 32;
const MAX_BUNDLE_ID_LEN: usize = 255;
/// Applications restore saved frames with small adjustments (title bars,
/// display rounding), so a remembered frame matches within this tolerance.
pub const FRAME_MATCH_TOLERANCE: i32 = 8;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RememberedWindow {
    pub workspace: WorkspaceId,
    pub frame: Rect,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub floating: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RememberedApp {
    /// Unix seconds when these placements last changed; drives eviction.
    pub updated_at: u64,
    /// Ordered by window creation (window-server ID) when recorded.
    pub windows: Vec<RememberedWindow>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct WindowMemory {
    #[serde(default)]
    apps: BTreeMap<String, RememberedApp>,
    /// Bundles whose new instances always recall their placements.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    returning: BTreeSet<String>,
}

impl WindowMemory {
    pub fn validate(&self) -> Result<(), String> {
        if self.apps.len() > MAX_APPS {
            return Err(format!(
                "window memory holds more than {MAX_APPS} applications"
            ));
        }
        for (bundle_id, app) in &self.apps {
            if bundle_id.is_empty() || bundle_id.len() > MAX_BUNDLE_ID_LEN {
                return Err("window memory has an invalid bundle identifier".into());
            }
            if app.windows.is_empty() || app.windows.len() > MAX_WINDOWS_PER_APP {
                return Err(format!(
                    "window memory for {bundle_id} must hold 1 through {MAX_WINDOWS_PER_APP} windows"
                ));
            }
        }
        if self.returning.len() > MAX_APPS {
            return Err(format!(
                "window memory marks more than {MAX_APPS} applications as returning"
            ));
        }
        if self
            .returning
            .iter()
            .any(|bundle_id| bundle_id.is_empty() || bundle_id.len() > MAX_BUNDLE_ID_LEN)
        {
            return Err("window memory has an invalid returning bundle identifier".into());
        }
        Ok(())
    }

    /// Whether every new instance of `bundle_id` recalls its placements.
    #[must_use]
    pub fn returns(&self, bundle_id: &str) -> bool {
        self.returning.contains(bundle_id)
    }

    /// Mark `bundle_id` as returning. Returns whether the memory changed.
    pub fn mark_returning(&mut self, bundle_id: &str) -> bool {
        if bundle_id.is_empty()
            || bundle_id.len() > MAX_BUNDLE_ID_LEN
            || self.returning.contains(bundle_id)
        {
            return false;
        }
        if self.returning.len() >= MAX_APPS {
            let Some(stale) = self
                .returning
                .iter()
                .find(|candidate| !self.apps.contains_key(*candidate))
                .cloned()
            else {
                return false;
            };
            self.returning.remove(&stale);
        }
        self.returning.insert(bundle_id.to_owned())
    }

    #[must_use]
    pub fn get(&self, bundle_id: &str) -> Option<&RememberedApp> {
        self.apps.get(bundle_id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.apps.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.apps.is_empty()
    }

    /// Replace an application's remembered placements. Returns whether the
    /// memory changed. Empty placements are ignored: an application with no
    /// managed windows keeps its previous memory for its next launch.
    pub fn record(
        &mut self,
        bundle_id: &str,
        mut windows: Vec<RememberedWindow>,
        now_secs: u64,
    ) -> bool {
        if bundle_id.is_empty() || bundle_id.len() > MAX_BUNDLE_ID_LEN || windows.is_empty() {
            return false;
        }
        windows.truncate(MAX_WINDOWS_PER_APP);
        if self
            .apps
            .get(bundle_id)
            .is_some_and(|app| app.windows == windows)
        {
            return false;
        }
        self.apps.insert(
            bundle_id.to_owned(),
            RememberedApp {
                updated_at: now_secs,
                windows,
            },
        );
        while self.apps.len() > MAX_APPS {
            let Some(oldest) = self
                .apps
                .iter()
                .filter(|(candidate, _)| candidate.as_str() != bundle_id)
                .min_by_key(|(_, app)| app.updated_at)
                .map(|(candidate, _)| candidate.clone())
            else {
                break;
            };
            self.apps.remove(&oldest);
            self.returning.remove(&oldest);
        }
        true
    }
}

/// Assign remembered placements to newly observed windows.
///
/// `windows` are `(window, frame)` pairs whose keys order by creation. Each
/// remembered placement is claimed at most once: first by a window whose frame
/// matches it within [`FRAME_MATCH_TOLERANCE`], then in creation order by the
/// remaining windows. Returns `(window, index into remembered)` pairs; windows
/// beyond the remembered count receive nothing.
#[must_use]
pub fn assign<K: Ord + Clone>(
    remembered: &[RememberedWindow],
    windows: &[(K, Rect)],
) -> Vec<(K, usize)> {
    let mut ordered = windows.to_vec();
    ordered.sort_by(|(left, _), (right, _)| left.cmp(right));
    let mut claimed = vec![false; remembered.len()];
    let mut assignments = Vec::new();
    let mut unmatched = Vec::new();
    for (id, frame) in ordered {
        let exact = claimed
            .iter()
            .zip(remembered)
            .position(|(&is_claimed, candidate)| {
                !is_claimed
                    && candidate
                        .frame
                        .approximately_equals(frame, FRAME_MATCH_TOLERANCE)
            });
        match exact {
            Some(index) => {
                claimed[index] = true;
                assignments.push((id, index));
            }
            None => unmatched.push(id),
        }
    }
    for id in unmatched {
        let Some(index) = claimed.iter().position(|claimed| !claimed) else {
            break;
        };
        claimed[index] = true;
        assignments.push((id, index));
    }
    assignments.sort_by(|(left, _), (right, _)| left.cmp(right));
    assignments
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(x: i32) -> Rect {
        Rect {
            x,
            y: 0,
            width: 400,
            height: 300,
        }
    }

    fn placed(workspace: u8, x: i32) -> RememberedWindow {
        RememberedWindow {
            workspace: WorkspaceId::new(workspace).unwrap(),
            frame: frame(x),
            floating: false,
        }
    }

    #[test]
    fn frame_matches_win_before_creation_order() {
        let remembered = [placed(1, 0), placed(3, 500), placed(5, 900)];
        // Windows were recreated in a different order: 20 at the second
        // frame, 30 at the first, 10 somewhere new.
        let assignments = assign(
            &remembered,
            &[(20, frame(502)), (30, frame(0)), (10, frame(1500))],
        );
        assert_eq!(assignments, vec![(10, 2), (20, 1), (30, 0)]);
    }

    #[test]
    fn identical_frames_fall_back_to_creation_order() {
        let remembered = [placed(1, 0), placed(2, 0)];
        let assignments = assign(&remembered, &[(9, frame(0)), (4, frame(0))]);
        assert_eq!(assignments, vec![(4, 0), (9, 1)]);
    }

    #[test]
    fn surplus_windows_are_left_unassigned() {
        let remembered = [placed(4, 0)];
        let assignments = assign(&remembered, &[(1, frame(700)), (2, frame(0))]);
        assert_eq!(assignments, vec![(2, 0)]);
    }

    #[test]
    fn recording_ignores_empty_and_unchanged_placements() {
        let mut memory = WindowMemory::default();
        assert!(!memory.record("com.example.App", Vec::new(), 1));
        assert!(memory.record("com.example.App", vec![placed(2, 0)], 1));
        assert!(!memory.record("com.example.App", vec![placed(2, 0)], 2));
        assert_eq!(memory.get("com.example.App").unwrap().updated_at, 1);
        assert!(!memory.record("com.example.App", Vec::new(), 3));
        assert_eq!(memory.get("com.example.App").unwrap().windows.len(), 1);
    }

    #[test]
    fn recording_evicts_the_least_recently_changed_application() {
        let mut memory = WindowMemory::default();
        for index in 0..MAX_APPS {
            memory.record(
                &format!("com.example.{index}"),
                vec![placed(1, 0)],
                index as u64 + 10,
            );
        }
        memory.record("com.example.0", vec![placed(2, 0)], 1_000);
        memory.record("com.example.new", vec![placed(3, 0)], 1_001);
        assert_eq!(memory.len(), MAX_APPS);
        assert!(memory.get("com.example.0").is_some());
        assert!(memory.get("com.example.1").is_none());
        assert!(memory.get("com.example.new").is_some());
        memory.validate().unwrap();
    }

    #[test]
    fn returning_marks_persist_and_older_files_still_load() {
        let mut memory = WindowMemory::default();
        memory.record("com.example.App", vec![placed(3, 0)], 1);
        assert!(!memory.returns("com.example.App"));
        assert!(memory.mark_returning("com.example.App"));
        assert!(!memory.mark_returning("com.example.App"));
        assert!(!memory.mark_returning(""));
        let json = serde_json::to_string(&memory).unwrap();
        assert!(
            json.ends_with(r#""returning":["com.example.App"]}"#),
            "{json}"
        );
        let restored: WindowMemory = serde_json::from_str(&json).unwrap();
        restored.validate().unwrap();
        assert!(restored.returns("com.example.App"));

        // Files written before returning marks existed load unchanged.
        let older: WindowMemory = serde_json::from_str(
            r#"{"apps":{"com.example.App":{"updated_at":1,"windows":[{"workspace":3,"frame":{"x":0,"y":0,"width":400,"height":300}}]}}}"#,
        )
        .unwrap();
        older.validate().unwrap();
        assert!(!older.returns("com.example.App"));
    }

    #[test]
    fn persisted_form_round_trips_and_validates() {
        let mut memory = WindowMemory::default();
        memory.record(
            "com.example.App",
            vec![
                placed(1, 0),
                RememberedWindow {
                    workspace: WorkspaceId::named('K').unwrap(),
                    frame: frame(10),
                    floating: true,
                },
            ],
            7,
        );
        let json = serde_json::to_string(&memory).unwrap();
        assert_eq!(
            json,
            r#"{"apps":{"com.example.App":{"updated_at":7,"windows":[{"workspace":1,"frame":{"x":0,"y":0,"width":400,"height":300}},{"workspace":"K","frame":{"x":10,"y":0,"width":400,"height":300},"floating":true}]}}}"#
        );
        let restored: WindowMemory = serde_json::from_str(&json).unwrap();
        restored.validate().unwrap();
        assert_eq!(restored, memory);
        let invalid: WindowMemory =
            serde_json::from_str(r#"{"apps":{"com.example.App":{"updated_at":1,"windows":[]}}}"#)
                .unwrap();
        assert!(invalid.validate().is_err());
    }
}
