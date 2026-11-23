use super::*;

const SOURCE: &str = r#"# Spaceman configuration.
version = 1

[[rules]]
bundle_id = "anomaly.hex"
action = "float"

[[rules]]
# OpenCode Browser's agent window.
bundle_id = "com.brave.Browser"
title = "OpenCode Browser agents - Brave"
action = "route"
workspace = 9

[[rules]]
bundle_id = "com.apple.MobileSMS"
action = "route"
workspace = "M"

[[workspaces]]
workspace = "5"
monitor = "main"
"#;

fn ws(name: &str) -> WorkspaceId {
    WorkspaceId::parse(name).unwrap()
}

#[test]
fn reads_rules_in_file_order() {
    let doc = Doc::parse(SOURCE);
    assert_eq!(doc.error, None);
    let rules = doc.rules();
    assert_eq!(rules.len(), 3);
    assert_eq!(rules[0].place, Place::Float);
    assert_eq!(rules[1].place, Place::Workspace(ws("9")));
    assert_eq!(
        rules[1].title.as_deref(),
        Some("OpenCode Browser agents - Brave")
    );
    assert_eq!(rules[2].place, Place::Workspace(ws("M")));
}

#[test]
fn moving_a_rule_keeps_comments_and_the_rest_of_the_file() {
    let mut doc = Doc::parse(SOURCE);
    doc.place(1, Place::Float);
    doc.place(0, Place::Workspace(ws("3")));
    let source = doc.source();
    assert!(source.contains("# OpenCode Browser's agent window."));
    assert!(source.contains("title = \"OpenCode Browser agents - Brave\""));
    let rules = Doc::parse(&source).rules();
    assert_eq!(rules[0].place, Place::Workspace(ws("3")));
    assert_eq!(rules[1].place, Place::Float);
    assert!(spaceman::config::Config::parse(&source).is_ok());
    assert!(
        source.contains("workspace = 3\n"),
        "numbers stay integers:\n{source}"
    );
}

#[test]
fn adds_and_removes_rules() {
    let mut doc = Doc::parse(SOURCE);
    doc.add("com.tinyspeck.slackmacgap", Place::Workspace(ws("2")));
    doc.remove(0);
    let rules = Doc::parse(&doc.source()).rules();
    assert_eq!(rules.len(), 3);
    assert_eq!(rules[0].bundle_id.as_deref(), Some("com.brave.Browser"));
    assert_eq!(
        rules[2].bundle_id.as_deref(),
        Some("com.tinyspeck.slackmacgap")
    );
    assert_eq!(rules[2].place, Place::Workspace(ws("2")));
}

#[test]
fn adds_to_an_empty_file() {
    let mut doc = Doc::parse(EMPTY);
    doc.add("anomaly.hex", Place::Float);
    assert!(spaceman::config::Config::parse(&doc.source()).is_ok());
    assert_eq!(doc.rules().len(), 1);
}

#[test]
fn workspace_displays_round_trip() {
    let mut doc = Doc::parse(SOURCE);
    assert_eq!(doc.monitor(ws("5")).as_deref(), Some("main"));
    doc.set_monitor(ws("M"), Some("secondary"));
    doc.set_monitor(ws("5"), None);
    let doc = Doc::parse(&doc.source());
    assert_eq!(doc.monitor(ws("5")), None);
    assert_eq!(doc.monitor(ws("M")).as_deref(), Some("secondary"));
}

#[test]
fn a_broken_file_is_never_saved_over() {
    let mut doc = Doc::parse("version = 1\n[[rules]]\naction = \"route\"\n");
    assert!(doc.error.is_some());
    assert!(doc.save().is_err());
}

#[test]
fn set_app_place_updates_bundle_rule_without_touching_title_rules() {
    let mut doc = Doc::parse(SOURCE);
    assert_eq!(doc.app_place("com.brave.Browser"), None);
    doc.set_app_place("com.brave.Browser", Some(Place::Workspace(ws("1"))));
    assert_eq!(
        doc.app_place("com.brave.Browser"),
        Some(Place::Workspace(ws("1")))
    );
    doc.set_app_place("com.brave.Browser", Some(Place::Float));
    assert_eq!(doc.app_place("com.brave.Browser"), Some(Place::Float));
    doc.set_app_place("com.brave.Browser", None);
    assert_eq!(doc.app_place("com.brave.Browser"), None);
    // Title-specific rule on com.brave.Browser stays intact.
    assert!(doc.rules().iter().any(|rule| {
        rule.bundle_id.as_deref() == Some("com.brave.Browser")
            && rule.title.as_deref() == Some("OpenCode Browser agents - Brave")
    }));
}

#[test]
fn hud_setting_round_trips_and_cleans_up_default_table() {
    let mut doc = Doc::parse(SOURCE);
    assert!(doc.hud());
    doc.set_hud(false);
    assert!(!doc.hud());
    let disabled = doc.source();
    assert!(!spaceman::config::Config::parse(&disabled).unwrap().hud());
    assert!(disabled.contains("# OpenCode Browser's agent window."));

    doc.set_hud(true);
    assert!(doc.hud());
    let enabled = doc.source();
    assert!(spaceman::config::Config::parse(&enabled).unwrap().hud());
    assert!(!enabled.contains("[settings]"));
}

#[test]
fn hud_style_round_trips_and_leaves_the_default_out() {
    use spaceman::config::{Config, HudStyle};
    let mut doc = Doc::parse(SOURCE);
    assert_eq!(doc.hud_style(), HudStyle::Layouts);
    doc.set_hud_style(HudStyle::Icons);
    assert_eq!(doc.hud_style(), HudStyle::Icons);
    let icons = doc.source();
    assert_eq!(Config::parse(&icons).unwrap().hud_style(), HudStyle::Icons);
    assert!(icons.contains("hud_style = \"icons\""));

    doc.set_hud(false);
    doc.set_hud_style(HudStyle::Layouts);
    let layouts = doc.source();
    assert!(!layouts.contains("hud_style"));
    assert!(!Config::parse(&layouts).unwrap().hud());
    doc.set_hud(true);
    assert!(!doc.source().contains("[settings]"));
}
