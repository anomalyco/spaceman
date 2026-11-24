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

/// Settings › Layout's view of `[settings.gaps]` with `gaps` inside it.
fn gaps(gaps: &str) -> Doc {
    let doc = Doc::parse(&format!("# Mine.\nversion = 1\n\n[settings.gaps]\n{gaps}"));
    assert_eq!(doc.error, None);
    doc
}

/// What the running agent would tile with once `doc` is saved.
fn saved_spacing(doc: &Doc) -> spaceman::spacing::Spacing {
    spaceman::config::Config::parse(&doc.source())
        .unwrap()
        .spacing()
}

#[test]
fn spacing_shows_the_edges_outer_resolves_to() {
    use spaceman::spacing::Spacing;
    let doc = gaps("inner = 6\nouter = 8\ntop = 0\n");
    assert_eq!(
        doc.spacing(),
        Spacing {
            inner: 6,
            top: 0,
            ..Spacing::NONE.with_edges(8)
        }
    );
    assert_eq!(doc.spacing(), saved_spacing(&doc));
}

#[test]
fn unrelated_edits_keep_outer_and_its_explicit_edges() {
    use spaceman::spacing::Spacing;
    for source in [
        "inner = 6\nouter = 8 # Everywhere.\ntop = 0\nleft = 8\n",
        "inner = 6\nouter = 0\ntop = 0\nleft = 8\n",
    ] {
        let mut doc = gaps(source);
        let before = doc.spacing();
        doc.set_spacing(before);
        doc.set_hud(false);
        doc.set_hud(true);
        doc.set_monitor(ws("M"), Some("secondary"));
        doc.set_monitor(ws("M"), None);
        assert_eq!(doc.source(), gaps(source).source());

        doc.set_spacing(Spacing {
            inner: 10,
            ..before
        });
        assert_eq!(
            doc.source(),
            gaps(&source.replace("inner = 6", "inner = 10")).source()
        );
        doc.set_spacing(Spacing {
            single_window_max_width: 1680,
            ..doc.spacing()
        });
        assert!(
            doc.source()
                .contains(&source.replace("inner = 6", "inner = 10"))
        );
        assert_eq!(
            saved_spacing(&doc),
            Spacing {
                inner: 10,
                single_window_max_width: 1680,
                ..before
            }
        );
    }
}

#[test]
fn an_edge_set_to_zero_does_not_fall_back_to_outer() {
    use spaceman::spacing::{Spacing, SpacingKey};
    for key in SpacingKey::EDGES {
        let mut doc = gaps("outer = 8\n");
        let mut spacing = doc.spacing();
        key.set(&mut spacing, 0);
        doc.set_spacing(spacing);
        assert_eq!(
            doc.source(),
            gaps(&format!("outer = 8\n{} = 0\n", key.gap_name().unwrap())).source()
        );
        assert_eq!(saved_spacing(&doc), spacing);

        // Another slider's save leaves it at zero, and it can be raised again.
        let inner = Spacing {
            inner: 4,
            ..doc.spacing()
        };
        doc.set_spacing(inner);
        assert_eq!(saved_spacing(&doc), inner);
        doc.set_spacing(Spacing::NONE.with_edges(8));
        assert_eq!(saved_spacing(&doc), Spacing::NONE.with_edges(8));
    }

    let mut doc = gaps("inner = 6\nouter = 8\n");
    doc.set_spacing(Spacing::NONE);
    assert_eq!(
        doc.source(),
        gaps("outer = 8\ntop = 0\nbottom = 0\nleft = 0\nright = 0\n").source()
    );
    assert_eq!(saved_spacing(&doc), Spacing::NONE);
}

#[test]
fn editing_one_edge_leaves_the_others_following_outer() {
    use spaceman::spacing::Spacing;
    let mut doc = gaps("inner = 6\nouter = 8\n");
    doc.set_spacing(Spacing {
        top: 30,
        ..doc.spacing()
    });
    assert_eq!(
        doc.source(),
        gaps("inner = 6\nouter = 8\ntop = 30\n").source()
    );
    // Every edge together still overrides, and the shorthand is there once they go.
    doc.set_spacing(doc.spacing().with_edges(12));
    assert_eq!(saved_spacing(&doc).uniform_edges(), Some(12));
    assert!(doc.source().contains("outer = 8\n"), "{}", doc.source());
}

