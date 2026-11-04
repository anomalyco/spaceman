//! Three-finger horizontal trackpad swipes, as workspace steps.

/// Fingers that make a swipe. Four-finger swipes stay with macOS.
const FINGERS: usize = 3;
/// How far the fingers travel, as a fraction of the trackpad's width.
const DISTANCE: f64 = 0.15;
/// Horizontal travel must beat vertical travel by this much, so Mission Control's
/// upward swipe never switches.
const DOMINANCE: f64 = 2.0;
/// A gap this long between frames means the swipe's end was missed.
const STALE_SECONDS: f64 = 0.5;

/// Which way the fingers moved. Like macOS Spaces, swiping left goes to the
/// next workspace and swiping right to the previous one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Swipe {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum State {
    #[default]
    Ready,
    Tracking {
        x: f64,
        y: f64,
    },
    /// Fired, or a fourth finger landed: nothing more until every finger lifts.
    Spent,
}

/// Fires at most once per swipe, from the touching fingers' normalized positions.
#[derive(Debug, Default)]
pub struct SwipeTracker {
    state: State,
    last_frame: Option<f64>,
}

impl SwipeTracker {
    /// `at` is the frame's timestamp in seconds; `touches` are the fingers on the
    /// trackpad, each `(x, y)` from 0 to 1.
    pub fn frame(&mut self, at: f64, touches: &[(f64, f64)]) -> Option<Swipe> {
        if self
            .last_frame
            .is_some_and(|last| at - last > STALE_SECONDS)
        {
            self.state = State::Ready;
        }
        self.last_frame = Some(at);
        if touches.is_empty() {
            self.state = State::Ready;
            return None;
        }
        if touches.len() > FINGERS {
            self.state = State::Spent;
            return None;
        }
        if self.state == State::Spent {
            return None;
        }
        if touches.len() < FINGERS {
            self.state = State::Ready;
            return None;
        }
        let count = FINGERS as f64;
        let x = touches.iter().map(|touch| touch.0).sum::<f64>() / count;
        let y = touches.iter().map(|touch| touch.1).sum::<f64>() / count;
        let State::Tracking {
            x: start_x,
            y: start_y,
        } = self.state
        else {
            self.state = State::Tracking { x, y };
            return None;
        };
        let (dx, dy) = (x - start_x, y - start_y);
        if dx.abs() < DISTANCE || dx.abs() < DOMINANCE * dy.abs() {
            return None;
        }
        self.state = State::Spent;
        Some(if dx < 0.0 { Swipe::Left } else { Swipe::Right })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fingers(x: f64, y: f64, count: usize) -> Vec<(f64, f64)> {
        (0..count)
            .map(|finger| (x + finger as f64 * 0.05, y))
            .collect()
    }

    /// Feeds a swipe from `from` to `to` in small steps, 10 ms apart.
    fn drag(
        tracker: &mut SwipeTracker,
        start: f64,
        from: (f64, f64),
        to: (f64, f64),
        count: usize,
    ) -> Vec<Swipe> {
        (0..=20)
            .filter_map(|step| {
                let t = f64::from(step) / 20.0;
                let x = from.0 + (to.0 - from.0) * t;
                let y = from.1 + (to.1 - from.1) * t;
                tracker.frame(start + f64::from(step) * 0.01, &fingers(x, y, count))
            })
            .collect()
    }

    #[test]
    fn three_fingers_left_and_right_fire_once_each() {
        let mut tracker = SwipeTracker::default();
        assert_eq!(
            drag(&mut tracker, 0.0, (0.6, 0.5), (0.2, 0.5), 3),
            [Swipe::Left]
        );
        assert_eq!(tracker.frame(0.3, &[]), None);
        assert_eq!(
            drag(&mut tracker, 0.4, (0.2, 0.5), (0.6, 0.5), 3),
            [Swipe::Right]
        );
    }

    #[test]
    fn short_vertical_and_four_finger_swipes_do_nothing() {
        let mut tracker = SwipeTracker::default();
        assert!(drag(&mut tracker, 0.0, (0.5, 0.5), (0.4, 0.5), 3).is_empty());
        tracker.frame(0.3, &[]);
        assert!(drag(&mut tracker, 0.4, (0.5, 0.2), (0.3, 0.8), 3).is_empty());
        tracker.frame(0.7, &[]);
        assert!(drag(&mut tracker, 0.8, (0.6, 0.5), (0.1, 0.5), 4).is_empty());
    }

    #[test]
    fn lifting_from_four_fingers_to_three_does_not_fire() {
        let mut tracker = SwipeTracker::default();
        tracker.frame(0.0, &fingers(0.5, 0.5, 4));
        assert!(drag(&mut tracker, 0.01, (0.6, 0.5), (0.1, 0.5), 3).is_empty());
    }

    #[test]
    fn a_missed_lift_does_not_block_the_next_swipe() {
        let mut tracker = SwipeTracker::default();
        assert_eq!(drag(&mut tracker, 0.0, (0.6, 0.5), (0.2, 0.5), 3).len(), 1);
        assert_eq!(
            drag(&mut tracker, 2.0, (0.6, 0.5), (0.2, 0.5), 3),
            [Swipe::Left]
        );
    }
}
