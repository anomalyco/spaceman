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

#[test]
fn lookback_reconstructs_arrangement_from_snapshot_before_cutoff() {
    let minute = 60_000;
    let now = 10_000_000;
    let since = now - 60 * minute;
    let display = Rect {
        x: 0,
        y: 0,
        width: 1200,
        height: 800,
    };
    let initial = Arrangement {
        displays: vec![display],
        active: vec![WorkspaceId::ONE],
        windows: vec![window(1, Some(1), 0)],
    };
    let before_cutoff = Arrangement {
        displays: vec![display],
        active: vec![WorkspaceId::ONE],
        windows: vec![window(1, Some(1), 0), window(2, Some(3), 0)],
    };
    let after_cutoff = Arrangement {
        displays: vec![display],
        active: vec![WorkspaceId::new(2).unwrap()],
        windows: vec![window(1, Some(1), 0), window(2, Some(3), 0)],
    };
    let lines = vec![
        JournalLine {
            t: now - 75 * minute,
            event: JournalEvent::Snapshot {
                arrangement: initial.clone(),
            },
        },
        JournalLine {
            t: now - 70 * minute,
            event: diff(&initial, &before_cutoff).unwrap(),
        },
        JournalLine {
            t: now - 50 * minute,
            event: diff(&before_cutoff, &after_cutoff).unwrap(),
        },
    ];

    let lookback = summarize(&lines, since, now, 0);
    assert!(
        lookback.starts_with("1 events from 50m ago to 50m ago"),
        "{lookback}"
    );
    assert!(
        lookback.contains("Last known arrangement: active 2"),
        "{lookback}"
    );
    assert!(lookback.contains("  1: App"), "{lookback}");
    assert!(lookback.contains("  3: App"), "{lookback}");

    let all_time = summarize(&lines, 0, now, 0);
    assert!(
        all_time.starts_with("3 events from 1h 15m ago to 50m ago"),
        "{all_time}"
    );
    assert!(
        all_time.contains("Last known arrangement: active 2"),
        "{all_time}"
    );
}

#[test]
fn every_anomaly_kind_has_prose_meaning_distinct_from_its_label() {
    for kind in [
        AnomalyKind::Leaked,
        AnomalyKind::Stranded,
        AnomalyKind::ParkedVisible,
        AnomalyKind::Unadopted,
        AnomalyKind::ConstrainedOverlap,
        AnomalyKind::Misplaced,
        AnomalyKind::Missing,
        AnomalyKind::ParkingExempt,
    ] {
        assert_ne!(
            kind.meaning(),
            kind.label(),
            "anomaly {:?} meaning should explain the condition rather than repeating its label",
            kind
        );
        assert!(kind.meaning().contains(' '), "expected prose for {kind:?}");
    }
}
