use std::time::{Duration, Instant};

/// How long a key press the keyboard tap saw may wait for Carbon to deliver it.
const DELIVERY_DEADLINE: Duration = Duration::from_millis(750);
/// How far a Carbon delivery may come before the tap reports the same press.
const DELIVERY_LEAD: Duration = Duration::from_millis(250);

/// Pairs key presses the keyboard tap saw with the hotkeys Carbon delivered, so
/// a press macOS never delivered is noticed instead of silently doing nothing.
///
/// The tap and Carbon report the same press separately, in either order, both
/// on the main thread. A delivery matches at most one press.
#[derive(Debug, Default)]
pub(crate) struct DeliveryWatch {
    pending: Option<(u32, Instant)>,
    delivered: Option<(u32, Instant)>,
}

impl DeliveryWatch {
    pub(crate) fn pressed(&mut self, id: u32, now: Instant) {
        if self.delivered.is_some_and(|(delivered, at)| {
            delivered == id && now.saturating_duration_since(at) <= DELIVERY_LEAD
        }) {
            self.delivered = None;
            return;
        }
        self.pending = Some((id, now));
    }

    pub(crate) fn delivered(&mut self, id: u32, now: Instant) {
        if self.pending.is_some_and(|(pending, _)| pending == id) {
            self.pending = None;
        } else {
            self.delivered = Some((id, now));
        }
    }

    /// The hotkey whose press went undelivered past the deadline, if any.
    pub(crate) fn missed(&mut self, now: Instant) -> Option<u32> {
        let (id, at) = self.pending?;
        if now.saturating_duration_since(at) < DELIVERY_DEADLINE {
            return None;
        }
        self.pending = None;
        Some(id)
    }

    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(start: Instant, millis: u64) -> Instant {
        start + Duration::from_millis(millis)
    }

    #[test]
    fn a_delivery_after_the_press_is_not_a_miss() {
        let start = Instant::now();
        let mut watch = DeliveryWatch::default();
        watch.pressed(3, start);
        watch.delivered(3, ms(start, 5));
        assert_eq!(watch.missed(ms(start, 2_000)), None);
    }

    #[test]
    fn a_delivery_before_the_press_is_not_a_miss() {
        let start = Instant::now();
        let mut watch = DeliveryWatch::default();
        watch.delivered(3, start);
        watch.pressed(3, ms(start, 5));
        assert_eq!(watch.missed(ms(start, 2_000)), None);
    }

    #[test]
    fn an_undelivered_press_is_reported_once_after_the_deadline() {
        let start = Instant::now();
        let mut watch = DeliveryWatch::default();
        watch.pressed(3, start);
        assert_eq!(watch.missed(ms(start, 100)), None);
        assert_eq!(watch.missed(ms(start, 800)), Some(3));
        assert_eq!(watch.missed(ms(start, 900)), None);
    }

    #[test]
    fn an_old_delivery_does_not_cover_a_later_press() {
        let start = Instant::now();
        let mut watch = DeliveryWatch::default();
        watch.delivered(3, start);
        watch.pressed(3, ms(start, 1_000));
        assert_eq!(watch.missed(ms(start, 2_000)), Some(3));
    }

    #[test]
    fn one_delivery_covers_only_one_press() {
        let start = Instant::now();
        let mut watch = DeliveryWatch::default();
        watch.delivered(3, start);
        watch.pressed(3, ms(start, 5));
        watch.pressed(3, ms(start, 10));
        assert_eq!(watch.missed(ms(start, 1_000)), Some(3));
    }

    #[test]
    fn a_different_hotkey_does_not_cover_the_press() {
        let start = Instant::now();
        let mut watch = DeliveryWatch::default();
        watch.pressed(3, start);
        watch.delivered(4, ms(start, 5));
        assert_eq!(watch.missed(ms(start, 1_000)), Some(3));
    }
}
