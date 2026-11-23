//! Activity and Doctor diagnostics pane: summarizes Pane's title-free
//! arrangement journal (`journal.jsonl`), model/screen anomalies, placement
//! recalls, and `procwatch` process vitals inside the Rune Settings window.

#[cfg(test)]
mod tests;

use std::{collections::BTreeMap, time::SystemTime};

use rune::{El, Tx, col, row, text};
use spaceman::journal::{AnomalyKind, AnomalyState, JournalEvent, JournalLine};

use crate::{
    Pane, Ui,
    controls::{self, ButtonKind, Paint},
    playground::{Scene, reset_fixture},
    view::scroll_body,
};

pub(crate) const SCENES: &[Scene] = &[("activity", |ui| {
    reset_fixture(ui, Pane::Activity);
    ui.activity = ActivityPane::fixture_healthy();
})];

const WINDOW_24H_MS: u64 = 24 * 3_600 * 1_000;
const MAX_ROWS: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tone {
    Normal,
    Success,
    Warning,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActivityRow {
    pub title: String,
    pub detail: String,
    pub age: String,
    pub tone: Tone,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DoctorReport {
    pub total_events: usize,
    pub skipped_lines: usize,
    pub starts: usize,
    pub stops: usize,
    pub restores: usize,
    pub restore_failures: usize,
    pub recalled_windows: usize,
    pub wrong_placements: usize,
    pub marked_wrong: usize,
    pub anomalies: usize,
    pub open_anomalies: usize,
    pub top_anomaly: Option<(AnomalyKind, usize)>,
    pub rows: Vec<ActivityRow>,
}

impl DoctorReport {
    #[must_use]
    pub fn from_lines(lines: &[JournalLine], now_ms: u64, skipped_lines: usize) -> Self {
        let since_ms = now_ms.saturating_sub(WINDOW_24H_MS);
        let mut report = Self {
            skipped_lines,
            ..Self::default()
        };
        let mut anomaly_counts: BTreeMap<AnomalyKind, usize> = BTreeMap::new();
        let mut open_anomalies: BTreeMap<(AnomalyKind, u32), ()> = BTreeMap::new();
        let mut notable = Vec::new();

        for line in lines.iter().filter(|line| line.t >= since_ms) {
            report.total_events += 1;
            let age = human_age(now_ms, line.t);
            match &line.event {
                JournalEvent::Start {
                    version,
                    restored_state,
                    slots,
                    parked,
                    remembered_apps,
                    ..
                } => {
                    open_anomalies.clear();
                    report.starts += 1;
                    let state = if *restored_state {
                        "resumed saved workspaces"
                    } else {
                        "fresh workspace state"
                    };
                    notable.push(ActivityRow {
                        title: format!("Agent started (v{version})"),
                        detail: format!(
                            "{state} · {slots} slots, {parked} parked, {remembered_apps} apps in memory"
                        ),
                        age,
                        tone: Tone::Normal,
                    });
                }
                JournalEvent::Stop => {
                    open_anomalies.clear();
                    report.stops += 1;
                    notable.push(ActivityRow {
                        title: "Agent stopped cleanly".into(),
                        detail: "Parked windows restored before exit.".into(),
                        age,
                        tone: Tone::Success,
                    });
                }
                JournalEvent::Restore {
                    reason,
                    requested,
                    failed,
                } => {
                    report.restores += 1;
                    report.restore_failures += *failed;
                    notable.push(ActivityRow {
                        title: if *failed == 0 {
                            format!("Restored {requested} parked window{}", plural(*requested))
                        } else {
                            format!("Window restore had {failed} failure{}", plural(*failed))
                        },
                        detail: human_restore_reason(reason),
                        age,
                        tone: if *failed == 0 {
                            Tone::Success
                        } else {
                            Tone::Warning
                        },
                    });
                }
                JournalEvent::Recall {
                    bundle_id,
                    workspace,
                    floating,
                    ..
                } => {
                    report.recalled_windows += 1;
                    let app = short_bundle_name(bundle_id);
                    let mode = if *floating { "floating" } else { "tiled" };
                    notable.push(ActivityRow {
                        title: format!("Recalled {app} to Workspace {workspace}"),
                        detail: format!("Matched remembered {mode} placement ({bundle_id})"),
                        age,
                        tone: Tone::Normal,
                    });
                }
                JournalEvent::WrongPlacement {
                    bundle_id,
                    source,
                    placed,
                    moved_to,
                    ..
                } => {
                    report.wrong_placements += 1;
                    let app = short_bundle_name(bundle_id);
                    notable.push(ActivityRow {
                        title: format!("Hand-moved {app} from {placed} to {moved_to}"),
                        detail: format!("Originally {}", source.label()),
                        age,
                        tone: Tone::Warning,
                    });
                }
                JournalEvent::Anomaly {
                    state,
                    anomaly,
                    duration_ms,
                } => {
                    let key = (anomaly.kind, anomaly.window);
                    let app = short_bundle_name(&anomaly.bundle_id);
                    match state {
                        AnomalyState::Begin => {
                            report.anomalies += 1;
                            *anomaly_counts.entry(anomaly.kind).or_default() += 1;
                            open_anomalies.insert(key, ());
                            let where_label = anomaly
                                .workspace
                                .map_or_else(String::new, |ws| format!(" on Workspace {ws}"));
                            notable.push(ActivityRow {
                                title: format!(
                                    "Anomaly: {} ({app}){where_label}",
                                    anomaly.kind.label()
                                ),
                                detail: anomaly.kind.meaning().into(),
                                age,
                                tone: Tone::Warning,
                            });
                        }
                        AnomalyState::End => {
                            open_anomalies.remove(&key);
                            let lasted = human_duration(duration_ms.unwrap_or_default());
                            notable.push(ActivityRow {
                                title: format!("Resolved {} ({app})", anomaly.kind.label()),
                                detail: format!("Settled after {lasted}"),
                                age,
                                tone: Tone::Success,
                            });
                        }
                    }
                }
                JournalEvent::MarkedWrong => {
                    report.marked_wrong += 1;
                    notable.push(ActivityRow {
                        title: "Arrangement marked for inspection".into(),
                        detail: "Recorded full title-free snapshot into journal.jsonl.".into(),
                        age,
                        tone: Tone::Warning,
                    });
                }
                JournalEvent::Diff {
                    active: Some(active),
                    ..
                } if !active.is_empty() => {
                    let list = active
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(" · ");
                    notable.push(ActivityRow {
                        title: format!("Active workspaces: {list}"),
                        detail: "Workspace visibility updated".into(),
                        age,
                        tone: Tone::Normal,
                    });
                }
                JournalEvent::RecallExpired { .. }
                | JournalEvent::Remembered { .. }
                | JournalEvent::Snapshot { .. }
                | JournalEvent::Diff { .. } => {}
            }
        }

        report.open_anomalies = open_anomalies.len();
        report.top_anomaly = anomaly_counts
            .into_iter()
            .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)));
        notable.reverse();
        notable.truncate(MAX_ROWS);
        report.rows = notable;
        report
    }
}