#[test]
fn inline_settings_keep_outer_when_something_else_changes() {
    use spaceman::spacing::Spacing;
    for source in [
        "version = 1\n[settings]\nhud = true\ngaps = { inner = 6, outer = 8, top = 0 }\n",
        "version = 1\nsettings = { hud = true, gaps = { inner = 6, outer = 8, top = 0 } }\n",
        "version = 1\n[settings]\nhud = true\ngaps.inner = 6\ngaps.outer = 8\ngaps.top = 0\n",
        "version = 1\nsettings.hud = true\nsettings.gaps.inner = 6\nsettings.gaps.outer = 8\nsettings.gaps.top = 0\n",
    ] {
        let mut doc = Doc::parse(source);
        assert_eq!(doc.error, None);
        let before = doc.spacing();
        assert_eq!(before, saved_spacing(&doc));
        let wider = Spacing {
            inner: 10,
            ..before
        };
        doc.set_spacing(wider);
        assert_eq!(doc.source(), source.replace("inner = 6", "inner = 10"));
        doc.set_hud(false);
        assert_eq!(saved_spacing(&doc), wider, "{}", doc.source());
        assert!(
            !spaceman::config::Config::parse(&doc.source())
                .unwrap()
                .hud()
        );
        doc.set_spacing(Spacing { left: 0, ..wider });
        assert_eq!(saved_spacing(&doc), Spacing { left: 0, ..wider });
        doc.set_spacing(Spacing::NONE);
        assert_eq!(saved_spacing(&doc), Spacing::NONE, "{}", doc.source());
        assert!(doc.source().contains("outer = 8"), "{}", doc.source());
    }

    let mut doc = Doc::parse("version = 1\nsettings = { hud = false }\n");
    let spacing = Spacing::NONE.with_edges(8);
    doc.set_spacing(spacing);
    assert_eq!(saved_spacing(&doc), spacing, "{}", doc.source());
    assert!(!doc.hud());
    doc.set_spacing(Spacing::NONE);
    doc.set_hud(true);
    assert_eq!(doc.source(), "version = 1\n");
}

#[test]
fn spacing_without_outer_still_drops_an_edge_set_to_zero() {
    use spaceman::spacing::Spacing;
    let mut doc = gaps("inner = 6\nleft = 20\n");
    doc.set_spacing(Spacing {
        left: 0,
        ..doc.spacing()
    });
    assert_eq!(doc.source(), gaps("inner = 6\n").source());
    doc.set_spacing(Spacing::NONE);
    assert_eq!(doc.source(), "# Mine.\nversion = 1\n");
}

#[test]
fn unrelated_edits_leave_an_explicit_zero_for_a_later_outer() {
    use spaceman::spacing::Spacing;
    for source in [
        "version = 1\n\n[settings] # Displays.\n\n[settings.gaps]\ninner = 6\ntop = 0\n",
        "version = 1\n\n[settings] # Displays.\ngaps = { inner = 6, top = 0 } # Flush.\n",
        "version = 1\n\n[settings] # Displays.\ngaps = { top = 0 } # Flush.\n",
    ] {
        let mut doc = Doc::parse(source);
        doc.set_spacing(Spacing {
            single_window_max_width: 1680,
            ..doc.spacing()
        });
        let saved = doc.source();
        assert_eq!(
            saved.replace("single_window_max_width = 1680\n", ""),
            source
        );
        let padded = Doc::parse(&saved.replace("top = 0", "outer = 8, top = 0").replace(
            "inner = 6\nouter = 8, top = 0",
            "inner = 6\nouter = 8\ntop = 0",
        ));
        assert_eq!(padded.error, None);
        assert_eq!((padded.spacing().top, padded.spacing().left), (0, 8));
    }
}

#[test]
fn edits_keep_the_comments_around_outer() {
    use spaceman::spacing::Spacing;
    for source in [
        "version = 1\n[settings.gaps]\nouter = 8 # Around.\n# Between.\ninner = 6 # Tight.\ntop = 0\n",
        "version = 1\n[settings]\ngaps = {\n  outer = 8, # Around.\n  inner = 6, # Tight.\n  top = 0,\n}\n",
    ] {
        let mut doc = Doc::parse(source);
        assert_eq!(doc.error, None);
        // `toml_edit` reflows a multiline inline table even unedited.
        let unedited = doc.source();
        assert!(unedited.contains("outer = 8"), "{unedited}");
        doc.set_spacing(Spacing {
            inner: 10,
            ..doc.spacing()
        });
        assert_eq!(doc.source(), unedited.replace("inner = 6", "inner = 10"));
    }

    let source = "version = 1\nsettings = {\n  gaps = { outer = 8 }, # Around.\n  single_window_max_width = 1440, # Wide.\n}\n";
    let mut doc = Doc::parse(source);
    assert_eq!(doc.error, None);
    let unedited = doc.source();
    assert!(unedited.contains("1440"), "{unedited}");
    doc.set_spacing(Spacing {
        single_window_max_width: 1680,
        ..doc.spacing()
    });
    assert_eq!(doc.source(), unedited.replace("1440", "1680"));
}
