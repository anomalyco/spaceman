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
    let spring = labs.strip_spring();
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
