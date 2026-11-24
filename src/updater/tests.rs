use super::*;

fn temporary(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("spaceman-updater-{name}-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&path));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn staged(version: &str, sha: char) -> StagedUpdate {
    staged_for(UpdateChannel::Stable, version, sha)
}

fn staged_for(channel: UpdateChannel, version: &str, sha: char) -> StagedUpdate {
    StagedUpdate {
        channel,
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

#[test]
fn a_stamped_build_names_its_own_channel_and_version() {
    // A fresh dev zip, before any receipt: its stamp, not Cargo's 0.3.2.
    let dev = stamp::identity(Some("dev"), Some("0.0.0-dev-12"), None, "0.3.2");
    assert_eq!(dev.channel.as_deref(), Some("dev"));
    assert_eq!(dev.version, "0.0.0-dev-12");
    assert_eq!(
        transport::user_agent(dev.channel.as_deref().unwrap_or("stable"), &dev.version),
        "spaceman/dev/0.0.0-dev-12"
    );
    let stable = stamp::identity(Some("stable"), Some("0.3.3"), None, "0.3.3");
    assert_eq!(stable.channel.as_deref(), Some("stable"));
    // Builds from before the stamp: the receipt's version, and dev for a dev one.
    let legacy = stamp::identity(None, None, Some("0.0.0-dev-9"), "0.3.2");
    assert_eq!(legacy.channel.as_deref(), Some("dev"));
    assert_eq!(legacy.version, "0.0.0-dev-9");
    let unknown = stamp::identity(None, None, None, "0.3.2");
    assert_eq!(unknown.channel, None);
    assert_eq!(unknown.version, "0.3.2");
    // An unusable stamp is ignored.
    let odd = stamp::identity(Some("nightly"), Some("../x"), None, "0.3.2");
    assert_eq!(odd.channel, None);
    assert_eq!(odd.version, "0.3.2");
}

#[test]
fn another_channel_waits_for_the_user_to_switch() {
    let files = format!("/release/spaceman/files/{}/Spaceman.zip", "c".repeat(64));
    let stable = manifest("stable", "0.3.3", &files);
    let mut dev_build = install("0.0.0-dev-12", None);
    dev_build.channel = Some("dev".into());
    let decision = protocol::decide(APP, ARTIFACT, "stable", &stable, &dev_build);
    assert!(matches!(
        decision,
        Decision::Install {
            reason: Reason::Switch,
            ..
        }
    ));
    let sha = "c".repeat(64);
    assert_eq!(
        plan(decision.clone(), Method::Direct, None, None),
        Plan::Switch {
            version: "0.3.3".into(),
            sha256: sha.clone()
        }
    );
    // Once confirmed it stages, and a staged switch stays staged.
    assert!(matches!(
        plan(decision.clone(), Method::Direct, Some(&sha), None),
        Plan::Stage { .. }
    ));
    assert_eq!(
        plan(decision.clone(), Method::Direct, None, Some(&sha)),
        Plan::Staged
    );
    // The same channel never asks.
    let dev = manifest("dev", "0.0.0-dev-13", &files);
    assert!(matches!(
        plan(
            protocol::decide(APP, ARTIFACT, "dev", &dev, &dev_build),
            Method::Direct,
            None,
            None
        ),
        Plan::Stage { .. }
    ));

    let updater = Updater::with_phase(Phase::Available(Available::Switch {
        channel: UpdateChannel::Stable,
        version: "0.3.3".into(),
        sha256: sha.clone(),
    }));
    assert_eq!(
        updater.status(),
        UiUpdateStatus::Switch {
            channel: "stable".into(),
            version: "0.3.3".into()
        }
    );
    updater.confirm_switch();
    let core = updater.shared.lock();
    assert_eq!(
        core.confirmed_switch,
        Some((UpdateChannel::Stable, sha.clone()))
    );
    assert!(core.check_requested);
    assert_eq!(core.phase, Phase::Checking);
}

#[test]
fn a_copy_that_cant_replace_itself_offers_the_download() {
    let files = format!("/release/spaceman/files/{}/Spaceman.zip", "c".repeat(64));
    let stable = manifest("stable", "0.3.3", &files);
    let decision = protocol::decide(APP, ARTIFACT, "stable", &stable, &install("0.3.2", None));
    let Plan::Download { version, target } = plan(decision, Method::DownloadOnly, None, None)
    else {
        panic!("a read-only copy must not stage");
    };
    assert_eq!(version, "0.3.3");
    let manifest_url = format!("{ORIGIN}/{APP}/{ARTIFACT}/stable.json");
    assert_eq!(
        files_url(&manifest_url, &target).unwrap(),
        format!("{FILES}{}/Spaceman.zip", "c".repeat(64))
    );
    // A switch needs no confirmation there: nothing installs either way.
    let mut dev_build = install("0.0.0-dev-12", None);
    dev_build.channel = Some("dev".into());
    let decision = protocol::decide(APP, ARTIFACT, "stable", &stable, &dev_build);
    assert!(matches!(
        plan(decision, Method::DownloadOnly, None, None),
        Plan::Download { .. }
    ));
    let updater = Updater::with_phase(Phase::Available(Available::Download {
        version: "0.3.3".into(),
        url: format!("{FILES}x/Spaceman.zip"),
    }));
    assert_eq!(
        updater.status(),
        UiUpdateStatus::Download {
            version: "0.3.3".into()
        }
    );
    assert!(!updater.ready());
    assert!(updater.install_on_quit().is_none());
    assert!(Updater::with_phase(Phase::Idle).open_download().is_err());
}

#[test]
fn quitting_installs_a_ready_update_for_the_next_launch() {
    // Nothing staged: a quit installs nothing and touches nothing.
    let updater = Updater::with_phase(Phase::Idle);
    assert!(updater.install_on_quit().is_none());
    assert_eq!(updater.status(), UiUpdateStatus::Idle);

    let updates = temporary("quit");
    let updater = Updater::with_phase(Phase::Checking);
    assert_eq!(updater.shared.lock().adopt(staged("0.3.3", 'a')), None);
    let mut installed = Vec::new();
    let version = updater
        .apply_for(Some(UpdateChannel::Stable), Ok(updates.clone()), |staged| {
            installed.push(staged.update.clone());
            Ok(())
        })
        .unwrap();
    assert_eq!(version, "0.3.3");
    assert_eq!(installed.len(), 1);
    // The receipt names it once it runs, and no later check can stage another.
    let receipt = receipt_for(&updates, &"e".repeat(64)).unwrap();
    assert_eq!(receipt.version, "0.3.3");
    assert_eq!(updater.status(), UiUpdateStatus::Installing);
    assert!(updater.install_on_quit().is_none());
    assert!(updater.shared.lock().adopt(staged("0.3.4", 'b')).is_some());

    // A swap that fails leaves Spaceman as it was, and the quit goes on.
    let updater = Updater::with_phase(Phase::Idle);
    assert_eq!(updater.shared.lock().adopt(staged("0.3.3", 'a')), None);
    let error = updater
        .apply_for(Some(UpdateChannel::Stable), Ok(updates.clone()), |_| {
            Err("read-only".into())
        })
        .unwrap_err();
    assert_eq!(error, "read-only");
    assert_eq!(updater.status(), UiUpdateStatus::Failed);
    drop(std::fs::remove_dir_all(updates));
}

#[test]
fn switching_channels_offline_discards_a_staged_update_for_the_old_channel() {
    let updates = temporary("offline-channel-switch");
    let staged_root = updates.join(".Spaceman-update-0.0.0-dev-62");
    let staged_app = staged_root.join("Spaceman.app");
    std::fs::create_dir_all(&staged_app).unwrap();

    let updater = Updater::with_phase(Phase::Idle);
    let dev_update = StagedUpdate {
        channel: UpdateChannel::Dev,
        staged: bundle::Staged {
            update: staged_app,
            installed: updates.join("Spaceman.app"),
        },
        sha256: "d".repeat(64),
        version: "0.0.0-dev-62".into(),
        executable: "e".repeat(64),
    };
    assert_eq!(updater.shared.lock().adopt(dev_update), None);
    assert!(updater.ready());

    // User switches back to Stable while offline: prepare_check discards the
    // staged Dev bundle before the offline manifest fetch fails.
    prepare_check(&updater.shared, UpdateChannel::Stable);
    updater.shared.set_phase(Phase::Failed);

    assert!(!staged_root.exists());
    assert!(!updater.ready());
    assert_eq!(updater.status(), UiUpdateStatus::Failed);
    let mut installed = false;
    assert!(
        updater
            .install_on_quit_for(UpdateChannel::Stable, Ok(updates.clone()), |_| {
                installed = true;
                Ok(())
            })
            .is_none()
    );
    assert!(!installed);
    drop(std::fs::remove_dir_all(updates));
}

#[test]
fn quitting_respects_the_selected_channel_even_before_a_new_check_runs() {
    let updates = temporary("quit-channel-check");
    let staged_root = updates.join(".Spaceman-update-0.0.0-dev-62");
    let staged_app = staged_root.join("Spaceman.app");
    std::fs::create_dir_all(&staged_app).unwrap();

    // Dev is staged, user switches config back to Stable and quits before a check runs.
    let updater = Updater::with_phase(Phase::Idle);
    let dev_update = StagedUpdate {
        channel: UpdateChannel::Dev,
        staged: bundle::Staged {
            update: staged_app,
            installed: updates.join("Spaceman.app"),
        },
        sha256: "d".repeat(64),
        version: "0.0.0-dev-62".into(),
        executable: "e".repeat(64),
    };
    assert_eq!(updater.shared.lock().adopt(dev_update), None);
    let mut installed = false;
    assert!(
        updater
            .install_on_quit_for(UpdateChannel::Stable, Ok(updates.clone()), |_| {
                installed = true;
                Ok(())
            })
            .is_none()
    );
    assert!(!installed);
    assert!(!staged_root.exists());
    assert!(!updater.ready());

    // Control: keeping Dev selected (even after an offline check failure) still installs Dev on quit.
    let updater = Updater::with_phase(Phase::Idle);
    assert_eq!(
        updater
            .shared
            .lock()
            .adopt(staged_for(UpdateChannel::Dev, "0.0.0-dev-62", 'd')),
        None
    );
    prepare_check(&updater.shared, UpdateChannel::Dev);
    updater.shared.set_phase(Phase::Failed);
    assert!(updater.ready());
    let mut installed_dev = false;
    let outcome = updater
        .install_on_quit_for(UpdateChannel::Dev, Ok(updates.clone()), |_| {
            installed_dev = true;
            Ok(())
        })
        .unwrap();
    assert_eq!(outcome.unwrap(), "0.0.0-dev-62");
    assert!(installed_dev);
    drop(std::fs::remove_dir_all(updates));
}
