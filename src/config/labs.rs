//! `[labs]`: experimental features, each behind its own switch and off by default.

use serde::{Deserialize, Serialize};

/// The `[labs]` table: experimental features, each behind its own switch and off by default.
/// With every switch off Spaceman behaves exactly as if Labs did not exist.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LabsConfig {
    /// The i3/sway-style Scratchpad ([`crate::scratchpad`]) and its `⌥\` / `⌥⇧\` shortcuts.
    #[serde(default)]
    pub scratchpad: bool,
    /// Settings › Activity: the live arrangement log, doctor diagnostics, and process vitals.
    #[serde(default)]
    pub activity: bool,
    /// Top and bottom display edge drop bands that make a half-height row on portrait displays.
    #[serde(default)]
    pub portrait_edge_rows: bool,
    /// The menu bar, its menu and the HUD's Layouts style follow the displays' physical
    /// arrangement: left to right, portrait displays drawn tall, same-named displays told apart.
    #[serde(default)]
    pub physical_displays: bool,
    /// Directional focus prefers the most recently focused of the tiles that
    /// border the focused one on the side pressed.
    #[serde(default)]
    pub recent_focus: bool,
    /// An off switch for the three-finger workspace swipe: `settings.trackpad_swipe` and the
    /// Settings › Shortcuts switch for it.
    #[serde(default)]
    pub swipe_switch: bool,
    /// The next window opened after an empty workspace is chosen on another display joins it.
    #[serde(default)]
    pub open_on_chosen_workspace: bool,
}

impl LabsConfig {
    /// Whether `feature`'s switch is on.
    #[must_use]
    pub const fn enabled(self, feature: LabFeature) -> bool {
        match feature {
            LabFeature::Scratchpad => self.scratchpad,
            LabFeature::Activity => self.activity,
            LabFeature::PortraitEdgeRows => self.portrait_edge_rows,
            LabFeature::PhysicalDisplays => self.physical_displays,
            LabFeature::RecentFocus => self.recent_focus,
            LabFeature::SwipeSwitch => self.swipe_switch,
            LabFeature::OpenOnChosenWorkspace => self.open_on_chosen_workspace,
        }
    }
}

/// Every `[labs]` switch, one per experimental feature.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LabFeature {
    /// The i3/sway-style Scratchpad ([`crate::scratchpad`]) and its `⌥\` / `⌥⇧\` shortcuts.
    Scratchpad,
    /// Settings › Activity: the live arrangement log, doctor diagnostics, and process vitals.
    Activity,
    /// Top and bottom display edge drop bands that make a half-height row on portrait displays.
    PortraitEdgeRows,
    /// The menu bar, its menu and the HUD's Layouts style in physical display order and shape.
    PhysicalDisplays,
    /// Directional focus prefers the most recently focused bordering tile.
    RecentFocus,
    /// An off switch for the three-finger workspace swipe (`settings.trackpad_swipe`).
    SwipeSwitch,
    /// The next window opened after an empty workspace is chosen on another display joins it.
    OpenOnChosenWorkspace,
}

impl LabFeature {
    /// Every switch, in the order `[labs]` lists them.
    pub const ALL: [Self; 7] = [
        Self::Scratchpad,
        Self::Activity,
        Self::PortraitEdgeRows,
        Self::PhysicalDisplays,
        Self::RecentFocus,
        Self::SwipeSwitch,
        Self::OpenOnChosenWorkspace,
    ];

    /// The `[labs]` key that turns this feature on.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Scratchpad => "scratchpad",
            Self::Activity => "activity",
            Self::PortraitEdgeRows => "portrait_edge_rows",
            Self::PhysicalDisplays => "physical_displays",
            Self::RecentFocus => "recent_focus",
            Self::SwipeSwitch => "swipe_switch",
            Self::OpenOnChosenWorkspace => "open_on_chosen_workspace",
        }
    }

    /// Parses a `[labs]` key such as `activity`.
    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|feature| feature.key() == key)
    }
}
