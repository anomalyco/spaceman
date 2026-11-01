use std::collections::{BTreeMap, HashMap};

use crate::{Rect, WorkspaceId};

use super::{
    AnomalyKind, AnomalyState, ArrangedWindow, Arrangement, CORRECTION_WINDOW, JournalEvent,
    JournalLine, PlacementSource, WindowChange, WindowRole,
};

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
    let (Some(first), Some(last)) = (lines.first(), lines.last()) else {
        return "No journal events in this period.".into();
    };
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
