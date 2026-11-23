use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Condvar, Mutex},
    thread,
    time::{Duration, Instant},
};

use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Event {
    Suspend(i32),
    Restore(i32),
    Started(u32, Point),
    Wrote(u32, Point),
}

/// Records every sink call across lanes and can hold one app's writes until released.
#[derive(Default)]
struct Recorder {
    events: Mutex<Vec<Event>>,
    changed: Condvar,
    blocked: Mutex<HashSet<i32>>,
    unblocked: Condvar,
    enhanced: Mutex<HashMap<i32, EnhancedUi>>,
}

impl Recorder {
    fn record(&self, event: Event) {
        self.events.lock().unwrap().push(event);
        self.changed.notify_all();
    }

    fn block(&self, pid: i32) {
        self.blocked.lock().unwrap().insert(pid);
    }

    fn unblock(&self, pid: i32) {
        self.blocked.lock().unwrap().remove(&pid);
        self.unblocked.notify_all();
    }

    fn set_enhanced(&self, pid: i32, outcome: EnhancedUi) {
        self.enhanced.lock().unwrap().insert(pid, outcome);
    }

    fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }

    fn wait_until(&self, predicate: impl Fn(&[Event]) -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut events = self.events.lock().unwrap();
        while !predicate(&events) {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return false;
            }
            events = self.changed.wait_timeout(events, remaining).unwrap().0;
        }
        true
    }

    fn count(&self, event: Event) -> usize {
        self.events().iter().filter(|seen| **seen == event).count()
    }

    fn writes(&self, window_id: u32) -> Vec<Point> {
        self.events()
            .iter()
            .filter_map(|event| match event {
                Event::Wrote(id, point) if *id == window_id => Some(*point),
                _ => None,
            })
            .collect()
    }
}

#[derive(Clone)]
struct FakeBackend(Arc<Recorder>);

struct FakeSink {
    pid: i32,
    recorder: Arc<Recorder>,
}

impl PresenterBackend for FakeBackend {
    type Sink = FakeSink;

    fn open(&self, app: &AppInstanceId) -> FakeSink {
        FakeSink {
            pid: app.pid,
            recorder: Arc::clone(&self.0),
        }
    }
}

impl AppSink for FakeSink {
    fn suspend_enhanced_ui(&mut self) -> EnhancedUi {
        self.recorder.record(Event::Suspend(self.pid));
        self.recorder
            .enhanced
            .lock()
            .unwrap()
            .get(&self.pid)
            .copied()
            .unwrap_or(EnhancedUi::Suspended)
    }

    fn restore_enhanced_ui(&mut self) {
        self.recorder.record(Event::Restore(self.pid));
    }

    fn set_position(&mut self, window: &PhysicalWindowId, position: Point) -> WriteOutcome {
        self.recorder
            .record(Event::Started(window.window_id, position));
        let mut blocked = self.recorder.blocked.lock().unwrap();
        while blocked.contains(&self.pid) {
            blocked = self.recorder.unblocked.wait(blocked).unwrap();
        }
        drop(blocked);
        self.recorder
            .record(Event::Wrote(window.window_id, position));
        WriteOutcome::Written
    }
}

fn window(pid: i32, window_id: u32) -> PhysicalWindowId {
    PhysicalWindowId {
        app: AppInstanceId {
            pid,
            launched_at: 1,
        },
        window_id,
    }
}

fn presenter(
    idle_restore: Duration,
    max_lanes: usize,
) -> (InteractivePresenter<FakeBackend>, Arc<Recorder>) {
    let recorder = Arc::new(Recorder::default());
    let presenter = InteractivePresenter::with_backend(
        FakeBackend(Arc::clone(&recorder)),
        idle_restore,
        max_lanes,
    );
    (presenter, recorder)
}

const WAIT: Duration = Duration::from_secs(2);

#[test]
fn a_busy_app_skips_stale_frames_and_gets_the_newest() {
    let (mut presenter, recorder) = presenter(IDLE_RESTORE, MAX_LANES);
    let a = window(1, 10);
    recorder.block(1);
    presenter.submit_positions(&[(a.clone(), (0, 0))]);
    assert!(recorder.wait_until(|events| events.contains(&Event::Started(10, (0, 0)))));
    for step in 1..=3 {
        presenter.submit_positions(&[(a.clone(), (step, step))]);
    }
    assert_eq!(presenter.stats().coalesced, 2);
    recorder.unblock(1);
    assert!(presenter.wait_idle(WAIT));
    assert_eq!(recorder.writes(10), [(0, 0), (3, 3)]);
    assert_eq!(presenter.stats().written, 2);
}

#[test]
fn a_slow_app_never_holds_back_another() {
    let (mut presenter, recorder) = presenter(IDLE_RESTORE, MAX_LANES);
    recorder.block(1);
    presenter.submit_positions(&[(window(1, 10), (5, 5))]);
    assert!(recorder.wait_until(|events| events.contains(&Event::Started(10, (5, 5)))));
    for step in 0..3 {
        presenter.submit_positions(&[(window(2, 20), (step, 0))]);
        assert!(recorder.wait_until(|events| events.contains(&Event::Wrote(20, (step, 0)))));
    }
    assert!(
        recorder.writes(10).is_empty(),
        "the slow app is still blocked"
    );
    recorder.unblock(1);
    assert!(presenter.wait_idle(WAIT));
    assert_eq!(recorder.writes(10), [(5, 5)]);
}

