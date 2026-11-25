//! `[labs]` switches through `spaceman config`: read, written beside the file's other tables,
//! removed back to their default, and guarded against unknown Lab keys.

use serde_json::Value;
use spaceman::config::Config;

use super::TempConfig;

#[test]
fn config_gets_sets_and_resets_labs_activity() {
    let temp = TempConfig::new("config-labs-activity");
    temp.write("# Mine.\nversion = 1\n[settings]\nhud = false\n");
    assert_eq!(
        temp.run(&["config", "get", "labs.activity"])
            .stdout
            .as_deref(),
        Some("false")
    );

    // On writes the switch beside the tables the file already has.
    let set = temp.run(&["config", "set", "labs.activity", "true"]);
    assert_eq!(set.exit_code, 0, "{:?}", set.stderr);
    assert_eq!(set.stdout.as_deref(), Some("set labs.activity = true"));
    let source = temp.read();
    assert!(source.contains("[labs]"), "{source}");
    assert!(source.contains("activity = true"), "{source}");
    assert!(source.contains("hud = false"), "{source}");
    assert!(Config::parse(&source).unwrap().labs().activity);

    assert_eq!(
        temp.run(&["config", "get", "labs.activity"])
            .stdout
            .as_deref(),
        Some("true")
    );
    let show: Value = serde_json::from_str(
        temp.run(&["config", "show", "--json"])
            .stdout
            .as_deref()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(show["labs"]["activity"], true);
    // Unset switches stay out of the JSON instead of printing null.
    assert_eq!(show["labs"].get("workspace_steps"), None);
    assert_eq!(show["labs"].get("scratchpad"), None);
    let shown = Config::parse(&temp.run(&["config", "show"]).stdout.unwrap()).unwrap();
    assert!(shown.labs().activity);

    // Reset removes the switch and the table it emptied.
    let reset = temp.run(&["config", "reset", "labs.activity"]);
    assert_eq!(reset.exit_code, 0, "{:?}", reset.stderr);
    assert_eq!(reset.stdout.as_deref(), Some("reset labs.activity = false"));
    let source = temp.read();
    assert!(!source.contains("[labs]"), "{source}");
    assert!(!Config::parse(&source).unwrap().labs().activity);

    // Off removes only its own key; other switches keep the table.
    temp.write("version = 1\n[labs]\nscratchpad = true\nactivity = true\n");
    let off = temp.run(&["config", "set", "labs.activity", "false"]);
    assert_eq!(off.exit_code, 0, "{:?}", off.stderr);
    let source = temp.read();
    assert!(!source.contains("activity"), "{source}");
    assert!(source.contains("scratchpad = true"), "{source}");
    assert!(!Config::parse(&source).unwrap().labs().activity);

    // Every Lab switch shares the same path.
    assert_eq!(
        temp.run(&["config", "get", "labs.scratchpad"])
            .stdout
            .as_deref(),
        Some("true")
    );

    let bad = temp.run(&["config", "set", "labs.activity", "yes"]);
    assert_eq!(bad.exit_code, 1);
    assert!(bad.stderr.unwrap().contains("expected 'true' or 'false'"));
    let unknown = temp.run(&["config", "get", "labs.warp_drive"]);
    assert_eq!(unknown.exit_code, 1);
    assert!(unknown.stderr.unwrap().contains("unknown Lab"));
}

#[test]
fn config_show_lists_only_the_lab_switches_the_file_sets() {
    let temp = TempConfig::new("config-labs-show-unset");
    temp.write("version = 1\n");
    let set = temp.run(&["config", "set", "labs.scratchpad", "true"]);
    assert_eq!(set.exit_code, 0, "{:?}", set.stderr);

    let shown = temp.run(&["config", "show", "--json"]).stdout.unwrap();
    let show: Value = serde_json::from_str(&shown).unwrap();
    let labs = show["labs"].as_object().unwrap();
    assert_eq!(labs.get("scratchpad"), Some(&Value::Bool(true)));
    for unset in [
        "activity",
        "portrait_edge_rows",
        "physical_displays",
        "recent_focus",
        "swipe_switch",
    ] {
        assert!(!labs.contains_key(unset), "{unset} in {shown}");
    }
    let text = temp.run(&["config", "show"]).stdout.unwrap();
    assert!(!text.contains("swipe_switch"), "{text}");
}
