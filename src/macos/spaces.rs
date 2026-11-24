//! When macOS last changed the active Space (Mission Control desktops and
//! native fullscreen Spaces). `NSWorkspace` posts the change on the main thread
//! while the slide animation can still be running; the worker reads the time
//! to hold layout writes until the slide has settled, because windows from the
//! outgoing and incoming Spaces are both on screen while it runs.

use std::{
    sync::{
        OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

/// Process-relative reference for the stored timestamp.
static EPOCH: OnceLock<Instant> = OnceLock::new();
/// Nanoseconds after `EPOCH` of the latest change, plus one (zero means none).
static CHANGED_AT: AtomicU64 = AtomicU64::new(0);
/// Set by each change until the main thread takes it.
static PENDING: AtomicBool = AtomicBool::new(false);

fn epoch() -> Instant {
    *EPOCH.get_or_init(Instant::now)
}

/// Records an active-Space change happening now.
pub(super) fn note_active_space_change() {
    let elapsed = u64::try_from(epoch().elapsed().as_nanos()).unwrap_or(u64::MAX - 1);
    CHANGED_AT.store(elapsed.saturating_add(1), Ordering::Release);
    PENDING.store(true, Ordering::Release);
}

/// When macOS last changed the active Space, if it has since launch.
#[must_use]
pub fn last_active_space_change() -> Option<Instant> {
    match CHANGED_AT.load(Ordering::Acquire) {
        0 => None,
        stored => Some(epoch() + Duration::from_nanos(stored - 1)),
    }
}

/// Whether the active Space changed since the last call.
#[must_use]
pub fn take_active_space_changed() -> bool {
    PENDING.swap(false, Ordering::AcqRel)
}
