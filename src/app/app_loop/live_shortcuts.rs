//! `[shortcuts]` applied while Spaceman runs: a changed table re-registers the global hotkeys at
//! once, and Settings › Shortcuts holds them off while it records a chord.

use crate::shortcuts::Shortcuts;

use super::*;

/// The longest Settings may hold the hotkeys off for one recording. A recorder left listening
/// (a window that never reported losing focus) must not cost the user every shortcut.
const RECORDING_LIMIT: Duration = Duration::from_secs(60);

/// What a `[shortcuts]` change asks of the registered hotkeys.
#[derive(Debug, Eq, PartialEq)]
pub(super) enum ShortcutChange {
    /// The same hotkeys, perhaps spelled differently: nothing to do.
    Same,
    /// Keep the table for the next registration; none is installed now.
    Store,
    /// Register the new table in place of the installed hotkeys.
    Reinstall,
}

pub(super) fn shortcut_change(
    active: &Shortcuts,
    next: &Shortcuts,
    installed: bool,
) -> ShortcutChange {
    if !active.differs_from(next) {
        ShortcutChange::Same
    } else if installed {
        ShortcutChange::Reinstall
    } else {
        ShortcutChange::Store
    }
}

impl MainThreadAgent {
    /// Applies a `[shortcuts]` table read from `config.toml`, on a Settings save or a hand edit.
    /// A table that can't be registered leaves the previous hotkeys working and says why.
    pub fn apply_shortcuts(&mut self, shortcuts: Shortcuts) {
        let installed = self.hotkeys.is_some();
        match shortcut_change(&self.shortcuts, &shortcuts, installed) {
            ShortcutChange::Same => {}
            ShortcutChange::Store => {
                log_event!("hotkeys: shortcuts changed; they register when tiling starts");
                self.shortcuts = shortcuts;
            }
            ShortcutChange::Reinstall => {
                let previous = std::mem::replace(&mut self.shortcuts, shortcuts);
                self.reinstall_changed_shortcuts(previous);
            }
        }
    }

    fn reinstall_changed_shortcuts(&mut self, previous: Shortcuts) {
        let plan = match HotkeyPlan::current(&self.hotkey_workspaces, &self.shortcuts) {
            Ok(plan) => plan,
            Err(error) => {
                self.shortcuts = previous;
                self.shortcuts_not_applied(&error);
                return;
            }
        };
        // Carbon cannot register the new chords until the old ones are released.
        self.hotkeys = None;
        self.hotkey_delivery.clear();
        match Hotkeys::install(self.actions_sender.clone(), plan) {
            Ok(hotkeys) => {
                log_event!("hotkeys: reloaded {} shortcuts", hotkeys.len());
                for skipped in hotkeys.skipped() {
                    log_event!("hotkeys: {skipped}");
                }
                self.hotkeys = Some(hotkeys);
                if self
                    .last_error
                    .as_deref()
                    .is_some_and(|error| error.starts_with(NOT_APPLIED))
                {
                    self.last_error = None;
                }
                self.ui_dirty = true;
            }
            Err(error) => {
                self.shortcuts = previous;
                match HotkeyPlan::current(&self.hotkey_workspaces, &self.shortcuts) {
                    Ok(plan) => {
                        self.reinstall_hotkeys(plan);
                        if self.hotkeys.is_some() {
                            self.shortcuts_not_applied(&error);
                        }
                    }
                    Err(previous_error) => {
                        log_event!(
                            "hotkeys: could not plan the previous shortcuts: {previous_error}"
                        );
                        self.shortcuts_not_applied(&error);
                        self.stop_automatic_and_restore();
                    }
                }
            }
        }
    }

    fn shortcuts_not_applied(&mut self, error: &str) {
        let message = format!("{NOT_APPLIED}: {error}");
        log_event!("hotkeys: {message}");
        self.last_error = Some(message);
        self.ui_dirty = true;
    }

    /// Settings › Shortcuts is recording a chord (`true`) or finished (`false`). While it records,
    /// the hotkeys are unregistered so the chord reaches the recorder instead of firing; they
    /// register again, with any table saved meanwhile, the moment it stops.
    pub fn set_shortcut_recording(&mut self, recording: bool) {
        let now = Instant::now();
        let hold = recording && self.recording_since.get_or_insert(now).elapsed() < RECORDING_LIMIT;
        if !recording {
            self.recording_since = None;
        }
        if hold {
            if self.hotkeys.take().is_some() {
                log_event!("hotkeys: paused while Settings records a shortcut");
                self.hotkey_delivery.clear();
                self.hotkeys_held = true;
            }
            return;
        }
        if !std::mem::take(&mut self.hotkeys_held) || self.hotkeys.is_some() {
            return;
        }
        log_event!("hotkeys: resumed after recording");
        match HotkeyPlan::current(&self.hotkey_workspaces, &self.shortcuts) {
            Ok(plan) => self.reinstall_hotkeys(plan),
            Err(error) => {
                let error = format!("Hotkeys unavailable: {error}");
                log_event!("{error}");
                self.last_error = Some(error.clone());
                self.stop_automatic_and_restore();
                status_item::set_status(&error);
            }
        }
    }
}

const NOT_APPLIED: &str = "Shortcuts not applied";

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(bindings: &str) -> Shortcuts {
        crate::config::Config::parse(&format!("version = 1\n[shortcuts.bindings]\n{bindings}"))
            .unwrap()
            .shortcuts()
            .clone()
    }

    #[test]
    fn a_changed_table_reinstalls_only_registered_hotkeys() {
        let active = parse(r#""alt+h" = "none""#);
        let rebound = parse(r#""alt+h" = "focus right""#);
        assert_eq!(
            shortcut_change(&active, &rebound, true),
            ShortcutChange::Reinstall
        );
        assert_eq!(
            shortcut_change(&active, &rebound, false),
            ShortcutChange::Store
        );
    }

    #[test]
    fn a_respelled_or_untouched_table_is_no_change() {
        let active = parse(r#""alt+h" = "none""#);
        let respelled = parse(r#""Option + H" = "none""#);
        assert_eq!(
            shortcut_change(&active, &respelled, true),
            ShortcutChange::Same
        );
        assert_eq!(
            shortcut_change(&Shortcuts::default(), &Shortcuts::default(), true),
            ShortcutChange::Same
        );
    }
}
