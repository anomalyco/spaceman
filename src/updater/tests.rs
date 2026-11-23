use super::*;

fn temporary(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("spaceman-updater-{name}-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&path));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn staged(version: &str, sha: char) -> StagedUpdate {
    StagedUpdate {
        staged: bundle::Staged {
            update: PathBuf::from(format!(
                "/nonexistent/.Spaceman-update-{version}/Spaceman.app"
            )),
            installed: PathBuf::from("/nonexistent/Spaceman.app"),
        },
        sha256: sha.to_string().repeat(64),
        version: version.into(),
        executable: "e".repeat(64),
    }
}

#[test]
fn a_staged_update_is_offered_while_later_checks_run() {
    let updater = Updater::with_phase(Phase::Idle);
    assert_eq!(updater.status(), UiUpdateStatus::Idle);
    assert!(!updater.ready());
    updater.check_now();
    assert_eq!(updater.status(), UiUpdateStatus::Checking);
    let before = updater.revision();
    assert_eq!(updater.shared.lock().adopt(staged("0.3.3", 'a')), None);
    updater.shared.set_phase(Phase::Checking);
    assert!(updater.revision() > before);
    assert_eq!(
        updater.status(),
        UiUpdateStatus::Ready {
            version: "0.3.3".into()
        }
    );
    assert!(updater.ready());
    // A newer release replaces it and hands back the one to discard.
    let replaced = updater.shared.lock().adopt(staged("0.3.4", 'b'));
    assert_eq!(
        replaced.map(|update| update.version).as_deref(),
        Some("0.3.3")
    );
}

#[test]
fn disabled_and_installed_updaters_never_check_or_stage() {
    for status in [UiUpdateStatus::SourceBuild, UiUpdateStatus::Unavailable] {
        let updater = Updater::with_phase(Phase::Disabled(status.clone()));
        updater.check_now();
        assert_eq!(updater.status(), status);
        assert!(!updater.shared.lock().check_requested);
        assert!(updater.install().is_err());
    }
    let updater = Updater::with_phase(Phase::Installed);
    let update = staged("0.3.3", 'a');
    assert_eq!(updater.shared.lock().adopt(update.clone()), Some(update));
    assert_eq!(updater.status(), UiUpdateStatus::Installing);
    assert!(!updater.ready());
}

#[test]
fn install_without_a_staged_update_changes_nothing() {
    let updater = Updater::with_phase(Phase::UpToDate);
    assert_eq!(updater.install().unwrap_err(), "no update is ready");
    assert_eq!(updater.status(), UiUpdateStatus::UpToDate);
}

#[test]
fn a_receipt_names_the_running_build_only_while_its_executable_runs() {
    let updates = temporary("receipt");
    assert_eq!(receipt_for(&updates, "e"), None);
    let update = staged("0.0.0-dev-9", 'c');
    write_receipt(&updates, &update).unwrap();
    let receipt = receipt_for(&updates, &"e".repeat(64)).unwrap();
    assert_eq!(receipt.sha256, "c".repeat(64));
    assert_eq!(receipt.version, "0.0.0-dev-9");
    // Another build at the same path (a manual install) ignores it.
    assert_eq!(receipt_for(&updates, &"f".repeat(64)), None);
    drop(std::fs::remove_dir_all(updates));
}

#[test]
fn the_rollout_bucket_is_drawn_once_and_kept() {
    let updates = temporary("bucket");
    let first = rollout_bucket(&updates).unwrap();
    assert!(first < 100);
    assert_eq!(rollout_bucket(&updates).unwrap(), first);
    std::fs::write(updates.join(BUCKET), "4294967295\n").unwrap();
    assert_eq!(rollout_bucket(&updates).unwrap(), 95);
    drop(std::fs::remove_dir_all(updates));
}

fn manifest(channel: &str, version: &str, url: &str) -> String {
    serde_json::json!({
        "schema": 1,
        "app": "spaceman",
        "artifact": "desktop",
        "channel": channel,
        "sequence": 2,
        "published_at": "2026-11-23T18:47:58Z",
        "release": {
            "version": version,
            "rollout_percent": 100,
            "targets": {
                "macos-arm64": { "url": url, "size": 13887717, "sha256": "c".repeat(64) }
            },
            "distributions": [{ "kind": "direct" }],
            "revoked_versions": []
        }
    })
    .to_string()
}

fn install(version: &str, sha256: Option<char>) -> Install {
    Install {
        version: version.into(),
        target: TARGET.into(),
        distribution: "direct".into(),
        rollout_bucket: 0,
        sha256: sha256.map(|sha| sha.to_string().repeat(64)),
        channel: None,
    }
}

#[test]
fn the_running_build_is_known_by_its_receipt_else_its_version() {
    let files = format!("/release/spaceman/files/{}/Spaceman.zip", "c".repeat(64));
    let stable = manifest("stable", "0.3.2", &files);
    let decide = |manifest: &str, install: &Install| {
        protocol::decide(APP, ARTIFACT, "stable", manifest, install)
    };
    assert!(matches!(
        decide(&stable, &install("0.3.2", None)),
        Decision::None { .. }
    ));
    assert!(matches!(
        decide(&stable, &install("0.3.1", None)),
        Decision::Install { .. }
    ));
    // A dev build's bundle carries Cargo's version; its receipt names it.
    let dev = manifest("dev", "0.0.0-dev-8", &files);
    let decide_dev = |install: &Install| protocol::decide(APP, ARTIFACT, "dev", &dev, install);
    assert!(matches!(
        decide_dev(&install("0.3.2", None)),
        Decision::Install { .. }
    ));
    assert!(matches!(
        decide_dev(&install("0.0.0-dev-7", Some('c'))),
        Decision::None { .. }
    ));
    assert!(matches!(
        decide_dev(&install("0.0.0-dev-7", Some('d'))),
        Decision::Install { .. }
    ));
    // Another app's or channel's manifest is refused.
    assert!(matches!(
        decide(&dev, &install("0.3.2", None)),
        Decision::Reject {
            reason: "wrong_channel"
        }
    ));
}

#[test]
fn downloads_only_come_from_the_release_services_spaceman_files() {
    let manifest_url = format!("{ORIGIN}/{APP}/{ARTIFACT}/stable.json");
    assert_eq!(
        manifest_url,
        "https://anoma.ly/release/spaceman/desktop/stable.json"
    );
    let allowed = |reference: &str| {
        protocol::resolve_url(&manifest_url, reference).filter(|url| url.starts_with(FILES))
    };
    assert_eq!(
        allowed("/release/spaceman/files/abc/Spaceman.zip").as_deref(),
        Some("https://anoma.ly/release/spaceman/files/abc/Spaceman.zip")
    );
    assert_eq!(
        allowed("../files/abc/Spaceman.zip").as_deref(),
        Some("https://anoma.ly/release/spaceman/files/abc/Spaceman.zip")
    );
    for refused in [
        "/release/hex/files/abc/Hex.zip",
        "/release/spaceman/desktop/x.zip",
        "/release/spaceman/files/../../hex/files/abc/Hex.zip",
        "https://example.com/release/spaceman/files/abc/Spaceman.zip",
        "https://anoma.ly.example.com/release/spaceman/files/abc/Spaceman.zip",
        "//example.com/release/spaceman/files/abc/Spaceman.zip",
    ] {
        assert_eq!(allowed(refused), None, "{refused}");
    }
}
