use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use serde_json::Value;
use spaceman::{
    AppInstanceId, ObservedWindow, PhysicalWindowId, Rect, WorkspaceId,
    config::{Config, HudStyle, WindowDisposition},
};

use super::{Outcome, try_run_with_config_path};

struct TempConfig {
    dir: PathBuf,
    path: PathBuf,
}

impl TempConfig {
    fn new(label: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "spaceman-cli-test-{label}-{}-{id}",
            std::process::id()
        ));
        let path = dir.join("config.toml");
        Self { dir, path }
    }

    fn write(&self, source: &str) {
        fs::create_dir_all(&self.dir).unwrap();
        fs::write(&self.path, source).unwrap();
    }

    fn read(&self) -> String {
        fs::read_to_string(&self.path).unwrap()
    }

    fn run(&self, args: &[&str]) -> Outcome {
        run_cli(args, Some(&self.path))
    }
}

impl Drop for TempConfig {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.dir));
    }
}

fn run_cli(args: &[&str], config_path: Option<&Path>) -> Outcome {
    let (&command, rest) = args.split_first().expect("args must be non-empty");
    try_run_with_config_path("spaceman", command, rest, config_path)
        .expect("command should be handled by cli module")
}

fn sample_window(title: Option<&str>) -> ObservedWindow {
    ObservedWindow {
        id: PhysicalWindowId {
            app: AppInstanceId {
                pid: 42,
                launched_at: 1,
            },
            window_id: 99,
        },
        frame: Rect::default(),
        resizable: true,
        title: title.map(std::sync::Arc::from),
    }
}

#[test]
fn help_and_version_exit_zero_on_stdout() {
    for args in [
        &["--help"][..],
        &["-h"][..],
        &["help"][..],
        &["help", "config"][..],
        &["config", "--help"][..],
        &["config", "-h"][..],
        &["config", "set", "--help"][..],
        &["rules", "--help"][..],
        &["rules", "add", "--help"][..],
        &["workspaces", "--help"][..],
        &["workspaces", "assign", "--help"][..],
        &["replay", "--help"][..],
        &["doctor", "--help"][..],
        &["config-check", "--help"][..],
    ] {
        let outcome = run_cli(args, None);
        assert_eq!(outcome.exit_code, 0, "args: {args:?}");
        assert!(outcome.stdout.as_deref().is_some_and(|s| !s.is_empty()));
        assert!(outcome.stderr.is_none());
    }

    #[cfg(target_os = "macos")]
    for args in [
        &["ctl", "--help"][..],
        &["ctl", "-h"][..],
        &["ctl", "help"][..],
        &["help", "ctl"][..],
        &["open", "--help"][..],
        &["help", "open"][..],
        &["snapshot", "--help"][..],
        &["displays", "--help"][..],
    ] {
        let outcome = run_cli(args, None);
        assert_eq!(outcome.exit_code, 0, "args: {args:?}");
        assert!(outcome.stdout.as_deref().is_some_and(|s| !s.is_empty()));
        assert!(outcome.stderr.is_none());
    }

    let expected_version = format!("spaceman {}", spaceman::build_version());
    for flag in ["--version", "-V"] {
        let outcome = run_cli(&[flag], None);
        assert_eq!(outcome.exit_code, 0);
        assert_eq!(outcome.stdout.as_deref(), Some(expected_version.as_str()));
        assert!(outcome.stderr.is_none());
    }

    let unknown = run_cli(&["help", "nonexistent"], None);
    assert_eq!(unknown.exit_code, 2);
    assert!(unknown.stderr.unwrap().contains("unknown command"));
}

