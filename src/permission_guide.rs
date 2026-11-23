//! Accessibility permission guide docked beside System Settings, plus focus
//! return tracking so Spaceman comes back to the front as soon as trust is
//! granted.

use std::{
    ffi::{CStr, c_char},
    time::{Duration, Instant},
};

use objc2::MainThreadMarker;

unsafe extern "C" {
    fn spaceman_show_accessibility_guide();
    fn spaceman_close_accessibility_guide();
    fn spaceman_accessibility_guide_active() -> bool;
    fn spaceman_accessibility_list_name() -> *const c_char;
    fn spaceman_permission_settings_state() -> i32;
    fn spaceman_prompt_accessibility_trust() -> bool;
}

/// How long after a grant System Settings may take to come back to the front
/// (for example while a Touch ID or password sheet closes) before Spaceman
/// stops waiting to return.
pub const RETURN_FROM_SETTINGS_WINDOW: Duration = Duration::from_secs(3);

/// Main-thread snapshot of System Settings and Spaceman window focus.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SettingsFocus {
    pub settings_open: bool,
    pub settings_frontmost: bool,
    pub spaceman_active: bool,
}

/// Brings Spaceman back after the user grants Accessibility in System Settings,
/// so the onboarding tour's auto-advance is immediately visible. Spaceman
/// returns only while System Settings is still frontmost: someone who switched
/// to another app keeps it.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReturnFromSettings {
    granted_at: Option<Instant>,
}

impl ReturnFromSettings {
    pub fn note_granted(&mut self, now: Instant) {
        self.granted_at = Some(now);
    }

    #[must_use]
    pub const fn is_pending(&self) -> bool {
        self.granted_at.is_some()
    }

    /// Returns `true` when Spaceman should be brought back to the front now.
    pub fn poll(&mut self, now: Instant, focus: SettingsFocus) -> bool {
        let Some(granted_at) = self.granted_at else {
            return false;
        };
        if focus.spaceman_active
            || now.saturating_duration_since(granted_at) > RETURN_FROM_SETTINGS_WINDOW
        {
            self.granted_at = None;
            return false;
        }
        if focus.settings_frontmost {
            self.granted_at = None;
            return true;
        }
        false
    }
}

/// Opens System Settings' Accessibility pane and docks Spaceman's floating drag
/// guide beside it (with a flying-tile handoff when clicked from Spaceman).
pub fn show() {
    if MainThreadMarker::new().is_some() {
        // SAFETY: Verified on the main thread; the Objective-C helper owns its panel state.
        unsafe { spaceman_show_accessibility_guide() };
    }
}

/// Dismisses the floating Accessibility guide and cancels pending cold-launch retries.
pub fn close() {
    if MainThreadMarker::new().is_some() {
        // SAFETY: Verified on the main thread; idempotent when no guide is open.
        unsafe { spaceman_close_accessibility_guide() };
    }
}

/// Whether the floating Accessibility guide is open (visible or waiting for System Settings).
#[must_use]
pub fn active() -> bool {
    // SAFETY: Verified on the main thread before touching Objective-C panel state.
    MainThreadMarker::new().is_some() && unsafe { spaceman_accessibility_guide_active() }
}

/// The name System Settings gives the Accessibility list (`"Accessibility"`
/// through macOS 26; `"Device Control and Data Access"` on macOS 27+).
#[must_use]
pub fn list_name() -> &'static str {
    // SAFETY: Returns a static NUL-terminated ASCII string literal from Objective-C.
    let name = unsafe { CStr::from_ptr(spaceman_accessibility_list_name()) };
    name.to_str().unwrap_or("Accessibility")
}

/// Registers the running binary in Accessibility and triggers macOS's prompt
/// when untrusted. Returns whether the process is already trusted.
#[must_use]
pub fn prompt_accessibility_trust() -> bool {
    // SAFETY: Objective-C helper builds an owned options dictionary and calls
    // `AXIsProcessTrustedWithOptions`.
    unsafe { spaceman_prompt_accessibility_trust() }
}

/// Reads whether System Settings has an on-screen window, whether System
/// Settings is frontmost, and whether Spaceman is active. Main thread only.
#[must_use]
pub fn settings_focus() -> SettingsFocus {
    if MainThreadMarker::new().is_none() {
        return SettingsFocus::default();
    }
    // SAFETY: Verified on the main thread; returns a bitmask of AppKit/CG state.
    let bits = unsafe { spaceman_permission_settings_state() };
    SettingsFocus {
        settings_open: bits & 1 != 0,
        settings_frontmost: bits & 2 != 0,
        spaceman_active: bits & 4 != 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn return_from_settings_brings_spaceman_back_only_from_system_settings() {
        let t0 = Instant::now();
        let in_settings = SettingsFocus {
            settings_open: true,
            settings_frontmost: true,
            spaceman_active: false,
        };
        let sheet_closing = SettingsFocus {
            settings_open: true,
            settings_frontmost: false,
            spaceman_active: false,
        };
        let back_in_spaceman = SettingsFocus {
            settings_open: true,
            settings_frontmost: false,
            spaceman_active: true,
        };

        let mut returning = ReturnFromSettings::default();
        assert!(!returning.poll(t0, in_settings));

        // Immediate grant while System Settings is frontmost returns once.
        returning.note_granted(t0);
        assert!(returning.poll(t0, in_settings));
        assert!(!returning.is_pending());
        assert!(!returning.poll(t0 + Duration::from_millis(500), in_settings));

        // Waiting for a Touch ID / password sheet to dismiss still returns within the window.
        returning.note_granted(t0);
        assert!(!returning.poll(t0, sheet_closing));
        assert!(returning.is_pending());
        assert!(returning.poll(t0 + Duration::from_secs(1), in_settings));
        assert!(!returning.is_pending());

        // Switching to another app past the window cancels instead of stealing focus later.
        returning.note_granted(t0);
        assert!(!returning.poll(t0 + Duration::from_secs(1), sheet_closing));
        assert!(!returning.poll(t0 + Duration::from_secs(4), in_settings));
        assert!(!returning.is_pending());

        // Already active in Spaceman cancels cleanly.
        returning.note_granted(t0);
        assert!(!returning.poll(t0, back_in_spaceman));
        assert!(!returning.is_pending());
    }

    #[test]
    fn accessibility_list_name_is_recognized() {
        let name = list_name();
        assert!(
            matches!(name, "Accessibility" | "Device Control and Data Access"),
            "unexpected Accessibility list name: {name}"
        );
    }
}
