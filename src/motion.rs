//! Pure motion math for animated scrolling and window transitions: a closed-form camera spring,
//! a gesture velocity tracker, and per-display bounds that keep sliding columns off neighbouring
//! monitors. Nothing here touches macOS, so every rule is unit-tested on any platform.

use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use crate::Rect;

/// A spring's physical constants. The default is critically damped with `ω = √800 ≈ 28.3 rad/s`,
/// which settles a typical column-width move in about a quarter of a second without overshoot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringParams {
    pub mass: f64,
    pub damping_ratio: f64,
    pub stiffness: f64,
}

impl Default for SpringParams {
    fn default() -> Self {
        Self {
            mass: 1.0,
            damping_ratio: 1.0,
            stiffness: 800.0,
        }
    }
}

impl SpringParams {
    /// Natural angular frequency `ω = √(k / m)`, in radians per second.
    #[must_use]
    pub fn angular_frequency(self) -> f64 {
        (self.stiffness / self.mass).sqrt()
    }
}

/// Position and velocity of a spring at one instant, in the caller's units (points, points/s).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringSample {
    pub position: f64,
    pub velocity: f64,
}

/// A one-dimensional spring evaluated in closed form from the moment it was last aimed, so its
/// value at any frame time is exact however irregularly frames arrive.
///
/// [`CameraSpring::retarget`] keeps the current position and velocity, so a second `⌥L` pressed
/// mid-glide bends the same motion toward the new column instead of restarting it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraSpring {
    params: SpringParams,
    target: f64,
    /// Displacement from `target` at `start`.
    displacement: f64,
    /// Velocity at `start`.
    velocity: f64,
    start: Instant,
}

impl CameraSpring {
    /// A spring resting at `position`.
    #[must_use]
    pub fn at_rest(params: SpringParams, position: f64, now: Instant) -> Self {
        Self::with_state(params, position, 0.0, position, now)
    }

    /// A spring at `position` moving at `velocity`, heading for `target`: the hand-off from a
    /// released gesture to its settling animation.
    #[must_use]
    pub fn with_state(
        params: SpringParams,
        position: f64,
        velocity: f64,
        target: f64,
        now: Instant,
    ) -> Self {
        Self {
            params,
            target,
            displacement: position - target,
            velocity,
            start: now,
        }
    }

    #[must_use]
    pub fn params(&self) -> SpringParams {
        self.params
    }

    #[must_use]
    pub fn target(&self) -> f64 {
        self.target
    }

    /// Aims the spring at `target` from wherever it is at `now`, keeping its exact position and
    /// velocity so the motion stays continuous.
    pub fn retarget(&mut self, target: f64, now: Instant) {
        let current = self.sample(now);
        *self = Self::with_state(self.params, current.position, current.velocity, target, now);
    }

    /// Stops the spring at `position`.
    pub fn snap_to(&mut self, position: f64, now: Instant) {
        *self = Self::at_rest(self.params, position, now);
    }

    #[must_use]
    pub fn position(&self, now: Instant) -> f64 {
        self.sample(now).position
    }

    #[must_use]
    pub fn velocity(&self, now: Instant) -> f64 {
        self.sample(now).velocity
    }

    /// The spring's state at `now`. Times before the spring was last aimed read as that moment.
    #[must_use]
    pub fn sample(&self, now: Instant) -> SpringSample {
        let t = now.saturating_duration_since(self.start).as_secs_f64();
        let (displacement, velocity) = evaluate(self.params, self.displacement, self.velocity, t);
        SpringSample {
            position: self.target + displacement,
            velocity,
        }
    }

    /// Whether the spring has visibly arrived: within a quarter of a device pixel of its target
    /// and moving slowly enough that it would cover less than that in `1/ω` seconds. Checking
    /// velocity too means a fast pass through the target never counts as settled.
    #[must_use]
    pub fn is_settled(&self, now: Instant, scale: f64) -> bool {
        let tolerance = 0.25 / scale.max(f64::EPSILON);
        let sample = self.sample(now);
        (sample.position - self.target).abs() <= tolerance
            && sample.velocity.abs() / self.params.angular_frequency() <= tolerance
    }
}

/// Displacement and velocity `t` seconds after starting at `(x0, v0)` relative to the target.
fn evaluate(params: SpringParams, x0: f64, v0: f64, t: f64) -> (f64, f64) {
    let omega = params.angular_frequency();
    let zeta = params.damping_ratio;
    if !(omega.is_finite() && omega > 0.0) {
        return (0.0, 0.0);
    }
    if (zeta - 1.0).abs() < 1e-9 {
        // Critically damped: x(t) = e^(-ωt) (x0 + (ωx0 + v0) t).
        let decay = (-omega * t).exp();
        let b = omega * x0 + v0;
        (decay * (x0 + b * t), decay * (v0 - omega * b * t))
    } else if zeta < 1.0 {
        let damped = omega * (1.0 - zeta * zeta).sqrt();
        let a = zeta * omega;
        let decay = (-a * t).exp();
        let b = (v0 + a * x0) / damped;
        let (sin, cos) = (damped * t).sin_cos();
        (
            decay * (x0 * cos + b * sin),
            decay * (v0 * cos - (a * b + damped * x0) * sin),
        )
    } else {
        let root = omega * (zeta * zeta - 1.0).sqrt();
        let r1 = -zeta * omega + root;
        let r2 = -zeta * omega - root;
        let c2 = (v0 - r1 * x0) / (r2 - r1);
        let c1 = x0 - c2;
        let (e1, e2) = ((r1 * t).exp(), (r2 * t).exp());
        (c1 * e1 + c2 * e2, c1 * r1 * e1 + c2 * r2 * e2)
    }
}

