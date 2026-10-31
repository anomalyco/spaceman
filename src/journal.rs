//! Arrangement journal: a bounded, title-free JSONL record of what Spaceman
//! believed about every window compared with what the window server showed.
//!
//! The agent writes a full [`Arrangement`] snapshot at startup and
//! periodically, a [`JournalEvent::Diff`] whenever the arrangement changes,
//! and begin/end events for debounced [`Anomaly`] disagreements between the
//! model and the screen. `spaceman doctor` summarizes the result.

use std::{
    collections::{BTreeMap, HashMap},
    io::{BufRead, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};

use crate::{Rect, WorkspaceId};

pub const JOURNAL_FILE: &str = "journal.jsonl";
pub const ROTATED_JOURNAL_FILE: &str = "journal.jsonl.1";
/// The live journal rotates past this size; one rotated generation is kept.
pub const MAX_JOURNAL_BYTES: u64 = 4 * 1024 * 1024;
/// A disagreement must persist this long before it is journaled. Workspace
/// switches and window creation briefly produce every anomaly kind.
pub const ANOMALY_SETTLE: Duration = Duration::from_secs(3);
/// Frame changes within this tolerance are not journaled as diffs.
pub const FRAME_DIFF_TOLERANCE: i32 = 4;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct JournalLine {
    /// Unix milliseconds.
    pub t: u64,
    #[serde(flatten)]
    pub event: JournalEvent,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JournalEvent {
    Start {
        version: String,
        pid: u32,
        restored_state: bool,
        slots: usize,
        parked: usize,
        remembered_apps: usize,
    },
    Snapshot {
        arrangement: Arrangement,
    },
    Diff {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        active: Option<Vec<WorkspaceId>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        displays: Option<Vec<Rect>>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        changes: Vec<WindowChange>,
    },
    Anomaly {
        state: AnomalyState,
        anomaly: Anomaly,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u64>,
    },
    /// A relaunched application's window claimed a remembered placement.
    Recall {
        bundle_id: String,
        window: u32,
        workspace: WorkspaceId,
        floating: bool,
    },
    /// A relaunch's recall window closed with placements left unclaimed.
    RecallExpired {
        bundle_id: String,
        unclaimed: usize,
    },
    /// An application's remembered placements were updated.
    Remembered {
        bundle_id: String,
        windows: usize,
    },
    Restore {
        reason: String,
        requested: usize,
        failed: usize,
    },
    /// Kit moved a window to another workspace by hand within
    /// [`CORRECTION_WINDOW`] of Spaceman placing it: the placement was wrong.
    WrongPlacement {
        bundle_id: String,
        window: u32,
        source: PlacementSource,
        placed: WorkspaceId,
        moved_to: WorkspaceId,
        after_ms: u64,
    },
    /// Kit ran `spaceman ctl wrong` when something looked wrong; a full
    /// snapshot follows.
    MarkedWrong,
    Stop,
}

/// A hand move this soon after Spaceman placed a window corrects that placement.
pub const CORRECTION_WINDOW: Duration = Duration::from_secs(120);

/// Why Spaceman put a newly seen window on its workspace.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlacementSource {
    /// No rule or memory applied: the workspace showing where it opened.
    Active,
    /// A remembered placement from an earlier instance of its app.
    Recall,
    /// A configured route rule.
    Rule,
}

impl PlacementSource {
    const fn label(self) -> &'static str {
        match self {
            Self::Active => "opened on the active workspace",
            Self::Recall => "recalled from memory",
            Self::Rule => "routed by a rule",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnomalyState {
    Begin,
    End,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Arrangement {
    pub displays: Vec<Rect>,
    pub active: Vec<WorkspaceId>,
    /// Sorted by window-server ID.
    pub windows: Vec<ArrangedWindow>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowRole {
    Tiled,
    Floating,
    /// Observed, managed by configuration, but owned by no workspace.
    Unowned,
    /// Excluded by an `ignore` rule.
    Ignored,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArrangedWindow {
    pub window: u32,
    pub pid: i32,
    pub bundle_id: String,
    /// Window-server frame; absent when the window server does not list it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<Rect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<WorkspaceId>,
    pub role: WindowRole,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub parked: bool,
}

impl ArrangedWindow {
    fn differs(&self, other: &Self) -> bool {
        self.pid != other.pid
            || self.bundle_id != other.bundle_id
            || self.owner != other.owner
            || self.role != other.role
            || self.parked != other.parked
            || match (self.frame, other.frame) {
                (Some(before), Some(after)) => {
                    !before.approximately_equals(after, FRAME_DIFF_TOLERANCE)
                }
                (None, None) => false,
                _ => true,
            }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WindowChange {
    pub window: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<ArrangedWindow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<ArrangedWindow>,
}

/// The diff from `before` to `after`, or `None` when nothing material changed.
#[must_use]
pub fn diff(before: &Arrangement, after: &Arrangement) -> Option<JournalEvent> {
    let old: BTreeMap<_, _> = before.windows.iter().map(|w| (w.window, w)).collect();
    let new: BTreeMap<_, _> = after.windows.iter().map(|w| (w.window, w)).collect();
    let mut changes = Vec::new();
    for (id, previous) in &old {
        match new.get(id) {
            None => changes.push(WindowChange {
                window: *id,
                before: Some((*previous).clone()),
                after: None,
            }),
            Some(current) if previous.differs(current) => changes.push(WindowChange {
                window: *id,
                before: Some((*previous).clone()),
                after: Some((*current).clone()),
            }),
            Some(_) => {}
        }
    }
    for (id, current) in &new {
        if !old.contains_key(id) {
            changes.push(WindowChange {
                window: *id,
                before: None,
                after: Some((*current).clone()),
            });
        }
    }
    changes.sort_by_key(|change| change.window);
    let active = (before.active != after.active).then(|| after.active.clone());
    let displays = (before.displays != after.displays).then(|| after.displays.clone());
    (active.is_some() || displays.is_some() || !changes.is_empty()).then_some(JournalEvent::Diff {
        active,
        displays,
        changes,
    })
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnomalyKind {
    /// Owned by a hidden workspace, not parked, yet on a display.
    Leaked,
    /// Owned by a visible workspace and not parked, yet off every display.
    Stranded,
    /// Recorded as parked, yet on a display.
    ParkedVisible,
    /// Managed by configuration and on a display, but owned by no workspace.
    Unadopted,
    /// Tiled on a visible workspace but away from its layout target.
    Misplaced,
    /// Owned, but the window server no longer lists it.
    Missing,
    /// Left visible across switches because it repeatedly rejects parking.
    ParkingExempt,
}

impl AnomalyKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Leaked => "leaked",
            Self::Stranded => "stranded",
            Self::ParkedVisible => "parked_visible",
            Self::Unadopted => "unadopted",
            Self::Misplaced => "misplaced",
            Self::Missing => "missing",
            Self::ParkingExempt => "parking_exempt",
        }
    }

    const fn meaning(self) -> &'static str {
        match self {
            Self::Leaked => "visible although its workspace is hidden",
            Self::Stranded => "off screen although its workspace is visible",
            Self::ParkedVisible => "marked hidden but on screen",
            Self::Unadopted => "on screen but owned by no workspace",
            Self::Misplaced => "not where its layout puts it",
            Self::Missing => "owned but gone from the window server",
            Self::ParkingExempt => "stopped hiding after repeatedly un-parking itself",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Anomaly {
    pub kind: AnomalyKind,
    pub window: u32,
    pub bundle_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<Rect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<Rect>,
}

/// Debounces anomalies: one is journaled only after persisting for
/// [`ANOMALY_SETTLE`], and its end is journaled with the total duration.
#[derive(Debug, Default)]
pub struct AnomalyTracker {
    pending: HashMap<(AnomalyKind, u32), Instant>,
    active: BTreeMap<(AnomalyKind, u32), (Anomaly, Instant)>,
}

impl AnomalyTracker {
    pub fn update(&mut self, now: Instant, current: Vec<Anomaly>) -> Vec<JournalEvent> {
        let mut events = Vec::new();
        let mut present = BTreeMap::new();
        for anomaly in current {
            present.insert((anomaly.kind, anomaly.window), anomaly);
        }
        for (_, (anomaly, began)) in self
            .active
            .extract_if(.., |key, _| !present.contains_key(key))
        {
            events.push(JournalEvent::Anomaly {
                state: AnomalyState::End,
                anomaly,
                duration_ms: Some(duration_ms(now.saturating_duration_since(began))),
            });
        }
        self.pending.retain(|key, _| present.contains_key(key));
        for (key, anomaly) in present {
            if self.active.contains_key(&key) {
                continue;
            }
            let first_seen = *self.pending.entry(key).or_insert(now);
            if now.saturating_duration_since(first_seen) < ANOMALY_SETTLE {
                continue;
            }
            self.pending.remove(&key);
            events.push(JournalEvent::Anomaly {
                state: AnomalyState::Begin,
                anomaly: anomaly.clone(),
                duration_ms: None,
            });
            self.active.insert(key, (anomaly, first_seen));
        }
        events
    }
}

fn duration_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[must_use]
pub fn unix_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(duration_ms)
        .unwrap_or_default()
}

/// Append one line to the journal at `path`, rotating it to `rotated` first
/// when it has outgrown [`MAX_JOURNAL_BYTES`]. Files are owner-only.
pub fn append(path: &Path, rotated: &Path, line: &JournalLine) -> Result<(), String> {
    use std::os::unix::fs::OpenOptionsExt;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create journal directory: {error}"))?;
    }
    if std::fs::metadata(path).is_ok_and(|metadata| metadata.len() > MAX_JOURNAL_BYTES) {
        std::fs::rename(path, rotated)
            .map_err(|error| format!("failed to rotate journal: {error}"))?;
    }
    let mut bytes =
        serde_json::to_vec(line).map_err(|error| format!("failed to encode journal: {error}"))?;
    bytes.push(b'\n');
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path)
        .and_then(|mut file| file.write_all(&bytes))
        .map_err(|error| format!("failed to write journal: {error}"))
}

#[must_use]
pub fn default_paths() -> Option<(PathBuf, PathBuf)> {
    Some((
        crate::config::application_support_path(JOURNAL_FILE)?,
        crate::config::application_support_path(ROTATED_JOURNAL_FILE)?,
    ))
}

/// Read the rotated generation followed by the live journal, skipping lines
/// that do not parse (a crash may truncate the final line).
pub fn read(paths: &[impl AsRef<Path>]) -> Result<(Vec<JournalLine>, usize), String> {
    let mut lines = Vec::new();
    let mut skipped = 0;
    for path in paths {
        let path = path.as_ref();
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("failed to read {}: {error}", path.display())),
        };
        for line in std::io::BufReader::new(file).lines() {
            let line = line.map_err(|error| format!("failed to read journal: {error}"))?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str(&line) {
                Ok(line) => lines.push(line),
                Err(_) => skipped += 1,
            }
        }
    }
    Ok((lines, skipped))
}

#[derive(Default)]
struct KindSummary {
    count: usize,
    total_ms: u64,
    longest_ms: u64,
    open: usize,
    bundles: BTreeMap<String, usize>,
}

/// Human-readable summary of journal lines at or after `since_ms`.
#[must_use]
pub fn summarize(lines: &[JournalLine], since_ms: u64, now_ms: u64, skipped: usize) -> String {
    let lines: Vec<_> = lines.iter().filter(|line| line.t >= since_ms).collect();
    let mut out = String::new();
    let Some(first) = lines.first() else {
        return "No journal events in this period.".into();
    };
    let last = lines.last().expect("non-empty");
    out.push_str(&format!(
        "{} events from {} to {}",
        lines.len(),
        ago(now_ms, first.t),
        ago(now_ms, last.t)
    ));
    if skipped > 0 {
        out.push_str(&format!(" ({skipped} unreadable lines skipped)"));
    }
    out.push('\n');

    let mut starts = Vec::new();
    let mut restores = Vec::new();
    let mut stops = 0;
    let mut recalls: BTreeMap<&str, usize> = BTreeMap::new();
    let mut wrong: BTreeMap<PlacementSource, BTreeMap<&str, usize>> = BTreeMap::new();
    let mut marked = Vec::new();
    let mut expired: BTreeMap<&str, usize> = BTreeMap::new();
    let mut kinds: BTreeMap<AnomalyKind, KindSummary> = BTreeMap::new();
    let mut open: HashMap<(AnomalyKind, u32), u64> = HashMap::new();
    let mut arrangement: Option<Arrangement> = None;
    for line in &lines {
        match &line.event {
            JournalEvent::Start {
                version,
                restored_state,
                slots,
                parked,
                remembered_apps,
                ..
            } => {
                open.clear();
                starts.push(format!(
                    "  {}: v{version}, {}, {slots} saved slots, {parked} parked, memory for {remembered_apps} apps\n",
                    ago(now_ms, line.t),
                    if *restored_state { "resumed saved state" } else { "no saved state" },
                ));
            }
            JournalEvent::Stop => {
                open.clear();
                stops += 1;
            }
            JournalEvent::Restore {
                reason,
                requested,
                failed,
            } => restores.push(format!(
                "  {}: {reason}, {requested} parked, {failed} failed\n",
                ago(now_ms, line.t)
            )),
            JournalEvent::Recall { bundle_id, .. } => *recalls.entry(bundle_id).or_default() += 1,
            JournalEvent::WrongPlacement {
                bundle_id, source, ..
            } => {
                *wrong
                    .entry(*source)
                    .or_default()
                    .entry(bundle_id)
                    .or_default() += 1;
            }
            JournalEvent::RecallExpired {
                bundle_id,
                unclaimed,
            } => *expired.entry(bundle_id).or_default() += unclaimed,
            JournalEvent::Anomaly {
                state,
                anomaly,
                duration_ms,
            } => {
                let key = (anomaly.kind, anomaly.window);
                match state {
                    AnomalyState::Begin => {
                        let summary = kinds.entry(anomaly.kind).or_default();
                        summary.count += 1;
                        *summary
                            .bundles
                            .entry(anomaly.bundle_id.clone())
                            .or_default() += 1;
                        open.insert(key, line.t);
                    }
                    AnomalyState::End => {
                        if open.remove(&key).is_some() || kinds.contains_key(&anomaly.kind) {
                            let summary = kinds.entry(anomaly.kind).or_default();
                            let duration = duration_ms.unwrap_or_default();
                            summary.total_ms += duration;
                            summary.longest_ms = summary.longest_ms.max(duration);
                        }
                    }
                }
            }
            JournalEvent::Snapshot { arrangement: full } => arrangement = Some(full.clone()),
            JournalEvent::Diff {
                active,
                displays,
                changes,
            } => {
                if let Some(current) = arrangement.as_mut() {
                    apply_diff(current, active.as_ref(), displays.as_ref(), changes);
                }
            }
            JournalEvent::MarkedWrong => marked.push(line.t),
            JournalEvent::Remembered { .. } => {}
        }
    }
    for ((kind, _), began) in &open {
        let summary = kinds.entry(*kind).or_default();
        summary.open += 1;
        let duration = now_ms.saturating_sub(*began);
        summary.longest_ms = summary.longest_ms.max(duration);
    }

    out.push_str(&format!(
        "\nAgent starts: {}, clean stops: {stops}\n",
        starts.len()
    ));
    for start in starts.iter().rev().take(5) {
        out.push_str(start);
    }
    if !restores.is_empty() {
        out.push_str("Window restorations:\n");
        for restore in restores.iter().rev().take(5) {
            out.push_str(restore);
        }
    }
    if !recalls.is_empty() || !expired.is_empty() {
        out.push_str(&format!(
            "Remembered placements: {} windows recalled",
            recalls.values().sum::<usize>()
        ));
        let unclaimed = expired.values().sum::<usize>();
        if unclaimed > 0 {
            out.push_str(&format!(", {unclaimed} left unclaimed"));
        }
        out.push('\n');
        for (bundle, count) in &recalls {
            out.push_str(&format!("  {bundle}: {count} recalled\n"));
        }
        for (bundle, count) in &expired {
            out.push_str(&format!("  {bundle}: {count} unclaimed\n"));
        }
    }

    out.push_str(&format!(
        "\nWrong placements (moved by hand within {}s of being placed): {}\n",
        CORRECTION_WINDOW.as_secs(),
        wrong.values().flat_map(BTreeMap::values).sum::<usize>()
    ));
    for (source, bundles) in &wrong {
        let mut bundles: Vec<_> = bundles.iter().collect();
        bundles.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        out.push_str(&format!(
            "  {:>3}x {}: {}\n",
            bundles.iter().map(|(_, count)| **count).sum::<usize>(),
            source.label(),
            bundles
                .iter()
                .take(4)
                .map(|(bundle, count)| format!("{bundle} ({count})"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    if !marked.is_empty() {
        out.push_str(&format!(
            "Marked wrong by hand (spaceman ctl wrong): {}\n",
            marked
                .iter()
                .rev()
                .take(5)
                .map(|t| ago(now_ms, *t))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    out.push_str("\nAnomalies (lasting at least 3s):\n");
    if kinds.is_empty() {
        out.push_str("  none\n");
    }
    for (kind, summary) in &kinds {
        out.push_str(&format!(
            "  {:<15} {:>3}x  total {:>7}  longest {:>7}{}  ({})\n",
            kind.label(),
            summary.count,
            human_duration(summary.total_ms),
            human_duration(summary.longest_ms),
            if summary.open > 0 {
                format!(", {} still open", summary.open)
            } else {
                String::new()
            },
            kind.meaning()
        ));
        let mut bundles: Vec<_> = summary.bundles.iter().collect();
        bundles.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        let listed = bundles
            .iter()
            .take(4)
            .map(|(bundle, count)| format!("{bundle} ({count})"))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("                  {listed}\n"));
    }

    if let Some(arrangement) = arrangement {
        out.push_str(&format!(
            "\nLast known arrangement: active {}\n",
            arrangement
                .active
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
        let mut workspaces: BTreeMap<WorkspaceId, Vec<&ArrangedWindow>> = BTreeMap::new();
        let mut loose = Vec::new();
        for window in &arrangement.windows {
            match window.owner {
                Some(owner) => workspaces.entry(owner).or_default().push(window),
                None if window.role == WindowRole::Unowned => loose.push(window),
                None => {}
            }
        }
        for (workspace, windows) in workspaces {
            out.push_str(&format!("  {workspace}: {}\n", describe(&windows)));
        }
        if !loose.is_empty() {
            out.push_str(&format!("  unowned: {}\n", describe(&loose)));
        }
    }
    out.trim_end().to_owned()
}

fn apply_diff(
    arrangement: &mut Arrangement,
    active: Option<&Vec<WorkspaceId>>,
    displays: Option<&Vec<Rect>>,
    changes: &[WindowChange],
) {
    if let Some(active) = active {
        arrangement.active.clone_from(active);
    }
    if let Some(displays) = displays {
        arrangement.displays.clone_from(displays);
    }
    for change in changes {
        arrangement
            .windows
            .retain(|window| window.window != change.window);
        if let Some(after) = &change.after {
            arrangement.windows.push(after.clone());
        }
    }
    arrangement.windows.sort_by_key(|window| window.window);
}

fn describe(windows: &[&ArrangedWindow]) -> String {
    let mut bundles: BTreeMap<&str, usize> = BTreeMap::new();
    for window in windows {
        let name = window
            .bundle_id
            .rsplit('.')
            .next()
            .filter(|name| !name.is_empty())
            .unwrap_or("unknown");
        *bundles.entry(name).or_default() += 1;
    }
    bundles
        .into_iter()
        .map(|(name, count)| {
            if count == 1 {
                name.to_owned()
            } else {
                format!("{name} x{count}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn ago(now_ms: u64, t: u64) -> String {
    let elapsed = now_ms.saturating_sub(t);
    if elapsed < 1_000 {
        "now".into()
    } else {
        format!("{} ago", human_duration(elapsed))
    }
}

fn human_duration(ms: u64) -> String {
    let seconds = ms / 1_000;
    match seconds {
        0..60 => format!("{seconds}s"),
        60..3_600 => format!("{}m{:02}s", seconds / 60, seconds % 60),
        3_600..86_400 => format!("{}h{:02}m", seconds / 3_600, seconds % 3_600 / 60),
        _ => format!("{}d{:02}h", seconds / 86_400, seconds % 86_400 / 3_600),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(id: u32, owner: Option<u8>, x: i32) -> ArrangedWindow {
        ArrangedWindow {
            window: id,
            pid: 7,
            bundle_id: "com.example.App".into(),
            frame: Some(Rect {
                x,
                y: 0,
                width: 100,
                height: 100,
            }),
            owner: owner.map(|owner| WorkspaceId::new(owner).unwrap()),
            role: if owner.is_some() {
                WindowRole::Tiled
            } else {
                WindowRole::Unowned
            },
            parked: false,
        }
    }

    fn anomaly(kind: AnomalyKind, id: u32) -> Anomaly {
        Anomaly {
            kind,
            window: id,
            bundle_id: "com.example.App".into(),
            workspace: None,
            frame: None,
            expected: None,
        }
    }

    #[test]
    fn diff_reports_added_removed_and_material_changes_only() {
        let before = Arrangement {
            displays: vec![],
            active: vec![WorkspaceId::ONE],
            windows: vec![
                window(1, Some(1), 0),
                window(2, Some(1), 0),
                window(3, None, 0),
            ],
        };
        let after = Arrangement {
            displays: vec![],
            active: vec![WorkspaceId::ONE],
            windows: vec![
                window(1, Some(1), 3),
                window(2, Some(2), 0),
                window(4, None, 0),
            ],
        };
        let Some(JournalEvent::Diff {
            active, changes, ..
        }) = diff(&before, &after)
        else {
            panic!("expected a diff");
        };
        assert_eq!(active, None);
        let ids: Vec<_> = changes
            .iter()
            .map(|change| {
                (
                    change.window,
                    change.before.is_some(),
                    change.after.is_some(),
                )
            })
            .collect();
        assert_eq!(
            ids,
            vec![(2, true, true), (3, true, false), (4, false, true)]
        );
        assert_eq!(diff(&after, &after), None);
    }

    #[test]
    fn anomalies_are_debounced_and_report_their_duration() {
        let mut tracker = AnomalyTracker::default();
        let start = Instant::now();
        let leak = || vec![anomaly(AnomalyKind::Leaked, 9)];
        assert!(tracker.update(start, leak()).is_empty());
        // A transient that clears before settling is never journaled.
        let mut with_transient = leak();
        with_transient.push(anomaly(AnomalyKind::Missing, 3));
        assert!(
            tracker
                .update(start + Duration::from_secs(1), with_transient)
                .is_empty()
        );
        assert!(
            tracker
                .update(start + Duration::from_secs(2), leak())
                .is_empty()
        );
        let began = tracker.update(start + Duration::from_secs(4), leak());
        assert!(matches!(
            began.as_slice(),
            [JournalEvent::Anomaly {
                state: AnomalyState::Begin,
                ..
            }]
        ));
        assert!(
            tracker
                .update(start + Duration::from_secs(8), leak())
                .is_empty()
        );
        let ended = tracker.update(start + Duration::from_secs(10), Vec::new());
        assert!(matches!(
            ended.as_slice(),
            [JournalEvent::Anomaly {
                state: AnomalyState::End,
                duration_ms: Some(10_000),
                ..
            }]
        ));
    }

    #[test]
    fn journal_rotates_and_reads_both_generations() {
        let directory = std::env::temp_dir().join(format!(
            "spaceman-journal-test-{}-{}",
            std::process::id(),
            unix_millis()
        ));
        let path = directory.join(JOURNAL_FILE);
        let rotated = directory.join(ROTATED_JOURNAL_FILE);
        let line = |t| JournalLine {
            t,
            event: JournalEvent::Stop,
        };
        append(&path, &rotated, &line(1)).unwrap();
        // Simulate an oversized live journal.
        let file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        file.set_len(MAX_JOURNAL_BYTES + 1).unwrap();
        append(&path, &rotated, &line(2)).unwrap();
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let (lines, skipped) = read(&[&rotated, &path]).unwrap();
        assert_eq!(
            lines.iter().map(|line| line.t).collect::<Vec<_>>(),
            vec![1, 2]
        );
        // The padding bytes form one unreadable line.
        assert_eq!(skipped, 1);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn summary_reconstructs_the_arrangement_and_counts_anomalies() {
        let base = 1_000_000;
        let lines = vec![
            JournalLine {
                t: base,
                event: JournalEvent::Start {
                    version: "0.1.2".into(),
                    pid: 1,
                    restored_state: true,
                    slots: 2,
                    parked: 0,
                    remembered_apps: 1,
                },
            },
            JournalLine {
                t: base + 10,
                event: JournalEvent::Snapshot {
                    arrangement: Arrangement {
                        displays: vec![],
                        active: vec![WorkspaceId::ONE],
                        windows: vec![window(1, Some(1), 0)],
                    },
                },
            },
            JournalLine {
                t: base + 20,
                event: diff(
                    &Arrangement {
                        displays: vec![],
                        active: vec![WorkspaceId::ONE],
                        windows: vec![window(1, Some(1), 0)],
                    },
                    &Arrangement {
                        displays: vec![],
                        active: vec![WorkspaceId::ONE],
                        windows: vec![window(1, Some(1), 0), window(2, Some(3), 0)],
                    },
                )
                .unwrap(),
            },
            JournalLine {
                t: base + 30,
                event: JournalEvent::Anomaly {
                    state: AnomalyState::Begin,
                    anomaly: anomaly(AnomalyKind::Leaked, 2),
                    duration_ms: None,
                },
            },
            JournalLine {
                t: base + 5_030,
                event: JournalEvent::Anomaly {
                    state: AnomalyState::End,
                    anomaly: anomaly(AnomalyKind::Leaked, 2),
                    duration_ms: Some(5_000),
                },
            },
            JournalLine {
                t: base + 6_000,
                event: JournalEvent::WrongPlacement {
                    bundle_id: "com.example.App".into(),
                    window: 2,
                    source: PlacementSource::Active,
                    placed: WorkspaceId::ONE,
                    moved_to: WorkspaceId::new(3).unwrap(),
                    after_ms: 4_000,
                },
            },
            JournalLine {
                t: base + 7_000,
                event: JournalEvent::MarkedWrong,
            },
        ];
        let summary = summarize(&lines, 0, base + 65_000, 0);
        assert!(
            summary.contains("Wrong placements (moved by hand within 120s of being placed): 1"),
            "{summary}"
        );
        assert!(
            summary.contains("1x opened on the active workspace: com.example.App (1)"),
            "{summary}"
        );
        assert!(
            summary.contains("Marked wrong by hand (spaceman ctl wrong): 58s ago"),
            "{summary}"
        );
        assert!(summary.contains("Agent starts: 1"), "{summary}");
        assert!(summary.contains("leaked"), "{summary}");
        assert!(summary.contains("1x"), "{summary}");
        assert!(summary.contains("  1: App"), "{summary}");
        assert!(summary.contains("  3: App"), "{summary}");
        assert_eq!(
            summarize(&lines, base + 100_000, base + 200_000, 0),
            "No journal events in this period."
        );
    }
}
