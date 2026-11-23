//! Spaceman Labs: experimental features, each behind its own switch in `config.toml`'s
//! `[labs]` table and off by default. There is no master switch; Settings › Labs lists every
//! feature in [`LabFeature::ALL`] as a card with a toggle.
//!
//! Gate a feature where its work starts (the worker action, the hotkey registration), not only
//! where its UI shows: with every switch off, Spaceman behaves exactly as if Labs did not exist.

use serde::{Deserialize, Serialize};

/// The `[labs]` table. Omitted from `config.toml` while every switch is off.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LabsConfig {
    #[serde(default)]
    pub scrolling_strip: bool,
}

impl LabsConfig {
    /// Whether every switch is off, so the table can be left out of the file.
    #[must_use]
    pub const fn is_default(&self) -> bool {
        !self.scrolling_strip
    }

    #[must_use]
    pub const fn enabled(self, feature: LabFeature) -> bool {
        match feature {
            LabFeature::ScrollingStrip => self.scrolling_strip,
        }
    }

    pub const fn set(&mut self, feature: LabFeature, enabled: bool) {
        match feature {
            LabFeature::ScrollingStrip => self.scrolling_strip = enabled,
        }
    }
}

/// One experiment in Labs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LabFeature {
    /// Wide columns on a horizontal strip per workspace ([`crate::strip`]).
    ScrollingStrip,
}

impl LabFeature {
    pub const ALL: &[Self] = &[Self::ScrollingStrip];

    /// Its key in `[labs]`.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::ScrollingStrip => "scrolling_strip",
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ScrollingStrip => "Scrolling Strip",
        }
    }

    #[must_use]
    pub const fn summary(self) -> &'static str {
        match self {
            Self::ScrollingStrip => {
                "Keep windows wide on a horizontal strip instead of shrinking tiles. Toggle per \
                 workspace with ⌥⇧S, step with ⌥H/⌥L, cycle width (50% · 66% · 100%) with ⌥-/⌥=."
            }
        }
    }

    /// The shortcut that turns it on where it applies.
    #[must_use]
    pub const fn shortcut(self) -> &'static str {
        match self {
            Self::ScrollingStrip => "⌥⇧S",
        }
    }

    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|feature| feature.id() == id)
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
            assert!(!labs.enabled(*feature));
            assert_eq!(LabFeature::from_id(feature.id()), Some(*feature));
            labs.set(*feature, true);
            assert!(labs.enabled(*feature));
            assert!(!labs.is_default());
            labs.set(*feature, false);
        }
        assert_eq!(LabFeature::from_id("warp_drive"), None);
    }

    #[test]
    fn off_message_points_at_settings() {
        assert_eq!(
            LabFeature::ScrollingStrip.off_message(),
            "Scrolling Strip is off (enable it in Settings › Labs)"
        );
    }
}