#[test]
fn enhanced_ui_turns_off_once_per_session_and_back_on_once_after_it() {
    let (mut presenter, recorder) = presenter(Duration::from_millis(30), MAX_LANES);
    let (a, b) = (window(1, 10), window(2, 20));
    presenter.begin_session(&[a.clone(), b.clone()]);
    for step in 0..20 {
        presenter.submit_positions(&[(a.clone(), (step, 0)), (b.clone(), (0, step))]);
        assert!(presenter.wait_idle(WAIT));
    }
    assert_eq!(recorder.count(Event::Restore(1)), 0);
    presenter.end_session();
    assert!(recorder.wait_until(|events| {
        events.contains(&Event::Restore(1)) && events.contains(&Event::Restore(2))
    }));
    let events = recorder.events();
    for (pid, id) in [(1, 10), (2, 20)] {
        assert_eq!(recorder.count(Event::Suspend(pid)), 1);
        assert_eq!(recorder.count(Event::Restore(pid)), 1);
        let suspend = events.iter().position(|e| *e == Event::Suspend(pid));
        let restore = events.iter().position(|e| *e == Event::Restore(pid));
        let first_write = events
            .iter()
            .position(|e| matches!(e, Event::Wrote(w, _) if *w == id));
        let last_write = events
            .iter()
            .rposition(|e| matches!(e, Event::Wrote(w, _) if *w == id));
        assert!(suspend < first_write && last_write < restore, "{events:?}");
    }
}

#[test]
fn a_new_session_inside_the_idle_delay_keeps_enhanced_ui_off() {
    let (mut presenter, recorder) = presenter(Duration::from_millis(500), MAX_LANES);
    let a = window(1, 10);
    for step in 0..3 {
        presenter.begin_session(std::slice::from_ref(&a));
        presenter.submit_positions(&[(a.clone(), (step, 0))]);
        assert!(presenter.wait_idle(WAIT));
        presenter.end_session();
    }
    assert!(presenter.restore_now(WAIT));
    assert_eq!(recorder.count(Event::Suspend(1)), 1);
    assert_eq!(recorder.count(Event::Restore(1)), 1);
}

#[test]
fn dropping_the_presenter_restores_enhanced_ui() {
    let (mut presenter, recorder) = presenter(Duration::from_secs(60), MAX_LANES);
    let a = window(1, 10);
    presenter.begin_session(std::slice::from_ref(&a));
    presenter.submit_positions(&[(a, (1, 1))]);
    assert!(presenter.wait_idle(WAIT));
    drop(presenter);
    assert_eq!(recorder.count(Event::Suspend(1)), 1);
    assert_eq!(recorder.count(Event::Restore(1)), 1);
}

#[test]
fn restore_now_skips_the_idle_delay() {
    let (mut presenter, recorder) = presenter(Duration::from_secs(60), MAX_LANES);
    let a = window(1, 10);
    presenter.begin_session(std::slice::from_ref(&a));
    presenter.submit_positions(&[(a, (1, 1))]);
    assert!(presenter.restore_now(WAIT));
    assert_eq!(recorder.writes(10), [(1, 1)]);
    assert_eq!(recorder.count(Event::Restore(1)), 1);
}

#[test]
fn frames_outside_a_session_still_hold_enhanced_ui_until_idle() {
    let (mut presenter, recorder) = presenter(Duration::from_millis(30), MAX_LANES);
    presenter.submit_positions(&[(window(1, 10), (1, 1))]);
    assert!(recorder.wait_until(|events| events.contains(&Event::Restore(1))));
    assert_eq!(recorder.count(Event::Suspend(1)), 1);
    assert_eq!(recorder.writes(10), [(1, 1)]);
}

#[test]
fn unsupported_apps_are_asked_once_and_never_restored() {
    let (mut presenter, recorder) = presenter(Duration::from_secs(60), MAX_LANES);
    recorder.set_enhanced(3, EnhancedUi::Unsupported);
    let c = window(3, 30);
    for step in 0..2 {
        presenter.begin_session(std::slice::from_ref(&c));
        presenter.submit_positions(&[(c.clone(), (step, 0))]);
        assert!(presenter.restore_now(WAIT));
    }
    assert_eq!(recorder.count(Event::Suspend(3)), 1);
    assert_eq!(recorder.count(Event::Restore(3)), 0);
    assert_eq!(recorder.writes(30), [(0, 0), (1, 0)]);
}

#[test]
fn apps_with_enhanced_ui_already_off_are_left_alone() {
    let (mut presenter, recorder) = presenter(Duration::from_secs(60), MAX_LANES);
    recorder.set_enhanced(4, EnhancedUi::AlreadyOff);
    let d = window(4, 40);
    presenter.begin_session(std::slice::from_ref(&d));
    presenter.submit_positions(&[(d, (2, 2))]);
    assert!(presenter.restore_now(WAIT));
    drop(presenter);
    assert_eq!(recorder.count(Event::Restore(4)), 0);
}

#[test]
fn lanes_are_bounded_and_exit_once_idle() {
    let (mut presenter, recorder) = presenter(Duration::from_millis(10), 2);
    let windows = [window(1, 10), window(2, 20), window(3, 30)];
    presenter.begin_session(&windows);
    let frames: Vec<_> = windows.iter().map(|w| (w.clone(), (7, 7))).collect();
    presenter.submit_positions(&frames);
    assert!(presenter.wait_idle(WAIT));
    assert_eq!(presenter.active_lanes(), 2);
    for id in [10, 20, 30] {
        assert_eq!(recorder.writes(id), [(7, 7)]);
    }
    assert!(presenter.restore_now(WAIT));
    let deadline = Instant::now() + WAIT;
    while presenter.active_lanes() > 0 && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(presenter.active_lanes(), 0);
    presenter.submit_positions(&[(window(1, 10), (8, 8))]);
    assert!(presenter.wait_idle(WAIT));
    assert_eq!(recorder.writes(10), [(7, 7), (8, 8)]);
}
