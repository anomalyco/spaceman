//! Pure rules [`super::observe_window`] uses to tell a primary window, which
//! tiles, from a dialog, settings window, or panel, which floats.

/// The state of a window's title-bar button (`AXFullScreenButton`, `AXZoomButton`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TitleButton {
    /// Not exposed.
    Absent,
    /// Exposed but disabled, for now or for good.
    Disabled,
    Enabled,
}

/// The title-bar evidence [`is_dialog`] weighs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct TitleBar {
    pub(super) fullscreen: TitleButton,
    pub(super) zoom: TitleButton,
    pub(super) close: bool,
}

impl TitleBar {
    /// Whether the window looks like an application's primary window: it can go
    /// fullscreen (even if that is blocked right now, as while a sheet is attached
    /// or for FaceTime's aspect-locked portrait window), or it can zoom (SwiftUI
    /// and scene-based windows such as FaceTime's landscape window, Phone, News,
    /// Podcasts, Games, Shortcuts and System Settings expose no fullscreen button).
    pub(super) fn is_primary(self) -> bool {
        self.fullscreen != TitleButton::Absent || self.zoom == TitleButton::Enabled
    }
}

/// Whether to skip the window entirely: an accessory application's buttonless
/// window (a menu-bar popover or overlay).
pub(super) fn is_ignored(accessory: bool, bar: TitleBar) -> bool {
    accessory && bar.fullscreen != TitleButton::Enabled && !bar.close
}

/// Whether a window floats as a dialog instead of tiling. Modal windows always
/// do. A regular application's window with a close button floats unless it looks
/// primary ([`TitleBar::is_primary`]); dialogs and settings windows expose
/// neither a fullscreen button nor an enabled zoom button. An accessory
/// application's window tiles only with an enabled fullscreen button.
pub(super) fn is_dialog(accessory: bool, modal: bool, bar: TitleBar) -> bool {
    if modal {
        return true;
    }
    if accessory {
        return bar.fullscreen != TitleButton::Enabled;
    }
    bar.close && !bar.is_primary()
}