#[derive(Clone, Default)]
pub struct ActivityPane {
    stamp: Option<(Option<SystemTime>, u64)>,
    loaded: bool,
    pub error: Option<String>,
    pub report: DoctorReport,
    fixture_vitals: Option<String>,
}

impl ActivityPane {
    pub fn poll(&mut self, visible: bool, fixture: bool) -> bool {
        if !visible || fixture {
            return false;
        }
        let current_stamp = journal_stamp();
        if self.loaded && self.stamp == current_stamp {
            return false;
        }
        self.reload(fixture);
        true
    }

    pub fn reload(&mut self, fixture: bool) {
        if fixture {
            return;
        }
        self.loaded = true;
        self.stamp = journal_stamp();
        let Some((live, rotated)) = spaceman::journal::default_paths() else {
            self.error = Some("HOME is unavailable".into());
            return;
        };
        match spaceman::journal::read(&[&rotated, &live]) {
            Ok((lines, skipped)) => {
                self.error = None;
                let now_ms = spaceman::journal::unix_millis();
                self.report = DoctorReport::from_lines(&lines, now_ms, skipped);
            }
            Err(error) => {
                self.error = Some(error);
            }
        }
    }

    #[must_use]
    fn fixture_healthy() -> Self {
        let now_ms = 1_700_000_000_000;
        let at = |secs_ago: u64| now_ms - secs_ago * 1_000;
        let lines = vec![
            JournalLine {
                t: at(3_400),
                event: JournalEvent::Start {
                    version: env!("CARGO_PKG_VERSION").into(),
                    pid: 4120,
                    restored_state: true,
                    slots: 6,
                    parked: 3,
                    remembered_apps: 9,
                },
            },
            JournalLine {
                t: at(3_380),
                event: JournalEvent::Recall {
                    bundle_id: "com.apple.Safari".into(),
                    window: 101,
                    workspace: spaceman::WorkspaceId::ONE,
                    floating: false,
                },
            },
            JournalLine {
                t: at(3_378),
                event: JournalEvent::Recall {
                    bundle_id: "com.apple.MobileSMS".into(),
                    window: 108,
                    workspace: spaceman::WorkspaceId::TWO,
                    floating: false,
                },
            },
            JournalLine {
                t: at(420),
                event: JournalEvent::Restore {
                    reason: "pause".into(),
                    requested: 3,
                    failed: 0,
                },
            },
            JournalLine {
                t: at(18),
                event: JournalEvent::Diff {
                    active: Some(vec![spaceman::WorkspaceId::TWO, spaceman::WorkspaceId::SIX]),
                    displays: None,
                    changes: Vec::new(),
                },
            },
        ];
        Self {
            stamp: None,
            loaded: true,
            error: None,
            report: DoctorReport::from_lines(&lines, now_ms, 0),
            fixture_vitals: Some("28 MB · 34 fds".into()),
        }
    }
}

