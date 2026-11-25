//! The Display shortcuts Lab's rows in Settings › Shortcuts: Move workspace to previous display,
//! and a Move workspace to row for each display, named as Settings › Workspaces names it. A
//! recorded row is written by the display's UUID, so it stays with that screen when another is
//! plugged in, and `move-workspace DISPLAY` bindings written by hand keep their own rows.

use std::borrow::Cow;

use spaceman::{
    DisplaySelector, DisplayTarget,
    shortcuts::{Command, Shortcuts},
};

use super::catalog::{Action, action};
use crate::panes::workspaces::{Choice, ConnectedDisplay, choices, index_of};

/// From this many connected displays on, every display gets its own row. With fewer, Move
/// workspace to next display already reaches the other one, so only a display a binding names
/// does.
const EVERY_DISPLAY_FROM: usize = 3;

/// The note on a row whose display isn't plugged in.
pub const NOT_CONNECTED: &str = "Not connected";

/// Move workspace to previous display, with no built-in chord.
pub const PREVIOUS: Action = action(
    "move-workspace-prev",
    "Move workspace to previous display",
    "move-workspace prev",
    None,
);

/// The Lab's rows, in order: previous display, every connected display that has (or, with three
/// or more displays, could have) a row, then the bound displays that aren't plugged in.
pub fn actions(shortcuts: &Shortcuts, displays: &[ConnectedDisplay]) -> Vec<Action> {
    let connected: Vec<Choice> = choices(displays)
        .into_iter()
        .filter(|choice| choice.connected)
        .collect();
    let mut placed: Vec<(usize, Action)> = Vec::new();
    let mut covered = vec![false; connected.len()];
    let mut unplugged = Vec::new();
    for selector in bound_selectors(shortcuts) {
        if let Some(index) = index_of(&connected, &selector) {
            let choice = &connected[index];
            covered[index] = true;
            // A hand-written `main` or display name follows whichever screen answers to it.
            let note = (choice.id.as_deref() != Some(selector.as_str()))
                .then(|| format!("As \"{selector}\" in config.toml"));
            placed.push((index, display_action(&selector, &choice.title, note)));
        } else {
            unplugged.push(unplugged_action(&selector));
        }
    }
    if connected.len() >= EVERY_DISPLAY_FROM {
        for (index, choice) in connected.iter().enumerate() {
            if let (false, Some(id)) = (covered[index], &choice.id) {
                placed.push((index, display_action(id, &choice.title, None)));
            }
        }
    }
    placed.sort_by_key(|(index, _)| *index);
    let mut rows = vec![PREVIOUS];
    rows.extend(placed.into_iter().map(|(_, action)| action));
    rows.extend(unplugged);
    rows
}

/// The display every `move-workspace DISPLAY` binding names, in file order, once each.
fn bound_selectors(shortcuts: &Shortcuts) -> Vec<String> {
    let mut selectors: Vec<String> = Vec::new();
    for binding in &shortcuts.bindings {
        if let Some(Command::MoveWorkspace(DisplayTarget::Selector(selector))) = &binding.command
            && !selectors.iter().any(|known| known == selector.as_str())
        {
            selectors.push(selector.as_str().to_owned());
        }
    }
    selectors
}

fn display_action(selector: &str, display: &str, note: Option<String>) -> Action {
    Action {
        id: Cow::Owned(format!("move-workspace-display-{selector}")),
        title: Cow::Owned(format!("Move workspace to {display}")),
        command: Cow::Owned(format!("move-workspace {selector}")),
        default: None,
        note: note.map(Cow::Owned),
    }
}

/// A bound display that isn't plugged in: `main` and `secondary` by their built-in titles, a
/// name as written, and a UUID by its start, so two of them can be told apart.
fn unplugged_action(selector: &str) -> Action {
    let note = Some(NOT_CONNECTED.to_owned());
    if let Some(title) = built_in_title(selector) {
        display_action(selector, title, note)
    } else if is_uuid(selector) {
        let short: String = selector.chars().take(8).collect();
        display_action(
            selector,
            "a disconnected display",
            Some(format!("{NOT_CONNECTED} · {short}")),
        )
    } else {
        display_action(selector, selector, note)
    }
}

/// The titles Settings › Workspaces gives the built-in selectors.
fn built_in_title(selector: &str) -> Option<&'static str> {
    match DisplaySelector::parse(selector) {
        DisplaySelector::Main => Some("Main display"),
        DisplaySelector::Secondary => Some("Secondary display"),
        DisplaySelector::Preferred(_) => None,
    }
}

