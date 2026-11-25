//! Spaceman Labs: experimental features, each behind its own switch in `config.toml`'s
//! `[labs]` table and off by default. There is no master switch; Settings › Labs lists every
//! feature in [`LabFeature::ALL`] with its own.
//!
//! Gate a feature where its work starts (the worker action, the hotkey registration), not only
//! where its UI shows: with every switch off, Spaceman behaves exactly as if Labs did not exist.

mod glide;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub use glide::{
    DAMPING_KEY, DURATION_KEY, GlideDamping, GlideDuration, GlideMotion, SHARED_EDGE_KEY,
};

pub use crate::motion::SharedEdge;
use crate::{
    motion::SpringParams,
    shortcuts::{Chord, Key, Modifiers, parse_chord},
};

/// The `[labs]` table. Omitted from `config.toml` while every switch is off and every
/// shortcut is its default.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LabsConfig {
    /// Wide columns on a horizontal strip per workspace ([`crate::strip`]).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub scrolling_strip: bool,
    /// The chord that turns the strip on or off for the focused workspace, in `[shortcuts]`
    /// syntax (`"alt+shift+s"`).
    #[serde(default, skip_serializing_if = "LabShortcut::is_default")]
    pub scrolling_strip_shortcut: LabShortcut,
    /// How long a strip glide takes, in milliseconds (80 to 900, 180 by default), or 0 for no
    /// glide.
    #[serde(default, skip_serializing_if = "GlideDuration::is_default")]
    pub scrolling_strip_duration_ms: GlideDuration,
    /// The glide spring's damping ratio (0.6 to 1.3, 1.0 by default: no overshoot).
    #[serde(default, skip_serializing_if = "GlideDamping::is_default")]
    pub scrolling_strip_damping: GlideDamping,
    /// How columns meet an edge shared with another monitor: `"tuck"` (default) or `"slide"`.
    #[serde(default, skip_serializing_if = "SharedEdge::is_default")]
    pub scrolling_strip_shared_edge: SharedEdge,
    /// The i3/sway-style Scratchpad ([`crate::scratchpad`]) and its `⌥\` / `⌥⇧\` shortcuts.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub scratchpad: bool,
    /// Settings › Activity: the live arrangement log, doctor diagnostics, and process vitals.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub activity: bool,
}

/// A Labs feature's own shortcut: a chord written as a `[shortcuts]` key.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LabShortcut(pub Chord);

impl LabShortcut {
    /// `⌥⇧S`, the strip's shortcut until one is picked.
    pub const STRIP_DEFAULT: &str = "alt+shift+s";

    #[must_use]
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

impl Default for LabShortcut {
    fn default() -> Self {
        Self(Chord {
            modifiers: Modifiers {
                option: true,
                shift: true,
                ..Modifiers::default()
            },
            key: Key::Character('s'),
        })
    }
}

impl Serialize for LabShortcut {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.config_name())
    }
}

impl<'de> Deserialize<'de> for LabShortcut {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let source = String::deserialize(deserializer)?;
        parse_chord(&source)
            .map(Self)
            .map_err(|error| serde::de::Error::custom(format!("shortcut {source:?}: {error}")))
    }
}

impl LabsConfig {
    /// Whether every switch is off and every shortcut its default, so the table can be left
    /// out of the file.
    #[must_use]
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// The spring a strip glide rides, from its duration and damping, or `None` when the
    /// strip snaps without one.
    #[must_use]
    pub fn strip_spring(&self) -> Option<SpringParams> {
        glide::spring(
            self.scrolling_strip_duration_ms,
            self.scrolling_strip_damping,
        )
    }

    /// Whether `feature`'s switch is on.
    #[must_use]
    pub const fn enabled(self, feature: LabFeature) -> bool {
        match feature {
            LabFeature::ScrollingStrip => self.scrolling_strip,
            LabFeature::Scratchpad => self.scratchpad,
            LabFeature::Activity => self.activity,
        }
    }

    pub const fn set(&mut self, feature: LabFeature, enabled: bool) {
        match feature {
            LabFeature::ScrollingStrip => self.scrolling_strip = enabled,
            LabFeature::Scratchpad => self.scratchpad = enabled,
            LabFeature::Activity => self.activity = enabled,
        }
    }
}

/// Every `[labs]` switch, one per experimental feature.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LabFeature {
    /// Wide columns on a horizontal strip per workspace ([`crate::strip`]).
    ScrollingStrip,
    /// The i3/sway-style Scratchpad ([`crate::scratchpad`]) and its `⌥\` / `⌥⇧\` shortcuts.
    Scratchpad,
    /// Settings › Activity: the live arrangement log, doctor diagnostics, and process vitals.
    Activity,
}

