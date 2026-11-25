//! Hears the Try it step's shortcuts.
//!
//! Carbon hotkeys take Option shortcuts before any window sees them once Spaceman has
//! registered them, so the window's own key events can't tick the lessons. A listen-only key
//! tap hears them either way, and only while the Try it step is up. It never stops a press:
//! the real shortcut still runs, on the real workspaces. It matches the step's three chords
//! by key code and modifiers and reads nothing else.

use spaceman::{macos::KeyMonitor, shortcuts::Chord};

use super::try_it;
use crate::Ui;

pub struct Listener {
    _monitor: KeyMonitor,
}

/// Starts hearing `chords` for `window`, or `None` without Accessibility.
pub fn listen(window: rune::AppHandle<Ui>, chords: [Option<Chord>; 3]) -> Option<Listener> {
    KeyMonitor::install(move |key_code, modifiers| {
        let Some(lesson) = try_it::heard(&chords, key_code, modifiers) else {
            return;
        };
        window.update_if(move |ui, tx| {
            // A press from the tap isn't input to this window, so ask for its motion.
            tx.animated();
            super::press(ui, lesson)
        });
    })
    .ok()
    .map(|monitor| Listener { _monitor: monitor })
}
