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
    assert_eq!(doc.hud_style(), HudStyle::Icons);
    doc.set_hud_style(HudStyle::Layouts);
    assert_eq!(doc.hud_style(), HudStyle::Layouts);
    let layouts = doc.source();
    assert_eq!(
        Config::parse(&layouts).unwrap().hud_style(),
        HudStyle::Layouts
    );
    assert!(layouts.contains("hud_style = \"layouts\""));

    doc.set_hud(false);
    doc.set_hud_style(HudStyle::Icons);
    let icons = doc.source();
    assert!(!icons.contains("hud_style"));
    assert!(!Config::parse(&icons).unwrap().hud());
    doc.set_hud(true);
    assert!(!doc.source().contains("[settings]"));
}

#[test]
fn update_channel_round_trips_and_leaves_stable_out() {
    use spaceman::config::{Config, UpdateChannel};
    let mut doc = Doc::parse(SOURCE);
    assert_eq!(doc.update_channel(), UpdateChannel::Stable);
    doc.set_update_channel(UpdateChannel::Dev);
    assert_eq!(doc.update_channel(), UpdateChannel::Dev);
    let dev = doc.source();
    assert!(dev.contains("update_channel = \"dev\""));
    assert_eq!(
        Config::parse(&dev).unwrap().update_channel(),
        UpdateChannel::Dev
    );
    doc.set_update_channel(UpdateChannel::Stable);
    assert!(!doc.source().contains("[settings]"));
}

#[test]
fn mouse_follows_focus_round_trips_preserving_comments_and_leaves_off_out() {
    use spaceman::config::{Config, MouseFollowsFocus};
    let mut doc = Doc::parse(SOURCE);
    assert_eq!(doc.mouse_follows_focus(), MouseFollowsFocus::Off);
    doc.set_mouse_follows_focus(MouseFollowsFocus::Display);
    assert_eq!(doc.mouse_follows_focus(), MouseFollowsFocus::Display);
    let display = doc.source();
    assert!(display.contains("# OpenCode Browser's agent window."));
    assert!(display.contains("mouse_follows_focus = \"display\""));
    assert_eq!(
        Config::parse(&display).unwrap().mouse_follows_focus(),
        MouseFollowsFocus::Display
    );

    doc.set_mouse_follows_focus(MouseFollowsFocus::Window);
    assert_eq!(doc.mouse_follows_focus(), MouseFollowsFocus::Window);
    let window = doc.source();
    assert!(window.contains("mouse_follows_focus = \"window\""));
    assert_eq!(
        Config::parse(&window).unwrap().mouse_follows_focus(),
        MouseFollowsFocus::Window
    );

    doc.set_mouse_follows_focus(MouseFollowsFocus::Off);
    assert_eq!(doc.mouse_follows_focus(), MouseFollowsFocus::Off);
    assert!(!doc.source().contains("[settings]"));
    assert!(doc.source().contains("# OpenCode Browser's agent window."));
}

#[test]
fn spacing_round_trips_and_leaves_the_file_as_it_was() {
    use spaceman::{config::Config, spacing::Spacing};
    let mut doc = Doc::parse(SOURCE);
    assert_eq!(doc.spacing(), Spacing::NONE);
    let spacing = Spacing {
        inner: 8,
        single_window_max_width: 1680,
        ..Spacing::NONE.with_edges(12)
    };
    doc.set_spacing(spacing);
    assert_eq!(doc.spacing(), spacing);
    let saved = doc.source();
    assert_eq!(Config::parse(&saved).unwrap().spacing(), spacing);
    assert!(saved.contains("# OpenCode Browser's agent window."));
    assert!(saved.contains("[settings.gaps]\ninner = 8\n"), "{saved}");
    assert!(saved.contains("single_window_max_width = 1680"), "{saved}");
    assert_eq!(Doc::parse(&saved).spacing(), spacing);

    doc.set_spacing(Spacing { top: 30, ..spacing });
    assert_eq!(Config::parse(&doc.source()).unwrap().spacing().top, 30);
    assert_eq!(Config::parse(&doc.source()).unwrap().spacing().left, 12);

    doc.set_spacing(Spacing::NONE);
    assert_eq!(doc.source(), SOURCE);
}

#[test]
fn spacing_lands_beside_existing_settings_and_keeps_their_comments() {
    use spaceman::{config::Config, spacing::Spacing};
    let source = format!(
        "[settings]\n# Quiet.\nhud = false\n\n{}",
        &SOURCE[SOURCE.find("[[rules]]").unwrap()..]
    );
    let source = format!("version = 1\n{source}");
    let mut doc = Doc::parse(&source);
    doc.set_spacing(Spacing {
        inner: 6,
        left: 40,
        ..Spacing::NONE
    });
    let saved = doc.source();
    let config = Config::parse(&saved).unwrap();
    assert!(!config.hud());
    assert_eq!((config.spacing().inner, config.spacing().left), (6, 40));
    assert!(saved.contains("# Quiet.\nhud = false"), "{saved}");
    assert!(
        saved.find("[settings.gaps]").unwrap() < saved.find("[[rules]]").unwrap(),
        "gaps sit with their settings:\n{saved}"
    );
    doc.set_spacing(Spacing::NONE);
    assert_eq!(doc.source(), source);
}

#[test]
fn spacing_ignores_values_the_engine_would_refuse() {
    let doc = Doc::parse("version = 1\n[settings.gaps]\ninner = 999\ntop = 9\n");
    assert!(doc.error.is_some());
    assert_eq!(doc.spacing().inner, 0);
    assert_eq!(doc.spacing().top, 9);
}
