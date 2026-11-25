//! Which apps the interactive presenter holds `AXEnhancedUserInterface` off for.
//!
//! The layout engine also turns enhanced UI off around its own writes and back on afterwards
//! (`apply.rs`). If it did that for an app mid-glide, Chromium and Electron windows would start
//! animating every frame again. So both sides decide under this one lock: the layout engine
//! leaves a held app's attribute alone, and when it restores an app the presenter took over
//! meanwhile it hands the restore to the presenter, which performs it when the app's hold ends.

use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex, MutexGuard, PoisonError},
};

#[derive(Debug, Default)]
struct Hold {
    sinks: usize,
    /// The layout engine turned enhanced UI off and wanted it back on while this hold ran.
    owed: bool,
}

static HOLDS: LazyLock<Mutex<HashMap<i32, Hold>>> = LazyLock::new(Mutex::default);

/// The held apps, locked: decide and write the attribute while holding it.
pub(crate) struct Holds(MutexGuard<'static, HashMap<i32, Hold>>);

pub(crate) fn lock() -> Holds {
    Holds(HOLDS.lock().unwrap_or_else(PoisonError::into_inner))
}

impl Holds {
    /// Whether the presenter is moving or about to move process `pid`'s windows.
    pub(crate) fn held(&self, pid: i32) -> bool {
        self.0.get(&pid).is_some_and(|hold| hold.sinks > 0)
    }

    /// Asks the presenter to turn `pid`'s enhanced UI back on when its hold ends.
    pub(crate) fn owe_restore(&mut self, pid: i32) {
        if let Some(hold) = self.0.get_mut(&pid) {
            hold.owed = true;
        }
    }

    /// A presenter lane opened `pid`.
    pub(crate) fn acquire(&mut self, pid: i32) {
        self.0.entry(pid).or_default().sinks += 1;
    }

    /// A presenter lane closed `pid`. Returns whether its enhanced UI is owed a restore.
    pub(crate) fn release(&mut self, pid: i32) -> bool {
        let Some(hold) = self.0.get_mut(&pid) else {
            return false;
        };
        hold.sinks = hold.sinks.saturating_sub(1);
        if hold.sinks > 0 {
            return false;
        }
        self.0.remove(&pid).is_some_and(|hold| hold.owed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_restore_owed_during_a_hold_is_paid_when_the_last_sink_closes() {
        let pid = -4242;
        let mut holds = lock();
        assert!(!holds.held(pid));
        holds.owe_restore(pid);
        holds.acquire(pid);
        holds.acquire(pid);
        assert!(holds.held(pid));
        holds.owe_restore(pid);
        assert!(!holds.release(pid));
        assert!(holds.held(pid));
        assert!(holds.release(pid));
        assert!(!holds.held(pid));
        assert!(!holds.release(pid));
    }
}