pub fn header_actions(ui: &Ui, p: Paint) -> El {
    let can_mark = ui.agent.as_ref().is_some_and(|agent| agent.automatic);
    let mut actions = row().key("activity-actions").gap(8.0).center_y();
    if can_mark {
        actions = actions.child(controls::button(
            p,
            "mark-wrong",
            "Mark Issue",
            ButtonKind::Secondary,
            |ui: &mut Ui, _: &mut Tx| {
                ui.mark_wrong();
            },
        ));
    }
    actions.child(controls::button(
        p,
        "refresh-activity",
        "Refresh",
        ButtonKind::Secondary,
        |ui: &mut Ui, _: &mut Tx| {
            let fixture = ui.fixture;
            ui.activity.reload(fixture);
        },
    ))
}

pub fn pane(ui: &Ui, p: Paint) -> El {
    let report = &ui.activity.report;
    let vitals = ui
        .activity
        .fixture_vitals
        .clone()
        .unwrap_or_else(process_vitals);

    let (runtime_desc, runtime_dot) = match ui.agent.as_ref() {
        Some(agent) if agent.automatic => {
            let active = if agent.active_workspaces.is_empty() {
                "Automatic tiling active".to_owned()
            } else {
                let list = agent
                    .active_workspaces
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" · ");
                format!("Automatic tiling · Workspaces {list}")
            };
            (active, p.success())
        }
        Some(agent) if !agent.accessibility => (
            "Paused · Accessibility permission required".into(),
            p.warning(),
        ),
        Some(_) => ("Paused · Parked windows restored".into(), p.muted()),
        None => ("Agent disconnected".into(), p.muted()),
    };

    let vitals_badge = row()
        .key("vitals-badge")
        .gap(8.0)
        .center_y()
        .children((!vitals.is_empty()).then(|| {
            text(vitals)
                .key("vitals")
                .font(p.k.ty.small)
                .color(p.faint())
        }))
        .child(controls::status_dot(runtime_dot));

    let (anomaly_desc, anomaly_color, anomaly_dot) = if report.anomalies == 0 {
        (
            "No model/screen disagreements lasting over 3s in the last 24h.".to_owned(),
            p.muted(),
            p.success(),
        )
    } else {
        let top = report
            .top_anomaly
            .map(|(kind, count)| format!(" Most common: {} ({count}×).", kind.label()))
            .unwrap_or_default();
        let open = if report.open_anomalies > 0 {
            format!(" {} currently active.", report.open_anomalies)
        } else {
            String::new()
        };
        (
            format!(
                "{} disagreement{} in the last 24h.{top}{open}",
                report.anomalies,
                plural(report.anomalies)
            ),
            p.warning(),
            p.warning(),
        )
    };

    let memory_desc = format!(
        "{} window{} recalled on relaunch · {} hand-moved within 2m",
        report.recalled_windows,
        plural(report.recalled_windows),
        report.wrong_placements
    );
    let recovery_desc = format!(
        "{} start{} · {} restore{} ({} failed)",
        report.starts,
        plural(report.starts),
        report.restores,
        plural(report.restores),
        report.restore_failures
    );
    let recovery_dot = if report.restore_failures == 0 {
        p.success()
    } else {
        p.warning()
    };

    let mut sections = vec![controls::section(
        p,
        "health",
        "Health (last 24 hours)",
        vec![
            controls::setting_row(
                p,
                "runtime",
                "Runtime",
                Some((runtime_desc, p.muted())),
                vitals_badge,
            ),
            controls::setting_row(
                p,
                "anomalies",
                "Model vs. screen",
                Some((anomaly_desc, anomaly_color)),
                controls::status_dot(anomaly_dot),
            ),
            controls::setting_row(
                p,
                "memory",
                "Placement recall",
                Some((memory_desc, p.muted())),
                text(format!("{} events", report.total_events))
                    .key("event-count")
                    .font(p.k.ty.small)
                    .color(p.faint()),
            ),
            controls::setting_row(
                p,
                "recovery",
                "Window recovery",
                Some((recovery_desc, p.muted())),
                controls::status_dot(recovery_dot),
            ),
        ],
    )];

    let journal_rows = if let Some(error) = &ui.activity.error {
        vec![controls::setting_row(
            p,
            "journal-error",
            "Couldn't read arrangement journal",
            Some((error.clone(), p.danger())),
            controls::status_dot(p.danger()),
        )]
    } else if report.rows.is_empty() {
        vec![controls::setting_row(
            p,
            "journal-empty",
            "No arrangement events yet",
            Some((
                "Pane records title-free starts, restores, recalls, and anomalies here.".into(),
                p.muted(),
            )),
            controls::status_dot(p.k.c.border_strong),
        )]
    } else {
        report
            .rows
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let detail_color = match entry.tone {
                    Tone::Warning => p.warning(),
                    Tone::Normal | Tone::Success => p.muted(),
                };
                controls::setting_row(
                    p,
                    &format!("entry-{index}"),
                    &entry.title,
                    Some((entry.detail.clone(), detail_color)),
                    text(entry.age.clone())
                        .key("age")
                        .font(p.k.ty.small)
                        .color(p.faint()),
                )
            })
            .collect()
    };
    sections.push(controls::section(
        p,
        "journal",
        "Recent journal",
        journal_rows,
    ));

    sections.push(controls::section(
        p,
        "diagnostics",
        "Diagnostics",
        vec![controls::setting_row(
            p,
            "journal-file",
            "journal.jsonl",
            Some((
                "Title-free local log summarized here and by `pane doctor`.".into(),
                p.muted(),
            )),
            controls::button(
                p,
                "reveal-journal",
                "Show in Finder",
                ButtonKind::Secondary,
                |_: &mut Ui, _: &mut Tx| {
                    if let Some((live, _)) = spaceman::journal::default_paths() {
                        controls::reveal_in_finder(&live);
                    }
                },
            ),
        )],
    ));

    scroll_body(
        "activity-body",
        col().key("sections").gap(24.0).children(sections),
    )
}