/// How far back gesture velocity is measured.
pub const VELOCITY_WINDOW: Duration = Duration::from_millis(150);

/// Per-millisecond deceleration of a released touchpad fling, as in niri and libadwaita.
pub const TOUCHPAD_DECELERATION: f64 = 0.997;

#[derive(Clone, Copy, Debug, PartialEq)]
struct HistorySample {
    delta: f64,
    time: Instant,
}

/// Tracks a gesture's accumulated position and its velocity over the last
/// [`VELOCITY_WINDOW`], so a release can project where a fling would coast to.
///
/// A run of zero deltas (a finger resting on the trackpad) collapses into one anchor whose time
/// advances, so holding still decays the velocity to zero without growing the buffer.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MotionHistory {
    samples: VecDeque<HistorySample>,
    position: f64,
}

impl MotionHistory {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.samples.clear();
        self.position = 0.0;
    }

    /// Records a gesture delta that arrived at `time`.
    pub fn push(&mut self, delta: f64, time: Instant) {
        self.position += delta;
        match self.samples.back_mut() {
            Some(last) if delta == 0.0 && last.delta == 0.0 => last.time = last.time.max(time),
            _ => self.samples.push_back(HistorySample { delta, time }),
        }
        let newest = self.samples.back().map_or(time, |sample| sample.time);
        while self
            .samples
            .front()
            .is_some_and(|sample| newest.saturating_duration_since(sample.time) > VELOCITY_WINDOW)
        {
            self.samples.pop_front();
        }
    }

    /// The sum of every delta since the last reset.
    #[must_use]
    pub fn position(&self) -> f64 {
        self.position
    }

    /// Average velocity across the window in units per second; zero until two instants are known.
    #[must_use]
    pub fn velocity(&self) -> f64 {
        let (Some(first), Some(last)) = (self.samples.front(), self.samples.back()) else {
            return 0.0;
        };
        let elapsed = last
            .time
            .saturating_duration_since(first.time)
            .as_secs_f64();
        if elapsed <= 0.0 {
            return 0.0;
        }
        self.samples.iter().map(|sample| sample.delta).sum::<f64>() / elapsed
    }

    /// Where a released fling would come to rest under touchpad deceleration.
    #[must_use]
    pub fn projected_end(&self) -> f64 {
        project_end(self.position, self.velocity())
    }
}

/// Where motion starting at `position` with `velocity` (units per second) coasts to under
/// [`TOUCHPAD_DECELERATION`].
#[must_use]
pub fn project_end(position: f64, velocity: f64) -> f64 {
    position - velocity / (1000.0 * TOUCHPAD_DECELERATION.ln())
}

/// Edge slack for deciding whether a neighbouring display sits beside this one, so a Dock or
/// rounding gap between visible frames still counts as a shared seam.
const SEAM_TOLERANCE: i32 = 2;

/// Horizontal limits for a scrolling display's columns. A free edge lets a column slide off
/// screen leaving a sliver visible; an edge shared with another monitor keeps the column fully
/// inside, so a sliding window never shows up on the neighbouring display.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScrollingDisplayBounds {
    pub frame: Rect,
    pub adjacent_left: bool,
    pub adjacent_right: bool,
}

impl ScrollingDisplayBounds {
    /// Bounds for `frame` given every other display's frame.
    #[must_use]
    pub fn new(frame: Rect, others: impl IntoIterator<Item = Rect>) -> Self {
        let mut bounds = Self {
            frame,
            adjacent_left: false,
            adjacent_right: false,
        };
        let (left, right) = (frame.x, frame.x + frame.width);
        for other in others {
            if other == frame || !overlaps_vertically(frame, other) {
                continue;
            }
            bounds.adjacent_left |=
                other.x + other.width <= left + SEAM_TOLERANCE && other.x < left;
            bounds.adjacent_right |=
                other.x >= right - SEAM_TOLERANCE && other.x + other.width > right;
        }
        bounds
    }

    /// Bounds for the display with `display_id` among `displays` (each display's ID and frame),
    /// or `None` when it isn't connected.
    #[must_use]
    pub fn for_display<'a>(
        displays: impl IntoIterator<Item = (&'a str, Rect)>,
        display_id: &str,
    ) -> Option<Self> {
        let displays: Vec<_> = displays.into_iter().collect();
        let frame = displays
            .iter()
            .find(|(id, _)| *id == display_id)
            .map(|(_, frame)| *frame)?;
        Some(Self::new(
            frame,
            displays
                .iter()
                .filter(|(id, _)| *id != display_id)
                .map(|(_, frame)| *frame),
        ))
    }

    /// The lowest and highest left edge a column `width` wide may take, keeping `peek_px`
    /// (at least 1) visible past a free edge. When the column can't fit between two shared
    /// edges, both limits are the display's left edge.
    #[must_use]
    pub fn column_x_range(&self, width: i32, peek_px: i32) -> (i32, i32) {
        let width = width.max(1);
        let peek = peek_px.clamp(1, width);
        let left = self.frame.x;
        let right = self.frame.x + self.frame.width;
        let min = if self.adjacent_left {
            left
        } else {
            left - width + peek
        };
        let max = if self.adjacent_right {
            right - width
        } else {
            right - peek
        };
        (min, max.max(min))
    }

    /// `desired_x` clamped into [`Self::column_x_range`].
    #[must_use]
    pub fn clamp_column_x(&self, desired_x: i32, width: i32, peek_px: i32) -> i32 {
        let (min, max) = self.column_x_range(width, peek_px);
        desired_x.clamp(min, max)
    }
}

fn overlaps_vertically(left: Rect, right: Rect) -> bool {
    left.y < right.y + right.height && right.y < left.y + left.height
}

#[cfg(test)]
mod tests;