fn is_uuid(selector: &str) -> bool {
    selector.len() == 36
        && selector.chars().enumerate().all(|(index, character)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                character == '-'
            } else {
                character.is_ascii_hexdigit()
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panes::workspaces::{THIRD_FIXTURE_ID, fixture_pair, fixture_three};

    fn parse(bindings: &str) -> Shortcuts {
        spaceman::config::Config::parse(&format!("version = 1\n[shortcuts.bindings]\n{bindings}"))
            .unwrap()
            .shortcuts()
            .clone()
    }

    fn titles(actions: &[Action]) -> Vec<&str> {
        actions.iter().map(|action| action.title.as_ref()).collect()
    }

    #[test]
    fn every_command_parses() {
        let three = fixture_three();
        let bound = parse(&format!(
            "\"ctrl+alt+1\" = \"move-workspace main\"\n\"ctrl+alt+9\" = \"move-workspace 00000000-0000-0000-0000-000000000000\"\n\"ctrl+alt+8\" = \"move-workspace {THIRD_FIXTURE_ID}\""
        ));
        for action in actions(&bound, &three) {
            assert!(action.parsed().is_some(), "{}", action.command);
        }
    }

    #[test]
    fn two_displays_add_only_previous_until_a_display_is_bound() {
        let pair = fixture_pair();
        assert_eq!(
            titles(&actions(&Shortcuts::default(), &pair)),
            ["Move workspace to previous display"]
        );
        let bound = parse("\"ctrl+alt+2\" = \"move-workspace secondary\"");
        let rows = actions(&bound, &pair);
        assert_eq!(
            titles(&rows),
            [
                "Move workspace to previous display",
                "Move workspace to Secondary display"
            ]
        );
        assert_eq!(rows[1].command, "move-workspace secondary");
        assert_eq!(
            rows[1].note.as_deref(),
            Some("As \"secondary\" in config.toml")
        );
    }

    #[test]
    fn three_displays_offer_each_by_uuid_and_tell_same_models_apart() {
        let three = fixture_three();
        let rows = actions(&Shortcuts::default(), &three);
        assert_eq!(
            titles(&rows),
            [
                "Move workspace to previous display",
                "Move workspace to Main display",
                "Move workspace to Studio Display 1",
                "Move workspace to Studio Display 2",
            ]
        );
        let commands: Vec<String> = three
            .iter()
            .map(|display| format!("move-workspace {}", display.id))
            .collect();
        for (row, command) in rows[1..].iter().zip(&commands) {
            assert_eq!(row.command, *command);
            assert_eq!(row.note, None);
        }
    }

    #[test]
    fn a_hand_written_binding_takes_its_displays_row() {
        let three = fixture_three();
        let bound = parse("\"ctrl+alt+m\" = \"move-workspace main\"");
        let rows = actions(&bound, &three);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[1].command, "move-workspace main");
        assert_eq!(rows[1].title, "Move workspace to Main display");
    }

    #[test]
    fn an_unplugged_bound_display_keeps_a_not_connected_row() {
        let pair = fixture_pair();
        let bound = parse(&format!(
            "\"ctrl+alt+3\" = \"move-workspace {THIRD_FIXTURE_ID}\"\n\"ctrl+alt+4\" = \"move-workspace LG UltraFine\""
        ));
        let rows = actions(&bound, &pair);
        assert_eq!(
            titles(&rows),
            [
                "Move workspace to previous display",
                "Move workspace to a disconnected display",
                "Move workspace to LG UltraFine",
            ]
        );
        assert_eq!(rows[1].note.as_deref(), Some("Not connected · 8F14E45F"));
        assert_eq!(rows[2].note.as_deref(), Some(NOT_CONNECTED));

        // A hand-written `secondary` with only the main display plugged in keeps its title.
        let main_only = &pair[..1];
        let secondary = parse("\"ctrl+alt+2\" = \"move-workspace secondary\"");
        let rows = actions(&secondary, main_only);
        assert_eq!(
            titles(&rows),
            [
                "Move workspace to previous display",
                "Move workspace to Secondary display",
            ]
        );
        assert_eq!(rows[1].command, "move-workspace secondary");
        assert_eq!(rows[1].note.as_deref(), Some(NOT_CONNECTED));
        // And `main` before any display has been read.
        let main = parse("\"ctrl+alt+1\" = \"move-workspace main\"");
        assert_eq!(
            actions(&main, &[])[1].title,
            "Move workspace to Main display"
        );
    }
}
