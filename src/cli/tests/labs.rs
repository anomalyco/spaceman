//! `[labs]` switches through `spaceman config`: read, written beside the file's other tables,
//! removed back to their default, and guarded against unknown Lab keys.

use serde_json::Value;
use spaceman::{config::Config, labs::SharedEdge};

use super::TempConfig;

#[test]
fn labs_switches_round_trip_and_survive_other_edits() {
    let temp = TempConfig::new("labs");
    temp.write("version = 1\n");
    assert_eq!(
        temp.run(&["config", "get", "labs.scrolling_strip"])
            .stdout
            .as_deref(),
        Some("false")
    );

    let set = temp.run(&["config", "set", "labs.scrolling_strip", "true"]);
    assert_eq!(set.exit_code, 0);
    assert_eq!(
        set.stdout.as_deref(),
        Some("set labs.scrolling_strip = true")
    );
    assert!(temp.read().contains("[labs]\nscrolling_strip = true"));
    assert!(Config::parse(&temp.read()).unwrap().labs().scrolling_strip);

    // Rules, workspaces and settings edits keep [labs] rather than rejecting it.
    for args in [
        &["config", "set", "hud", "false"][..],
        &["rules", "add", "--app", "com.example.Writer", "--float"][..],
        &["workspaces", "assign", "3", "secondary"][..],
    ] {
        assert_eq!(temp.run(args).exit_code, 0, "{args:?}");
        assert!(Config::parse(&temp.read()).unwrap().labs().scrolling_strip);
    }
    let show: Value = serde_json::from_str(
        temp.run(&["config", "show", "--json"])
            .stdout
            .as_deref()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(show["labs"]["scrolling_strip"], true);
    assert!(
        temp.run(&["config", "show"])
            .stdout
            .unwrap()
            .contains("[labs]")
    );

    // Off again: the table leaves the file.
    assert_eq!(
        temp.run(&["config", "set", "labs.scrolling_strip", "false"])
            .exit_code,
        0
    );
    assert!(!temp.read().contains("[labs]"));
    assert_ne!(
        temp.run(&["config", "set", "labs.scrolling_strip", "yes"])
            .exit_code,
        0
    );
    assert_ne!(temp.run(&["config", "get", "labs.warp_drive"]).exit_code, 0);
}

#[test]
fn strip_glide_tuning_round_trips_and_leaves_defaults_out() {
    let temp = TempConfig::new("labs-glide");
    temp.write("version = 1\n[labs]\nscrolling_strip = true\n");
    let get = |key: &str| temp.run(&["config", "get", key]).stdout.unwrap();
    assert_eq!(get("labs.scrolling_strip_duration_ms"), "180");
    assert_eq!(get("labs.scrolling_strip_damping"), "1");
    assert_eq!(get("labs.scrolling_strip_shared_edge"), "tuck");

    for (key, value, said) in [
        ("labs.scrolling_strip_duration_ms", "380", "380"),
        ("labs.scrolling_strip_damping", "0.78", "0.78"),
        ("labs.scrolling_strip_shared_edge", "slide", "slide"),
    ] {
        let set = temp.run(&["config", "set", key, value]);
        assert_eq!(set.exit_code, 0, "{key}");
        assert_eq!(set.stdout.unwrap(), format!("set {key} = {said}"));
        assert_eq!(get(key), said);
    }
    assert!(temp.read().contains(
        "scrolling_strip_duration_ms = 380\nscrolling_strip_damping = 0.78\n\
         scrolling_strip_shared_edge = \"slide\""
    ));
    let labs = Config::parse(&temp.read()).unwrap().labs();
    assert_eq!(labs.scrolling_strip_shared_edge, SharedEdge::Slide);
    let spring = labs.strip_spring().expect("a glide");
    assert_eq!(spring.damping_ratio, 0.78);
    assert!(
        spring.angular_frequency() < spaceman::motion::SpringParams::default().angular_frequency()
    );

    let show: Value = serde_json::from_str(
        temp.run(&["config", "show", "--json"])
            .stdout
            .as_deref()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(show["labs"]["scrolling_strip_duration_ms"], 380);
    assert_eq!(show["labs"]["scrolling_strip_shared_edge"], "slide");

    for (key, value) in [
        ("labs.scrolling_strip_duration_ms", "40"),
        ("labs.scrolling_strip_duration_ms", "fast"),
        ("labs.scrolling_strip_damping", "2"),
        ("labs.scrolling_strip_shared_edge", "curtain"),
    ] {
        let refused = temp.run(&["config", "set", key, value]);
        assert_eq!(refused.exit_code, 1, "{key} = {value}");
        assert!(refused.stderr.unwrap().starts_with("invalid value for"));
    }

    // 0 is no glide at all.
    let off = temp.run(&["config", "set", "labs.scrolling_strip_duration_ms", "0"]);
    assert_eq!(off.exit_code, 0);
    assert_eq!(get("labs.scrolling_strip_duration_ms"), "0");
    assert_eq!(
        Config::parse(&temp.read()).unwrap().labs().strip_spring(),
        None
    );

    // Back to the defaults: the keys leave the file.
    for (key, value) in [
        ("labs.scrolling_strip_duration_ms", "180"),
        ("labs.scrolling_strip_damping", "1.0"),
        ("labs.scrolling_strip_shared_edge", "tuck"),
    ] {
        assert_eq!(temp.run(&["config", "set", key, value]).exit_code, 0);
    }
    assert_eq!(
        temp.read(),
        "version = 1\n\n[labs]\nscrolling_strip = true\n"
    );
}

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