#[test]
fn config_path_show_get_set_and_check_round_trip() {
    let temp = TempConfig::new("config");

    let path_out = temp.run(&["config", "path"]);
    assert_eq!(path_out.exit_code, 0);
    assert_eq!(
        path_out.stdout.as_deref(),
        Some(temp.path.display().to_string().as_str())
    );

    let path_json: Value = serde_json::from_str(
        temp.run(&["config", "path", "--json"])
            .stdout
            .as_deref()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(path_json["exists"], false);

    // Defaults when config file does not exist yet
    assert_eq!(
        temp.run(&["config", "get", "hud"]).stdout.as_deref(),
        Some("true")
    );
    assert_eq!(
        temp.run(&["config", "get", "hud_style"]).stdout.as_deref(),
        Some("icons")
    );
    assert_eq!(
        temp.run(&["config", "get", "settings.hud_style"])
            .stdout
            .as_deref(),
        Some("icons")
    );

    let check_default = temp.run(&["config", "check", "--json"]);
    assert_eq!(check_default.exit_code, 0);
    let check_json: Value = serde_json::from_str(&check_default.stdout.unwrap()).unwrap();
    assert_eq!(check_json["valid"], true);
    assert_eq!(check_json["exists"], false);

    // Mutate hud and hud_style atomically
    let set_hud = temp.run(&["config", "set", "hud", "false", "--json"]);
    assert_eq!(set_hud.exit_code, 0);
    let set_hud_json: Value = serde_json::from_str(&set_hud.stdout.unwrap()).unwrap();
    assert_eq!(set_hud_json["key"], "hud");
    assert_eq!(set_hud_json["value"], false);

    let set_style = temp.run(&["config", "set", "hud_style", "layouts"]);
    assert_eq!(set_style.exit_code, 0);
    assert_eq!(set_style.stdout.as_deref(), Some("set hud_style = layouts"));

    // Verify strict Config::parse accepts what we wrote
    let parsed = Config::parse(&temp.read()).unwrap();
    assert!(!parsed.hud());
    assert_eq!(parsed.hud_style(), HudStyle::Layouts);

    // Verify get & show reflect changes
    let get_hud_json: Value = serde_json::from_str(
        temp.run(&["config", "get", "hud", "--json"])
            .stdout
            .as_deref()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(get_hud_json["value"], false);

    let show_toml = temp.run(&["config", "show"]);
    assert_eq!(show_toml.exit_code, 0);
    let show_parsed = Config::parse(&show_toml.stdout.unwrap()).unwrap();
    assert!(!show_parsed.hud());
    assert_eq!(show_parsed.hud_style(), HudStyle::Layouts);

    let show_json: Value = serde_json::from_str(
        temp.run(&["config", "show", "--json"])
            .stdout
            .as_deref()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(show_json["version"], 1);
    assert_eq!(show_json["exists"], true);
    assert_eq!(show_json["settings"]["hud"], false);
    assert_eq!(show_json["settings"]["hud_style"], "layouts");

    let check_explicit = temp.run(&["config", "check", temp.path.to_str().unwrap()]);
    assert_eq!(check_explicit.exit_code, 0);
    assert_eq!(
        check_explicit.stdout.as_deref(),
        Some("configuration is valid")
    );
}

#[test]
fn config_gets_sets_and_shows_spacing() {
    let temp = TempConfig::new("config-spacing");
    temp.write("# Mine.\nversion = 1\n[settings]\nhud = false\n");
    assert_eq!(
        temp.run(&["config", "get", "gaps.inner"]).stdout.as_deref(),
        Some("0")
    );
    for (key, value) in [
        ("gaps.inner", "8"),
        ("settings.gaps.top", "32"),
        ("gaps.right", "12"),
        ("single_window_max_width", "1600"),
    ] {
        let set = temp.run(&["config", "set", key, value]);
        assert_eq!(set.exit_code, 0, "{:?}", set.stderr);
    }
    let spacing = Config::parse(&temp.read()).unwrap().spacing();
    assert_eq!(
        (spacing.inner, spacing.top, spacing.right, spacing.bottom),
        (8, 32, 12, 0)
    );
    assert_eq!(spacing.single_window_max_width, 1600);
    assert!(!Config::parse(&temp.read()).unwrap().hud());
    assert_eq!(
        temp.run(&["config", "get", "gaps.top"]).stdout.as_deref(),
        Some("32")
    );

    let show: Value = serde_json::from_str(
        temp.run(&["config", "show", "--json"])
            .stdout
            .as_deref()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(show["settings"]["gaps"]["inner"], 8);
    assert_eq!(show["settings"]["gaps"]["left"], 0);
    assert_eq!(show["settings"]["single_window_max_width"], 1600);
    let shown = temp.run(&["config", "show"]).stdout.unwrap();
    assert_eq!(Config::parse(&shown).unwrap().spacing(), spacing);

    // Zero is the default and leaves the file.
    assert_eq!(temp.run(&["config", "set", "gaps.inner", "0"]).exit_code, 0);
    assert!(!temp.read().contains("inner"), "{}", temp.read());

    let too_big = temp.run(&["config", "set", "gaps.inner", "49"]);
    assert_eq!(too_big.exit_code, 1);
    assert!(
        too_big
            .stderr
            .unwrap()
            .contains("settings.gaps.inner must be between 0 and 48 px, got 49")
    );
    let not_a_number = temp.run(&["config", "set", "gaps.left", "wide"]);
    assert_eq!(not_a_number.exit_code, 1);
    assert!(not_a_number.stderr.unwrap().contains("whole number"));
    assert_eq!(Config::parse(&temp.read()).unwrap().spacing().right, 12);
}

#[test]
fn config_rejects_unknown_keys_invalid_values_and_corrupt_existing_file() {
    let temp = TempConfig::new("config-invalid");

    let bad_key = temp.run(&["config", "get", "gap"]);
    assert_eq!(bad_key.exit_code, 1);
    assert!(
        bad_key
            .stderr
            .unwrap()
            .contains("valid keys: hud, hud_style")
    );

    let bad_bool = temp.run(&["config", "set", "hud", "maybe"]);
    assert_eq!(bad_bool.exit_code, 1);
    assert!(
        bad_bool
            .stderr
            .unwrap()
            .contains("expected 'true' or 'false'")
    );
    assert!(!temp.path.exists());

    let bad_style = temp.run(&["config", "set", "hud_style", "grid"]);
    assert_eq!(bad_style.exit_code, 1);
    assert!(
        bad_style
            .stderr
            .unwrap()
            .contains("expected 'icons' or 'layouts'")
    );
    assert!(!temp.path.exists());

    // Refuse to overwrite an existing invalid config file
    temp.write(
        "version = 2
",
    );
    let refused = temp.run(&["config", "set", "hud", "false"]);
    assert_eq!(refused.exit_code, 1);
    assert!(
        refused
            .stderr
            .unwrap()
            .contains("unsupported config version")
    );
    assert_eq!(
        temp.read(),
        "version = 2
"
    );
}

#[test]
fn rules_list_add_position_and_remove_by_index_and_bundle_id() {
    let temp = TempConfig::new("rules");

    assert_eq!(
        temp.run(&["rules", "list"]).stdout.as_deref(),
        Some("No rules configured.")
    );

    // Add route rule (creates version = 1 file)
    let r1 = temp.run(&[
        "rules",
        "add",
        "--app",
        "com.tinyspeck.slackmacgap",
        "--workspace",
        "2",
        "--json",
    ]);
    assert_eq!(r1.exit_code, 0);
    let r1_json: Value = serde_json::from_str(&r1.stdout.unwrap()).unwrap();
    assert_eq!(r1_json["rule"]["index"], 1);
    assert_eq!(r1_json["rule"]["bundle_id"], "com.tinyspeck.slackmacgap");
    assert_eq!(r1_json["rule"]["action"], "route");
    assert_eq!(r1_json["rule"]["workspace"], 2);

    // Add named workspace route rule (lowercase normalizes to uppercase)
    let r2 = temp.run(&["rules", "add", "--app=ai.opencode.desktop", "--workspace=d"]);
    assert_eq!(r2.exit_code, 0);
    assert_eq!(
        r2.stdout.as_deref(),
        Some("added rule 2: bundle_id=ai.opencode.desktop -> workspace D")
    );

    // Insert title-specific float rule at position 1 (first-match wins!)
    let r3 = temp.run(&[
        "rules",
        "add",
        "--app",
        "com.tinyspeck.slackmacgap",
        "--title",
        "Preferences",
        "--float",
        "--position",
        "1",
    ]);
    assert_eq!(r3.exit_code, 0);

    // Add ignore rule
    let r4 = temp.run(&["rules", "add", "--app", "com.granola.app", "--ignore"]);
    assert_eq!(r4.exit_code, 0);

    // Validate with real Config::parse and check first-match semantics
    let config = Config::parse(&temp.read()).unwrap();
    assert_eq!(
        config.disposition(
            "com.tinyspeck.slackmacgap",
            &sample_window(Some("Preferences"))
        ),
        WindowDisposition::Floating
    );
    assert_eq!(
        config.disposition("com.tinyspeck.slackmacgap", &sample_window(Some("General"))),
        WindowDisposition::Route(WorkspaceId::TWO)
    );
    assert_eq!(
        config.disposition("ai.opencode.desktop", &sample_window(None)),
        WindowDisposition::Route(WorkspaceId::named('D').unwrap())
    );
    assert_eq!(
        config.disposition("com.granola.app", &sample_window(None)),
        WindowDisposition::Ignored
    );

    // List as JSON
    let listed: Value = serde_json::from_str(
        temp.run(&["rules", "list", "--json"])
            .stdout
            .as_deref()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(listed["rules"].as_array().unwrap().len(), 4);
    assert_eq!(listed["rules"][0]["title"], "Preferences");
    assert_eq!(listed["rules"][0]["action"], "float");
    assert_eq!(listed["rules"][2]["workspace"], "D");

    // Remove by 1-based index (remove com.granola.app at index 4)
    let rem_idx = temp.run(&["rules", "remove", "4", "--json"]);
    assert_eq!(rem_idx.exit_code, 0);
    let rem_idx_json: Value = serde_json::from_str(&rem_idx.stdout.unwrap()).unwrap();
    assert_eq!(rem_idx_json["removed"].as_array().unwrap().len(), 1);
    assert_eq!(rem_idx_json["removed"][0]["bundle_id"], "com.granola.app");
    assert_eq!(rem_idx_json["remaining"], 3);

    // Remove by --app bundle-id (removes both slack rules)
    let rem_app = temp.run(&[
        "rules",
        "remove",
        "--app",
        "com.tinyspeck.slackmacgap",
        "--json",
    ]);
    assert_eq!(rem_app.exit_code, 0);
    let rem_app_json: Value = serde_json::from_str(&rem_app.stdout.unwrap()).unwrap();
    assert_eq!(rem_app_json["removed"].as_array().unwrap().len(), 2);
    assert_eq!(rem_app_json["remaining"], 1);

    // Remove last remaining rule by positional bundle-id
    let rem_pos_app = temp.run(&["rules", "remove", "ai.opencode.desktop"]);
    assert_eq!(rem_pos_app.exit_code, 0);
    assert_eq!(
        temp.run(&["rules", "list"]).stdout.as_deref(),
        Some("No rules configured.")
    );
    assert!(Config::parse(&temp.read()).is_ok());
}

#[test]
fn rules_rejects_invalid_inputs() {
    let temp = TempConfig::new("rules-invalid");

    // Missing matcher
    assert_eq!(temp.run(&["rules", "add", "--float"]).exit_code, 1);
    // Missing action
    assert_eq!(
        temp.run(&["rules", "add", "--app", "com.example.App"])
            .exit_code,
        1
    );
    // Conflicting actions
    assert_eq!(
        temp.run(&[
            "rules",
            "add",
            "--app",
            "com.example.App",
            "--float",
            "--ignore"
        ])
        .exit_code,
        1
    );
    // Invalid workspace
    assert_eq!(
        temp.run(&[
            "rules",
            "add",
            "--app",
            "com.example.App",
            "--workspace",
            "11"
        ])
        .exit_code,
        1
    );
    // Out of range position (0 or 2 when empty)
    assert_eq!(
        temp.run(&[
            "rules",
            "add",
            "--app",
            "com.example.App",
            "--float",
            "--position",
            "0"
        ])
        .exit_code,
        1
    );
    assert_eq!(
        temp.run(&[
            "rules",
            "add",
            "--app",
            "com.example.App",
            "--float",
            "--position",
            "2"
        ])
        .exit_code,
        1
    );
    // Remove out-of-range / nonexistent
    assert_eq!(temp.run(&["rules", "remove", "0"]).exit_code, 1);
    assert_eq!(temp.run(&["rules", "remove", "1"]).exit_code, 1);
    assert_eq!(
        temp.run(&["rules", "remove", "com.missing.App"]).exit_code,
        1
    );
    assert!(!temp.path.exists());
}

#[test]
fn workspaces_list_assign_and_clear_round_trip() {
    let temp = TempConfig::new("workspaces");

    // Default list has 1..=10 (1-5 main, 6-10 secondary)
    let initial: Value = serde_json::from_str(
        temp.run(&["workspaces", "list", "--json"])
            .stdout
            .as_deref()
            .unwrap(),
    )
    .unwrap();
    let entries = initial["workspaces"].as_array().unwrap();
    assert_eq!(entries.len(), 10);
    assert_eq!(entries[0]["workspace"], 1);
    assert_eq!(entries[0]["monitor"], "main");
    assert_eq!(entries[0]["configured"], false);
    assert_eq!(entries[5]["workspace"], 6);
    assert_eq!(entries[5]["monitor"], "secondary");
    assert_eq!(entries[5]["configured"], false);

    // Assign workspace 9 to main and workspace D to a named display
    let assign_9 = temp.run(&["workspaces", "assign", "9", "main", "--json"]);
    assert_eq!(assign_9.exit_code, 0);
    let assign_d = temp.run(&["workspaces", "assign", "d", "Studio Display"]);
    assert_eq!(assign_d.exit_code, 0);
    assert_eq!(
        assign_d.stdout.as_deref(),
        Some("assigned workspace D -> Studio Display")
    );

    // Updating existing workspace replaces in place without creating duplicates
    let update_9 = temp.run(&["workspaces", "assign", "9", "secondary"]);
    assert_eq!(update_9.exit_code, 0);

    let parsed = Config::parse(&temp.read()).unwrap();
    let mut workspaces = spaceman::Workspaces::default();
    parsed.configure_workspaces(&mut workspaces);
    assert_eq!(
        workspaces.assignment(WorkspaceId::named('D').unwrap()),
        &spaceman::DisplaySelector::Preferred("Studio Display".into())
    );

    // List now includes workspace D
    let after_assign: Value = serde_json::from_str(
        temp.run(&["workspaces", "list", "--json"])
            .stdout
            .as_deref()
            .unwrap(),
    )
    .unwrap();
    let entries = after_assign["workspaces"].as_array().unwrap();
    assert_eq!(entries.len(), 11);
    assert_eq!(entries[8]["workspace"], 9);
    assert_eq!(entries[8]["monitor"], "secondary");
    assert_eq!(entries[8]["configured"], true);
    assert_eq!(entries[10]["workspace"], "D");
    assert_eq!(entries[10]["monitor"], "Studio Display");
    assert_eq!(entries[10]["configured"], true);

    // Clear workspace 9
    let clear_9 = temp.run(&["workspaces", "clear", "9", "--json"]);
    assert_eq!(clear_9.exit_code, 0);
    let clear_9_json: Value = serde_json::from_str(&clear_9.stdout.unwrap()).unwrap();
    assert_eq!(clear_9_json["cleared"], true);
    assert_eq!(clear_9_json["monitor"], "secondary");

    // Clearing again is idempotent
    let clear_9_again = temp.run(&["workspaces", "clear", "9", "--json"]);
    assert_eq!(clear_9_again.exit_code, 0);
    let clear_9_again_json: Value = serde_json::from_str(&clear_9_again.stdout.unwrap()).unwrap();
    assert_eq!(clear_9_again_json["cleared"], false);

    assert!(Config::parse(&temp.read()).is_ok());
}

#[test]
fn workspaces_rejects_invalid_inputs() {
    let temp = TempConfig::new("workspaces-invalid");
    assert_eq!(
        temp.run(&["workspaces", "assign", "11", "main"]).exit_code,
        1
    );
    assert_eq!(temp.run(&["workspaces", "assign", "1", "   "]).exit_code, 1);
    assert_eq!(temp.run(&["workspaces", "clear", "AA"]).exit_code, 1);
    assert!(!temp.path.exists());
}

#[test]
fn edits_preserve_other_sections_and_named_rule_workspaces_appear_in_list() {
    let temp = TempConfig::new("preserve");
    // The example leaves `[shortcuts]` commented out so copying it changes nothing; add one
    // here so every edit below has to carry a real table through untouched.
    let mut source = include_str!("../../config.example.toml").to_owned();
    source.push_str(
        "\n[shortcuts]\ndefaults = true\n\n[shortcuts.bindings]\n\"alt+shift+b\" = \"balance\"\n",
    );
    temp.write(&source);

    // Verify example config parses cleanly through all read commands
    assert_eq!(temp.run(&["config", "check", "--json"]).exit_code, 0);
    let initial_rules: Value = serde_json::from_str(
        temp.run(&["rules", "list", "--json"])
            .stdout
            .as_deref()
            .unwrap(),
    )
    .unwrap();
    let initial_rule_count = initial_rules["rules"].as_array().unwrap().len();
    assert!(initial_rule_count >= 4);

    // Edit settings, add a named-workspace rule (W), and assign workspace 3
    assert_eq!(temp.run(&["config", "set", "hud", "false"]).exit_code, 0);
    assert_eq!(
        temp.run(&[
            "rules",
            "add",
            "--app",
            "com.example.Writer",
            "--workspace",
            "W",
        ])
        .exit_code,
        0
    );
    assert_eq!(
        temp.run(&["workspaces", "assign", "3", "secondary"])
            .exit_code,
        0
    );

    // Re-parse with strict Config::parse and verify all sections survived intact
    let parsed = Config::parse(&temp.read()).unwrap();
    assert!(!parsed.hud());
    assert_eq!(parsed.hud_style(), HudStyle::Icons);
    assert_eq!(
        parsed.disposition("com.example.Writer", &sample_window(None)),
        WindowDisposition::Route(WorkspaceId::named('W').unwrap())
    );
    assert_eq!(
        parsed.disposition("com.granola.app", &sample_window(None)),
        WindowDisposition::Ignored
    );
    // The appended [shortcuts] table survives every edit above.
    assert!(parsed.shortcuts().defaults);
    assert_eq!(parsed.shortcuts().bindings.len(), 1);
    assert_eq!(parsed.shortcuts().bindings[0].source, "alt+shift+b");

    // Named workspace W (only referenced by a rule) appears in workspaces list with default main
    let workspaces: Value = serde_json::from_str(
        temp.run(&["workspaces", "list", "--json"])
            .stdout
            .as_deref()
            .unwrap(),
    )
    .unwrap();
    let w_entry = workspaces["workspaces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["workspace"] == "W")
        .expect("workspace W from rule should appear in workspaces list");
    assert_eq!(w_entry["monitor"], "main");
    assert_eq!(w_entry["configured"], false);
}

#[test]
fn rules_add_accepts_literal_help_flag_as_title() {
    let config = TempConfig::new("rules-add-help-title");

    let added = config.run(&[
        "rules",
        "add",
        "--app",
        "com.example.Editor",
        "--title",
        "--help",
        "--float",
        "--json",
    ]);
    assert_eq!(added.exit_code, 0, "stderr: {:?}", added.stderr);
    let parsed: Value =
        serde_json::from_str(added.stdout.as_deref().unwrap()).expect("valid json output");
    assert_eq!(parsed["rule"]["title"], "--help");
    assert_eq!(parsed["rule"]["action"], "float");

    let listed = config.run(&["rules", "list", "--json"]);
    assert_eq!(listed.exit_code, 0);
    let list_json: Value =
        serde_json::from_str(listed.stdout.as_deref().unwrap()).expect("valid json list");
    assert_eq!(list_json["rules"][0]["title"], "--help");

    let help_only = config.run(&["rules", "add", "--help"]);
    assert_eq!(help_only.exit_code, 0);
    assert!(
        help_only
            .stdout
            .as_deref()
            .is_some_and(|out| out.contains("Manage ordered, first-match window rules"))
    );
}