fn journal_stamp() -> Option<(Option<SystemTime>, u64)> {
    let (live, _) = spaceman::journal::default_paths()?;
    let metadata = std::fs::metadata(live).ok()?;
    Some((metadata.modified().ok(), metadata.len()))
}

pub(crate) fn process_vitals() -> String {
    let usage = procwatch::usage();
    match (usage.footprint, usage.fds) {
        (Some(footprint), Some(fds)) => format!("{} MB · {fds} fds", footprint >> 20),
        (Some(footprint), None) => format!("{} MB", footprint >> 20),
        _ => String::new(),
    }
}

fn short_bundle_name(bundle_id: &str) -> &str {
    bundle_id
        .rsplit('.')
        .next()
        .filter(|segment| !segment.is_empty())
        .unwrap_or(bundle_id)
}

const fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

fn human_restore_reason(reason: &str) -> String {
    match reason {
        "pause" => "Paused automatic tiling".into(),
        "restore all" => "Paused or quitting Pane".into(),
        "startup without saved workspaces" => "Startup recovery without saved workspaces".into(),
        other => other.into(),
    }
}

fn human_age(now_ms: u64, at_ms: u64) -> String {
    let elapsed = now_ms.saturating_sub(at_ms);
    if elapsed < 2_000 {
        "just now".into()
    } else {
        format!("{} ago", human_duration(elapsed))
    }
}

fn human_duration(ms: u64) -> String {
    let secs = ms / 1_000;
    match secs {
        0..60 => format!("{secs}s"),
        60..3_600 => format!("{}m", secs / 60),
        3_600..86_400 => format!("{}h {}m", secs / 3_600, (secs % 3_600) / 60),
        _ => format!("{}d", secs / 86_400),
    }
}