impl LabFeature {
    /// Every switch, in the order Settings › Labs lists them.
    pub const ALL: [Self; 3] = [Self::ScrollingStrip, Self::Scratchpad, Self::Activity];

    /// The `[labs]` key that turns this feature on.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::ScrollingStrip => "scrolling_strip",
            Self::Scratchpad => "scratchpad",
            Self::Activity => "activity",
        }
    }

    /// Parses a `[labs]` switch such as `activity`.
    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|feature| feature.key() == key)
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ScrollingStrip => "Scrolling Strip",
            Self::Scratchpad => "Scratchpad",
            Self::Activity => "Activity",
        }
    }

    /// What the feature does, in one phrase.
    #[must_use]
    pub const fn summary(self) -> &'static str {
        match self {
            Self::ScrollingStrip => "Windows keep their width on a strip that glides",
            Self::Scratchpad => "Tuck windows out of sight and summon them with a shortcut",
            Self::Activity => "The arrangement log and diagnostics, as a pane in Settings",
        }
    }

    /// The feature's own recordable shortcut as `labs` has it, for the features that keep
    /// one in `[labs]`. The Scratchpad's two shortcuts live in `[shortcuts]`.
    #[must_use]
    pub const fn shortcut(self, labs: &LabsConfig) -> Option<Chord> {
        match self {
            Self::ScrollingStrip => Some(labs.scrolling_strip_shortcut.0),
            Self::Scratchpad | Self::Activity => None,
        }
    }

    /// Its shortcut's key in `[labs]`.
    #[must_use]
    pub const fn shortcut_key(self) -> Option<&'static str> {
        match self {
            Self::ScrollingStrip => Some("scrolling_strip_shortcut"),
            Self::Scratchpad | Self::Activity => None,
        }
    }

    /// What a command that needs this feature answers while it is off.
    #[must_use]
    pub fn off_message(self) -> String {
        format!("{} is off (enable it in Settings › Labs)", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_feature_is_off_by_default_and_round_trips_its_switch() {
        let mut labs = LabsConfig::default();
        assert!(labs.is_default());
        for feature in LabFeature::ALL {
            assert!(!labs.enabled(feature));
            assert_eq!(LabFeature::parse(feature.key()), Some(feature));
            labs.set(feature, true);
            assert!(labs.enabled(feature));
            assert!(!labs.is_default());
            assert_eq!(
                toml::to_string(&labs).unwrap(),
                format!("{} = true\n", feature.key()),
                "only the switch that is on is written"
            );
            labs.set(feature, false);
        }
        assert!(labs.is_default());
        assert_eq!(LabFeature::parse("warp_drive"), None);
        assert_eq!(LabFeature::parse(""), None);
    }

    #[test]
    fn the_strip_shortcut_defaults_to_option_shift_s_and_reads_any_chord() {
        assert_eq!(
            parse_chord(LabShortcut::STRIP_DEFAULT).map(LabShortcut),
            Ok(LabShortcut::default())
        );
        let labs: LabsConfig = toml::from_str("scrolling_strip = true").unwrap();
        let symbols = |labs: &LabsConfig| {
            LabFeature::ScrollingStrip
                .shortcut(labs)
                .map(|chord| chord.symbols())
        };
        assert_eq!(symbols(&labs).as_deref(), Some("⌥⇧S"));
        let picked: LabsConfig =
            toml::from_str("scrolling_strip_shortcut = \"ctrl+alt+s\"").unwrap();
        assert_eq!(symbols(&picked).as_deref(), Some("⌃⌥S"));
        // Only the strip keeps a shortcut of its own in `[labs]`.
        for feature in [LabFeature::Scratchpad, LabFeature::Activity] {
            assert_eq!(feature.shortcut(&picked), None);
            assert_eq!(feature.shortcut_key(), None);
        }
        assert!(!picked.is_default());
        assert_eq!(
            toml::to_string(&picked).unwrap(),
            "scrolling_strip_shortcut = \"ctrl+alt+s\"\n"
        );
        assert_eq!(toml::to_string(&LabsConfig::default()).unwrap(), "");
        for bad in ["\"s\"", "\"shift+s\"", "\"alt+banana\"", "5"] {
            let source = format!("scrolling_strip_shortcut = {bad}");
            assert!(toml::from_str::<LabsConfig>(&source).is_err(), "{bad}");
        }
    }

    #[test]
    fn off_message_points_at_settings() {
        assert_eq!(
            LabFeature::ScrollingStrip.off_message(),
            "Scrolling Strip is off (enable it in Settings › Labs)"
        );
    }
}
