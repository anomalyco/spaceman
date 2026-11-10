use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError},
    thread,
    time::{Duration, Instant},
};

/// Writes advisory state files (workspace layout, window memory) on its own
/// thread so a busy disk never holds up a workspace switch. Each path keeps
/// only its newest contents: while one write is stuck behind other processes'
/// I/O, later snapshots replace each other and the next write sends the latest.
///
/// Parked-window frames don't go through here: they must be on disk before a
/// window moves off-screen, so they stay synchronous.
pub(super) struct StateWriter {
    shared: Arc<Shared>,
    /// False when the thread couldn't start; writes then happen inline.
    threaded: bool,
}

#[derive(Default)]
struct Shared {
    queue: Mutex<Queue>,
    changed: Condvar,
}

#[derive(Default)]
struct Queue {
    pending: BTreeMap<PathBuf, (Vec<u8>, &'static str)>,
    writing: bool,
    closed: bool,
}

impl StateWriter {
    pub(super) fn spawn() -> Self {
        let shared = Arc::new(Shared::default());
        let thread_shared = Arc::clone(&shared);
        let threaded = match thread::Builder::new()
            .name("spaceman-state-writer".into())
            .spawn(move || run(&thread_shared))
        {
            Ok(_) => true,
            Err(error) => {
                log_event!("state writer unavailable, writing inline: {error}");
                false
            }
        };
        Self { shared, threaded }
    }

    pub(super) fn write(&self, path: PathBuf, bytes: Vec<u8>, label: &'static str) {
        if !self.threaded {
            write(&path, &bytes, label);
            return;
        }
        self.shared.lock().pending.insert(path, (bytes, label));
        self.shared.changed.notify_all();
    }

    /// Waits up to `timeout` for every queued write to land. Returns `false` if
    /// some were still pending.
    pub(super) fn flush(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let mut queue = self.shared.lock();
        while !queue.pending.is_empty() || queue.writing {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return false;
            };
            queue = self
                .shared
                .changed
                .wait_timeout(queue, remaining)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        true
    }
}

impl Drop for StateWriter {
    fn drop(&mut self) {
        self.shared.lock().closed = true;
        self.shared.changed.notify_all();
    }
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, Queue> {
        self.queue.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn run(shared: &Shared) {
    let mut queue = shared.lock();
    loop {
        let Some((path, (bytes, label))) = queue.pending.pop_first() else {
            if queue.closed {
                return;
            }
            queue = shared
                .changed
                .wait(queue)
                .unwrap_or_else(PoisonError::into_inner);
            continue;
        };
        queue.writing = true;
        drop(queue);
        write(&path, &bytes, label);
        queue = shared.lock();
        queue.writing = false;
        shared.changed.notify_all();
    }
}

fn write(path: &Path, bytes: &[u8], label: &str) {
    let started = Instant::now();
    if let Err(error) = crate::state_file::write_atomic(path, bytes) {
        log_event!("failed to write {label} state: {error}");
    }
    let elapsed = started.elapsed();
    if elapsed >= Duration::from_millis(500) {
        log_event!("slow {label} state write: {}ms", elapsed.as_millis());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_the_latest_contents_and_flushes() {
        let directory =
            std::env::temp_dir().join(format!("spaceman-state-writer-{}", std::process::id()));
        let path = directory.join("state.json");
        let writer = StateWriter::spawn();
        for value in 0..50 {
            writer.write(path.clone(), value.to_string().into_bytes(), "test");
        }
        assert!(writer.flush(Duration::from_secs(10)));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "49");
        std::fs::remove_dir_all(directory).unwrap();
    }
}
