//! `[shortcuts]`: user-defined global shortcuts, parsed here without touching Carbon so the config
//! stays platform-neutral. `hotkeys.rs` turns each binding into a registration.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::{Direction, DisplaySelector, DisplayTarget, ResizeDirection, WorkspaceId};

/// The parsed `[shortcuts]` table.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Shortcuts {
    /// Keep the built-in shortcuts that no binding overrides.
    pub defaults: bool,
    /// User bindings in chord order. Each claims its chord before any built-in shortcut.
    pub bindings: Vec<Binding>,
    /// Register the Scratchpad Lab's shortcuts. Set from `[labs] scratchpad`, not this table.
    pub scratchpad: bool,
}

impl Default for Shortcuts {
    fn default() -> Self {
        Self {
            defaults: true,
            bindings: Vec::new(),
            scratchpad: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Binding {
    pub chord: Chord,
    /// `None` unbinds the chord, so no built-in shortcut claims it either.
    pub command: Option<Command>,
    /// The chord as written, for log messages.
    pub source: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct Chord {
    pub modifiers: Modifiers,
    pub key: Key,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, PartialOrd, Ord)]
pub struct Modifiers {
    pub command: bool,
    pub control: bool,
    pub option: bool,
    pub shift: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum Key {
    /// A character, found on whatever key types it in the active keyboard layout.
    Character(char),
    /// A number-row key, 1 through 10 (the `0` key), by physical position.
    Number(u8),
    /// A key that types no character (arrows, Return, F-keys), by macOS virtual key code.
    Code(u16),
}

/// What a binding does. The vocabulary is `spaceman ctl`'s.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    Switch(WorkspaceId),
    Move(WorkspaceId),
    BackAndForth,
    MoveWorkspace(DisplayTarget),
    Focus(Direction),
    FocusNext,
    MoveDirection(Direction),
    Layout,
    Fullscreen,
    Float,
    Resize(ResizeDirection),
    Balance,
    Retile,
}

pub const COMMAND_USAGE: &str = "switch WORKSPACE|move WORKSPACE|back-and-forth|move-workspace [next|prev|DISPLAY]|focus DIR|focus next|move-direction DIR|layout|fullscreen|float|resize shrink|resize grow|balance|retile|none";

/// `[shortcuts]` as written.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawShortcuts {
    #[serde(default = "default_true")]
    defaults: bool,
    #[serde(default)]
    bindings: BTreeMap<String, String>,
}

const fn default_true() -> bool {
    true
}

impl Default for RawShortcuts {
    fn default() -> Self {
        Self {
            defaults: true,
            bindings: BTreeMap::new(),
        }
    }
}

impl RawShortcuts {
    pub(crate) fn parse(self) -> Result<Shortcuts, String> {
        let mut bindings: Vec<Binding> = Vec::with_capacity(self.bindings.len());
        for (chord_source, command_source) in self.bindings {
            let chord = parse_chord(&chord_source)
                .map_err(|error| format!("shortcut {chord_source:?}: {error}"))?;
            if let Some(existing) = bindings.iter().find(|binding| binding.chord == chord) {
                return Err(format!(
                    "shortcuts {:?} and {chord_source:?} are the same chord",
                    existing.source
                ));
            }
            let command = parse_command(&command_source)
                .map_err(|error| format!("shortcut {chord_source:?}: {error}"))?;
            bindings.push(Binding {
                chord,
                command,
                source: chord_source,
            });
        }
        // Automatic tiling needs at least one shortcut, so an empty keymap is a config error
        // rather than a silently paused agent.
        if !self.defaults && bindings.iter().all(|binding| binding.command.is_none()) {
            return Err(
                "shortcuts.defaults = false needs at least one binding with a command".into(),
            );
        }
        Ok(Shortcuts {
            defaults: self.defaults,
            bindings,
            scratchpad: false,
        })
    }
}

/// Parses `alt+shift+left`, `cmd+ctrl+1`, `alt+-`, or `option+=`: modifiers and one key,
/// joined by `+`. A trailing `++` is the `+` key.
pub fn parse_chord(source: &str) -> Result<Chord, String> {
    let compact = source.split_whitespace().collect::<String>().to_lowercase();
    let (modifier_part, key_part) = if let Some(prefix) = compact.strip_suffix("++") {
        (prefix, "+")
    } else {
        match compact.rsplit_once('+') {
            Some((prefix, key)) => (prefix, key),
            None => ("", compact.as_str()),
        }
    };
    let mut modifiers = Modifiers::default();
    for name in modifier_part.split('+').filter(|name| !name.is_empty()) {
        let slot = match name {
            "alt" | "opt" | "option" => &mut modifiers.option,
            "cmd" | "command" => &mut modifiers.command,
            "ctrl" | "control" => &mut modifiers.control,
            "shift" => &mut modifiers.shift,
            other => {
                return Err(format!(
                    "unknown modifier {other:?}; expected alt, cmd, ctrl, or shift"
                ));
            }
        };
        if *slot {
            return Err(format!("modifier {name:?} appears twice"));
        }
        *slot = true;
    }
    if !(modifiers.option || modifiers.command || modifiers.control) {
        return Err("needs at least one of alt, cmd, or ctrl".into());
    }
    Ok(Chord {
        modifiers,
        key: parse_key(key_part)?,
    })
}

fn parse_key(name: &str) -> Result<Key, String> {
    if name.is_empty() {
        return Err("is missing a key".into());
    }
    let mut characters = name.chars();
    if let (Some(character), None) = (characters.next(), characters.next()) {
        return Ok(match character.to_digit(10).map(u8::try_from) {
            Some(Ok(0)) => Key::Number(10),
            Some(Ok(digit)) => Key::Number(digit),
            _ => Key::Character(character),
        });
    }
    if let Some(number) = name
        .strip_prefix('f')
        .and_then(|rest| rest.parse::<usize>().ok())
        && let Some(code) = number
            .checked_sub(1)
            .and_then(|index| FUNCTION_KEY_CODES.get(index))
    {
        return Ok(Key::Code(*code));
    }
    NAMED_KEY_CODES
        .iter()
        .find(|(names, _)| names.contains(&name))
        .map(|(_, code)| Key::Code(*code))
        .ok_or_else(|| format!("unknown key {name:?}"))
}

/// macOS virtual key codes (`kVK_*`) of the keys that type no character.
const NAMED_KEY_CODES: [(&[&str], u16); 14] = [
    (&["left"], 0x7b),
    (&["right"], 0x7c),
    (&["down"], 0x7d),
    (&["up"], 0x7e),
    (&["return", "enter"], 0x24),
    (&["tab"], 0x30),
    (&["space"], 0x31),
    (&["backspace", "delete"], 0x33),
    (&["escape", "esc"], 0x35),
    (&["forwarddelete"], 0x75),
    (&["home"], 0x73),
    (&["end"], 0x77),
    (&["pageup"], 0x74),
    (&["pagedown"], 0x79),
];

const FUNCTION_KEY_CODES: [u16; 12] = [
    0x7a, 0x78, 0x63, 0x76, 0x60, 0x61, 0x62, 0x64, 0x65, 0x6d, 0x67, 0x6f,
];

/// Parses a command in `spaceman ctl` syntax, or `none` to unbind.
pub fn parse_command(source: &str) -> Result<Option<Command>, String> {
    let trimmed = source.trim();
    if let Some(rest) = trimmed
        .strip_prefix("move-workspace")
        .filter(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
    {
        let target = match rest.trim() {
            "" | "next" => DisplayTarget::Next,
            "prev" => DisplayTarget::Prev,
            display => DisplayTarget::Selector(DisplaySelector::parse(display)),
        };
        return Ok(Some(Command::MoveWorkspace(target)));
    }
    let parts = trimmed.split_whitespace().collect::<Vec<_>>();
    let command = match parts.as_slice() {
        ["none"] => return Ok(None),
        ["switch", workspace] => Command::Switch(WorkspaceId::parse(workspace)?),
        ["move", workspace] => Command::Move(WorkspaceId::parse(workspace)?),
        ["back-and-forth"] => Command::BackAndForth,
        ["focus", "next"] => Command::FocusNext,
        ["focus", direction] => Command::Focus(direction.parse()?),
        ["move-direction", direction] => Command::MoveDirection(direction.parse()?),
        ["layout"] => Command::Layout,
        ["fullscreen"] => Command::Fullscreen,
        ["float"] => Command::Float,
        ["resize", "shrink"] => Command::Resize(ResizeDirection::Shrink),
        ["resize", "grow"] => Command::Resize(ResizeDirection::Grow),
        ["balance"] => Command::Balance,
        ["retile"] => Command::Retile,
        _ => {
            return Err(format!(
                "unknown command {trimmed:?}; expected {COMMAND_USAGE}"
            ));
        }
    };
    Ok(Some(command))
}

impl Command {
    /// The workspace this command needs enabled, if any.
    #[must_use]
    pub const fn workspace(&self) -> Option<WorkspaceId> {
        match self {
            Self::Switch(workspace) | Self::Move(workspace) => Some(*workspace),
            _ => None,
        }
    }

    /// A short description for logs, matching the built-in shortcuts' wording.
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            Self::Switch(workspace) => format!("switch workspace {workspace}"),
            Self::Move(workspace) => format!("move window to workspace {workspace}"),
            Self::BackAndForth => "workspace back-and-forth".into(),
            Self::MoveWorkspace(DisplayTarget::Next) => "move workspace to next display".into(),
            Self::MoveWorkspace(DisplayTarget::Prev) => "move workspace to previous display".into(),
            Self::MoveWorkspace(DisplayTarget::Selector(selector)) => {
                format!("move workspace to display {}", selector.as_str())
            }
            Self::Focus(direction) => format!("focus {direction}"),
            Self::FocusNext => "cycle workspace windows".into(),
            Self::MoveDirection(direction) => format!("move window {direction}"),
            Self::Layout => "toggle layout".into(),
            Self::Fullscreen => "toggle fullscreen layout".into(),
            Self::Float => "toggle floating window".into(),
            Self::Resize(ResizeDirection::Shrink) => "shrink layout".into(),
            Self::Resize(ResizeDirection::Grow) => "grow layout".into(),
            Self::Balance => "balance layout".into(),
            Self::Retile => "retile".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chord(option: bool, shift: bool, control: bool, command: bool, key: Key) -> Chord {
        Chord {
            modifiers: Modifiers {
                command,
                control,
                option,
                shift,
            },
            key,
        }
    }

    #[test]
    fn chords_parse_modifiers_in_any_order_and_spelling() {
        assert_eq!(
            parse_chord("alt+left").unwrap(),
            chord(true, false, false, false, Key::Code(0x7b))
        );
        assert_eq!(
            parse_chord("Shift + Option + Ctrl + Down").unwrap(),
            chord(true, true, true, false, Key::Code(0x7d))
        );
        assert_eq!(
            parse_chord("cmd+ctrl+J").unwrap(),
            chord(false, false, true, true, Key::Character('j'))
        );
        assert_eq!(
            parse_chord("alt+shift+0").unwrap(),
            chord(true, true, false, false, Key::Number(10))
        );
        assert_eq!(
            parse_chord("alt+-").unwrap(),
            chord(true, false, false, false, Key::Character('-'))
        );
        assert_eq!(
            parse_chord("alt++").unwrap(),
            chord(true, false, false, false, Key::Character('+'))
        );
        assert_eq!(
            parse_chord("ctrl+f12").unwrap(),
            chord(false, false, true, false, Key::Code(0x6f))
        );
        assert_eq!(
            parse_chord("alt+return").unwrap(),
            parse_chord("alt+enter").unwrap()
        );
    }

    #[test]
    fn chords_reject_unknown_names_and_shift_only_chords() {
        for (source, message) in [
            ("super+q", "unknown modifier"),
            ("alt+banana", "unknown key"),
            ("shift+left", "needs at least one"),
            ("left", "needs at least one"),
            ("alt+alt+left", "appears twice"),
            ("alt+", "missing a key"),
            ("alt+f13", "unknown key"),
        ] {
            let error = parse_chord(source).unwrap_err();
            assert!(error.contains(message), "{source}: {error}");
        }
    }

    #[test]
    fn commands_use_the_ctl_vocabulary() {
        let three = WorkspaceId::new(3).unwrap();
        let s = WorkspaceId::named('S').unwrap();
        for (source, expected) in [
            ("switch 3", Some(Command::Switch(three))),
            ("move S", Some(Command::Move(s))),
            ("focus left", Some(Command::Focus(Direction::Left))),
            ("focus next", Some(Command::FocusNext)),
            (
                "move-direction up",
                Some(Command::MoveDirection(Direction::Up)),
            ),
            ("layout", Some(Command::Layout)),
            ("float", Some(Command::Float)),
            ("balance", Some(Command::Balance)),
            (
                "move-workspace",
                Some(Command::MoveWorkspace(DisplayTarget::Next)),
            ),
            (
                "move-workspace prev",
                Some(Command::MoveWorkspace(DisplayTarget::Prev)),
            ),
            ("resize grow", Some(Command::Resize(ResizeDirection::Grow))),
            ("none", None),
        ] {
            assert_eq!(parse_command(source).unwrap(), expected, "{source}");
        }
        assert!(parse_command("explode").is_err());
        assert!(parse_command("switch 11").is_err());
        assert!(parse_command("focus sideways").is_err());
    }

    #[test]
    fn table_rejects_two_spellings_of_one_chord() {
        let raw: RawShortcuts = toml::from_str(
            r#"
            [bindings]
            "alt+v" = "float"
            "option+V" = "layout"
            "#,
        )
        .unwrap();
        let error = raw.parse().unwrap_err();
        assert!(error.contains("same chord"), "{error}");
    }

    #[test]
    fn table_defaults_to_keeping_built_in_shortcuts() {
        let raw: RawShortcuts = toml::from_str(
            r#"
            [bindings]
            "alt+j" = "layout"
            "alt+h" = "none"
            "#,
        )
        .unwrap();
        let shortcuts = raw.parse().unwrap();
        assert!(shortcuts.defaults);
        assert_eq!(shortcuts.bindings.len(), 2);
        assert!(
            shortcuts
                .bindings
                .iter()
                .any(|binding| binding.source == "alt+h" && binding.command.is_none())
        );
    }
}
