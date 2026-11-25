//! Presents Scrolling Strip glides on the main thread: a `CADisplayLink` ticks once per display
//! refresh (120 Hz on ProMotion) only while a glide runs, each tick samples the glide's springs
//! at the frame's display time, and the interactive presenter writes the new window positions
//! through per-app lanes without waiting on any app. The worker lays the strip out for good once
//! the springs settle.

use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
    time::{Duration, Instant},
};

use super::*;
use crate::{
    PhysicalWindowId,
    macos::{DisplayFrame, DisplayLink, InteractivePresenter},
    strip::StripMotion,
};

/// How long pausing or quitting waits for the presenter to hand windows back.
const RESTORE_TIMEOUT: Duration = Duration::from_millis(250);

/// The longest frame a glide samples ahead, so a stalled run loop doesn't jump the motion.
const MAX_FRAME: Duration = Duration::from_millis(50);

/// How far into the motion the first frame shown lands: one 120 Hz frame.
const FIRST_STEP: Duration = Duration::from_millis(8);

struct Glide {
    motion: StripMotion,
    started: Instant,
    frames: u32,
    /// How far the presented motion runs behind the worker's clock, fixed at the first frame
    /// of a glide from rest and kept through retargets so the motion stays continuous.
    lag: Option<Duration>,
}

impl Glide {
    /// The moment of the worker's clock that the frame shown at `now` presents.
    fn clock(&mut self, now: Instant) -> Instant {
        let lag = *self.lag.get_or_insert_with(|| {
            now.saturating_duration_since(self.motion.issued)
                .saturating_sub(FIRST_STEP)
                .min(strip::MAX_PRESENT_LAG)
        });
        now.checked_sub(lag).unwrap_or(now)
    }
}

#[derive(Default)]
struct State {
    glides: HashMap<WorkspaceId, Glide>,
    presenter: Option<InteractivePresenter>,
    /// The last position written per window, so a column that isn't moving costs nothing.
    written: HashMap<PhysicalWindowId, (i32, i32)>,
}

impl State {
    fn presenter(&mut self) -> &mut InteractivePresenter {
        self.presenter.get_or_insert_with(InteractivePresenter::new)
    }

    /// Writes every glide's positions for `now`; a settled glide gets its exact final
    /// positions and ends. Returns whether any glide is still running.
    fn frame(&mut self, now: Instant) -> bool {
        let mut positions = Vec::new();
        let mut finished = Vec::new();
        for (workspace, glide) in &mut self.glides {
            glide.frames += 1;
            let clock = glide.clock(now);
            if glide.motion.is_settled(clock) {
                positions.extend(glide.motion.final_positions());
                finished.push(*workspace);
            } else {
                positions.extend(glide.motion.positions(clock));
            }
        }
        positions.retain(|(window, position)| {
            self.written.insert(window.clone(), *position) != Some(*position)
        });
        if !positions.is_empty() {
            self.presenter().submit_positions(&positions);
        }
        for workspace in finished {
            if let Some(glide) = self.glides.remove(&workspace) {
                log_event!(
                    "workspace {workspace}: strip glide presented in {} frames over {}ms",
                    glide.frames,
                    glide.started.elapsed().as_millis()
                );
            }
        }
        if self.glides.is_empty() {
            self.written.clear();
            if let Some(presenter) = &mut self.presenter {
                presenter.end_session();
            }
            return false;
        }
        true
    }
}

/// The main thread's side of strip glides.
pub(in crate::app) struct GlidePresenter {
    state: Rc<RefCell<State>>,
    link: Rc<RefCell<Option<DisplayLink>>>,
    /// The display the link was made for.
    link_display: Option<u32>,
}

impl GlidePresenter {
    pub(in crate::app) fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(State::default())),
            link: Rc::new(RefCell::new(None)),
            link_display: None,
        }
    }

    pub(in crate::app) fn handle(&mut self, command: strip::StripCommand) {
        match command {
            strip::StripCommand::Glide(motion) => self.start(*motion),
            strip::StripCommand::Halt(workspace) => {
                let mut state = self.state.borrow_mut();
                state.glides.remove(&workspace);
                if state.glides.is_empty() {
                    state.written.clear();
                    if let Some(presenter) = &mut state.presenter {
                        presenter.end_session();
                    }
                    drop(state);
                    self.stop_link();
                }
            }
        }
    }

    fn start(&mut self, motion: StripMotion) {
        let display = macos::display_ids().get(&motion.display_id).copied();
        {
            let mut state = self.state.borrow_mut();
            let windows: Vec<_> = motion
                .columns
                .iter()
                .map(|column| column.window.clone())
                .collect();
            state.presenter().begin_session(&windows);
            let workspace = motion.workspace;
            let (started, frames, lag) = state
                .glides
                .get(&workspace)
                .map_or((Instant::now(), 0, None), |glide| {
                    (glide.started, glide.frames, glide.lag)
                });
            state.glides.insert(
                workspace,
                Glide {
                    motion,
                    started,
                    frames,
                    lag,
                },
            );
        }
        if self.link.borrow().is_none() || self.link_display != display {
            self.make_link(display);
        }
        // Without a display link (macOS 13, no screen) the agent's 25 ms tick drives frames.
        if let Some(link) = self.link.borrow().as_ref() {
            link.start();
        }
    }

    fn make_link(&mut self, display: Option<u32>) {
        // Replace the old link before making the new one; never from inside its own callback.
        drop(self.link.borrow_mut().take());
        let state = Rc::downgrade(&self.state);
        let link = Rc::downgrade(&self.link);
        match DisplayLink::new(display, move |frame| tick(&state, &link, frame)) {
            Ok(created) => {
                *self.link.borrow_mut() = Some(created);
                self.link_display = display;
            }
            Err(error) => log_event!("strip glides fall back to the agent tick: {error}"),
        }
    }

    fn stop_link(&self) {
        if let Ok(link) = self.link.try_borrow()
            && let Some(link) = link.as_ref()
        {
            link.stop();
        }
    }

    /// For the agent's own tick: drives glides when there is no display link.
    pub(in crate::app) fn tick_without_link(&mut self) {
        if self.link.borrow().is_some() {
            return;
        }
        let Ok(mut state) = self.state.try_borrow_mut() else {
            return;
        };
        if !state.glides.is_empty() {
            state.frame(Instant::now());
        }
    }

    /// Stops every glide and hands every window back with enhanced UI restored, for pause,
    /// quit and restart.
    pub(in crate::app) fn cancel(&mut self) {
        self.stop_link();
        let mut state = self.state.borrow_mut();
        state.glides.clear();
        state.written.clear();
        if let Some(presenter) = &mut state.presenter
            && !presenter.restore_now(RESTORE_TIMEOUT)
        {
            log_event!("strip glide presenter still busy after {RESTORE_TIMEOUT:?}");
        }
    }
}

/// One display refresh: sample at the moment the frame will be on screen.
fn tick(
    state: &Weak<RefCell<State>>,
    link: &Weak<RefCell<Option<DisplayLink>>>,
    frame: DisplayFrame,
) {
    let Some(state) = state.upgrade() else {
        return;
    };
    let ahead = Duration::try_from_secs_f64(frame.target_timestamp - frame.timestamp)
        .unwrap_or_default()
        .min(MAX_FRAME);
    let running = match state.try_borrow_mut() {
        Ok(mut state) => state.frame(Instant::now() + ahead),
        Err(_) => return,
    };
    if !running
        && let Some(link) = link.upgrade()
        && let Ok(link) = link.try_borrow()
        && let Some(link) = link.as_ref()
    {
        link.stop();
    }
}
