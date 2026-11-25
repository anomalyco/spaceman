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
    /// An empty workspace chosen on another display is the focused one for commands until
    /// another window takes focus.
    #[serde(default)]
    pub empty_workspace_focus: bool,
    /// A floating window dragged to another display joins the workspace showing there.
    #[serde(default)]
    pub float_follows_display: bool,
    /// `switch next|prev` and `move next|prev [--follow]`: step through the workspaces of the
    /// focused display from the keyboard.
    #[serde(default)]
    pub workspace_steps: bool,
    /// The workspace swipe takes four fingers instead of three, leaving three-finger
    /// gestures (such as Accessibility "Three Finger Drag") to macOS.
    #[serde(default)]
    pub four_finger_swipe: bool,
    /// `settings.single_window_max_height`: a lone tile is centered at that height too, and
    /// Settings › Layout shows its row.
    #[serde(default)]
    pub single_window_height: bool,
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
            LabFeature::EmptyWorkspaceFocus => self.empty_workspace_focus,
            LabFeature::FloatFollowsDisplay => self.float_follows_display,
            LabFeature::WorkspaceSteps => self.workspace_steps,
            LabFeature::FourFingerSwipe => self.four_finger_swipe,
            LabFeature::SingleWindowHeight => self.single_window_height,
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
    /// An empty workspace chosen on another display is the focused one for commands until
    /// another window takes focus.
    EmptyWorkspaceFocus,
    /// A floating window dragged to another display joins the workspace showing there.
    FloatFollowsDisplay,
    /// `switch next|prev` and `move next|prev [--follow]` on the focused display.
    WorkspaceSteps,
    /// The workspace swipe takes four fingers instead of three.
    FourFingerSwipe,
    /// A lone tile's height cap, `settings.single_window_max_height`.
    SingleWindowHeight,
}

impl LabFeature {
    /// Every switch, in the order `[labs]` lists them.
    pub const ALL: [Self; 11] = [
        Self::Scratchpad,
        Self::Activity,
        Self::PortraitEdgeRows,
        Self::PhysicalDisplays,
        Self::RecentFocus,
        Self::SwipeSwitch,
        Self::EmptyWorkspaceFocus,
        Self::FloatFollowsDisplay,
        Self::WorkspaceSteps,
        Self::FourFingerSwipe,
        Self::SingleWindowHeight,
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
            Self::EmptyWorkspaceFocus => "empty_workspace_focus",
            Self::FloatFollowsDisplay => "float_follows_display",
            Self::WorkspaceSteps => "workspace_steps",
            Self::FourFingerSwipe => "four_finger_swipe",
            Self::SingleWindowHeight => "single_window_height",
        }
    }

    /// Parses a `[labs]` key such as `activity`.
    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|feature| feature.key() == key)
    }
}
