//! `[labs] window_overlay`: holding the shortcuts' base modifier and Shift (`⌥⇧` by default)
//! shows a card over every visible window. Pure and cross-platform: the main thread samples the
//! modifier flags and mouse buttons each tick (no Accessibility), and reports key presses and
//! hotkeys as interruptions.
//!
//! | Since the chord went down, alone | What happens                                         |
//! | -------------------------------- | ---------------------------------------------------- |
//! | 60 ms                            | the worker reads the windows, ahead of the reveal    |
//! | 150 ms                           | the overlay shows, with that reading                 |
//! | 320 ms, after a recent shortcut  | the same, later: a hand on the keys is mid-shortcut  |

use std::time::{Duration, Instant};

use crate::shortcuts::Modifiers;

/// How long the chord must be held, alone, before the overlay shows. A shortcut such as
/// `⌥⇧1` lands well inside it, so it never flashes the overlay.
pub const HOLD_DELAY: Duration = Duration::from_millis(150);
/// The delay while a shortcut is fresh in the hands: someone working from the keyboard is far
/// likelier to be reaching for the next `⌥⇧` shortcut than for the pointer.
pub const HOLD_DELAY_AFTER_SHORTCUT: Duration = Duration::from_millis(320);
/// How long a key press or hotkey counts as fresh.
pub const SHORTCUT_MEMORY: Duration = Duration::from_secs(2);
/// When to read the windows ahead of the reveal, so the overlay shows the moment the hold
/// completes instead of a worker round trip later.
pub const WARM_DELAY: Duration = Duration::from_millis(60);

/// What a sample or an interruption changed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HoldChange {
    /// The hold looks deliberate: read what the overlay will show.
    Warm,
    Show,
    Hide,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Phase {
    /// The chord isn't held.
    #[default]
    Idle,
    /// The chord is held alone, since this moment.
    Armed {
        since: Instant,
        warmed: bool,
    },
    Shown,
    /// This hold went to something else (a key, another modifier, a mouse button): the overlay
    /// stays away until the chord is let go.
    Spent,
}

/// Whether the chord has been held long enough, alone, to show the overlay.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OverlayHold {
    phase: Phase,
    /// The last key press or hotkey.
    shortcut: Option<Instant>,
}

impl OverlayHold {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            phase: Phase::Idle,
            shortcut: None,
        }
    }

    /// Whether the overlay is up.
    #[must_use]
    pub const fn is_shown(self) -> bool {
        matches!(self.phase, Phase::Shown)
    }

    /// Whether the chord is held alone and the overlay has yet to show.
    #[must_use]
    pub const fn is_armed(self) -> bool {
        matches!(self.phase, Phase::Armed { .. })
    }

    /// Whether a sample can change anything: the chord is held, or something is.
    #[must_use]
    pub const fn is_idle(self) -> bool {
        matches!(self.phase, Phase::Idle)
    }

    /// How long a hold that began at `since` waits: longer just after a shortcut.
    fn delay(self, since: Instant) -> Duration {
        match self.shortcut {
            Some(shortcut) if since.saturating_duration_since(shortcut) < SHORTCUT_MEMORY => {
                HOLD_DELAY_AFTER_SHORTCUT
            }
            _ => HOLD_DELAY,
        }
    }

    /// One tick's reading: whether the overlay may show now (`enabled`: tiling and the Lab are
    /// on), the chord, the modifiers held, and whether a mouse button is down.
    pub fn sample(
        &mut self,
        now: Instant,
        enabled: bool,
        chord: Modifiers,
        held: Modifiers,
        mouse_down: bool,
    ) -> Option<HoldChange> {
        let was = self.phase;
        self.phase = match was {
            _ if !enabled || !contains(held, chord) => Phase::Idle,
            Phase::Idle if held == chord && !mouse_down => Phase::Armed {
                since: now,
                warmed: false,
            },
            Phase::Idle => Phase::Spent,
            Phase::Armed { .. } if held != chord || mouse_down => Phase::Spent,
            Phase::Armed { since, .. }
                if now.saturating_duration_since(since) >= self.delay(since) =>
            {
                Phase::Shown
            }
            Phase::Armed { since, .. } => Phase::Armed {
                since,
                warmed: now.saturating_duration_since(since) >= WARM_DELAY,
            },
            // A click on a card holds a button: the overlay stays up.
            phase @ (Phase::Shown | Phase::Spent) => phase,
        };
        match (was, self.phase) {
            (Phase::Shown, Phase::Shown) => None,
            (_, Phase::Shown) => Some(HoldChange::Show),
            (Phase::Shown, _) => Some(HoldChange::Hide),
            (Phase::Armed { warmed: false, .. }, Phase::Armed { warmed: true, .. }) => {
                Some(HoldChange::Warm)
            }
            _ => None,
        }
    }

    /// A key was pressed or a hotkey fired: this hold was a shortcut, so the overlay hides, or
    /// never shows, until the chord is pressed again, and the next hold waits a little longer.
    pub fn interrupt(&mut self, now: Instant) -> Option<HoldChange> {
        let was_shown = self.is_shown();
        self.phase = Phase::Spent;
        self.shortcut = Some(now);
        was_shown.then_some(HoldChange::Hide)
    }
}

/// Whether every modifier in `chord` is held.
const fn contains(held: Modifiers, chord: Modifiers) -> bool {
    (!chord.command || held.command)
        && (!chord.control || held.control)
        && (!chord.option || held.option)
        && (!chord.shift || held.shift)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPTION_SHIFT: Modifiers = Modifiers::OPTION.with_shift();
    const NONE: Modifiers = Modifiers {
        command: false,
        control: false,
        option: false,
        shift: false,
    };
    const DELAY: u64 = HOLD_DELAY.as_millis() as u64;
    const WARM: u64 = WARM_DELAY.as_millis() as u64;
    const SLOW: u64 = HOLD_DELAY_AFTER_SHORTCUT.as_millis() as u64;

    /// Samples `held` at `millis` after `start`, with the Lab on and no button down.
    fn at(
        hold: &mut OverlayHold,
        start: Instant,
        millis: u64,
        held: Modifiers,
    ) -> Option<HoldChange> {
        hold.sample(
            start + Duration::from_millis(millis),
            true,
            OPTION_SHIFT,
            held,
            false,
        )
    }

    #[test]
    fn holding_the_chord_alone_warms_then_shows_and_letting_go_hides() {
        let start = Instant::now();
        let mut hold = OverlayHold::default();
        assert_eq!(at(&mut hold, start, 0, OPTION_SHIFT), None);
        assert_eq!(at(&mut hold, start, WARM - 1, OPTION_SHIFT), None);
        assert_eq!(
            at(&mut hold, start, WARM, OPTION_SHIFT),
            Some(HoldChange::Warm)
        );
        assert!(hold.is_armed());
        assert_eq!(at(&mut hold, start, DELAY - 1, OPTION_SHIFT), None, "once");
        assert_eq!(
            at(&mut hold, start, DELAY, OPTION_SHIFT),
            Some(HoldChange::Show)
        );
        assert!(!hold.is_armed());
        // Clicking a card holds a button: still up.
        let later = start + Duration::from_millis(300);
        assert_eq!(
            hold.sample(later, true, OPTION_SHIFT, OPTION_SHIFT, true),
            None
        );
        assert!(hold.is_shown());
        // Lifting Shift alone is letting go.
        assert_eq!(
            at(&mut hold, start, 400, Modifiers::OPTION),
            Some(HoldChange::Hide)
        );
        assert!(hold.is_idle());
        // Pressed again, it waits the whole delay again.
        assert_eq!(at(&mut hold, start, 500, OPTION_SHIFT), None);
        assert_eq!(
            at(&mut hold, start, 500 + WARM, OPTION_SHIFT),
            Some(HoldChange::Warm)
        );
        assert_eq!(at(&mut hold, start, 500 + DELAY - 1, OPTION_SHIFT), None);
        assert_eq!(
            at(&mut hold, start, 500 + DELAY, OPTION_SHIFT),
            Some(HoldChange::Show)
        );
    }

    #[test]
    fn a_sample_that_skips_past_the_delay_shows_without_warming() {
        // A stalled main thread: the first sample after the press is already past the delay.
        let start = Instant::now();
        let mut hold = OverlayHold::default();
        at(&mut hold, start, 0, OPTION_SHIFT);
        assert_eq!(
            at(&mut hold, start, 400, OPTION_SHIFT),
            Some(HoldChange::Show)
        );
    }

    #[test]
    fn a_shortcut_inside_the_delay_never_shows_the_overlay() {
        let start = Instant::now();
        let mut hold = OverlayHold::default();
        at(&mut hold, start, 0, OPTION_SHIFT);
        // ⌥⇧1 fires.
        assert_eq!(hold.interrupt(start + Duration::from_millis(90)), None);
        for millis in [200, 400, 1000] {
            assert_eq!(
                at(&mut hold, start, millis, OPTION_SHIFT),
                None,
                "spent until let go"
            );
        }
        at(&mut hold, start, 1100, NONE);
        at(&mut hold, start, 1200, OPTION_SHIFT);
        assert_eq!(
            at(&mut hold, start, 1200 + SLOW, OPTION_SHIFT),
            Some(HoldChange::Show)
        );
    }

    #[test]
    fn a_hold_just_after_a_shortcut_waits_longer_and_a_later_one_does_not() {
        let start = Instant::now();
        let mut hold = OverlayHold::default();
        at(&mut hold, start, 0, OPTION_SHIFT);
        hold.interrupt(start + Duration::from_millis(80));
        at(&mut hold, start, 200, NONE);
        // The next chord, 220 ms after the shortcut: the usual delay passes with nothing shown.
        at(&mut hold, start, 300, OPTION_SHIFT);
        assert_eq!(
            at(&mut hold, start, 300 + WARM, OPTION_SHIFT),
            Some(HoldChange::Warm)
        );
        assert_eq!(at(&mut hold, start, 300 + DELAY, OPTION_SHIFT), None);
        assert_eq!(at(&mut hold, start, 300 + SLOW - 1, OPTION_SHIFT), None);
        assert_eq!(
            at(&mut hold, start, 300 + SLOW, OPTION_SHIFT),
            Some(HoldChange::Show)
        );
        // Once the shortcut is two seconds old, a hold is quick again.
        at(&mut hold, start, 1000, NONE);
        let fresh = 80 + SHORTCUT_MEMORY.as_millis() as u64;
        at(&mut hold, start, fresh, OPTION_SHIFT);
        assert_eq!(
            at(&mut hold, start, fresh + DELAY, OPTION_SHIFT),
            Some(HoldChange::Show)
        );
    }

    #[test]
    fn a_shortcut_reported_before_the_chord_is_sampled_still_spends_it() {
        // ⌥⇧1 pressed and its hotkey delivered within one tick, before the first sample.
        let start = Instant::now();
        let mut hold = OverlayHold::default();
        assert_eq!(hold.interrupt(start), None);
        at(&mut hold, start, 0, OPTION_SHIFT);
        assert_eq!(at(&mut hold, start, 500, OPTION_SHIFT), None);
    }

    #[test]
    fn a_key_while_shown_hides_until_let_go() {
        let start = Instant::now();
        let mut hold = OverlayHold::default();
        at(&mut hold, start, 0, OPTION_SHIFT);
        at(&mut hold, start, 200, OPTION_SHIFT);
        assert_eq!(
            hold.interrupt(start + Duration::from_millis(300)),
            Some(HoldChange::Hide)
        );
        assert!(!hold.is_shown());
        assert_eq!(at(&mut hold, start, 600, OPTION_SHIFT), None);
    }

    #[test]
    fn other_modifiers_or_a_held_button_spend_the_hold() {
        let start = Instant::now();
        let command = Modifiers {
            command: true,
            ..OPTION_SHIFT
        };
        let control = Modifiers {
            control: true,
            ..OPTION_SHIFT
        };
        for extra in [command, control] {
            let mut hold = OverlayHold::default();
            at(&mut hold, start, 0, extra);
            assert_eq!(
                at(&mut hold, start, 300, extra),
                None,
                "{extra:?} from the start"
            );
            let mut hold = OverlayHold::default();
            at(&mut hold, start, 0, OPTION_SHIFT);
            at(&mut hold, start, 50, extra);
            assert_eq!(
                at(&mut hold, start, 300, OPTION_SHIFT),
                None,
                "{extra:?} added inside the delay"
            );
        }
        // A drag started with the chord held is not a hold.
        let mut hold = OverlayHold::default();
        hold.sample(start, true, OPTION_SHIFT, OPTION_SHIFT, true);
        assert_eq!(at(&mut hold, start, 300, OPTION_SHIFT), None);
        // Option alone is not the chord.
        let mut hold = OverlayHold::default();
        at(&mut hold, start, 0, Modifiers::OPTION);
        assert_eq!(at(&mut hold, start, 300, Modifiers::OPTION), None);
    }

    #[test]
    fn the_lab_off_or_tiling_paused_never_shows_and_hides_a_shown_overlay() {
        let start = Instant::now();
        let mut hold = OverlayHold::default();
        let later = |millis| start + Duration::from_millis(millis);
        hold.sample(start, false, OPTION_SHIFT, OPTION_SHIFT, false);
        assert_eq!(
            hold.sample(later(500), false, OPTION_SHIFT, OPTION_SHIFT, false),
            None
        );
        at(&mut hold, start, 600, OPTION_SHIFT);
        at(&mut hold, start, 800, OPTION_SHIFT);
        assert!(hold.is_shown());
        assert_eq!(
            hold.sample(later(900), false, OPTION_SHIFT, OPTION_SHIFT, false),
            Some(HoldChange::Hide)
        );
    }

    #[test]
    fn any_base_modifier_with_shift_is_a_chord() {
        let chord = Modifiers {
            control: true,
            ..NONE
        }
        .with_shift();
        let start = Instant::now();
        let mut hold = OverlayHold::default();
        hold.sample(start, true, chord, chord, false);
        let later = start + HOLD_DELAY;
        assert_eq!(
            hold.sample(later, true, chord, chord, false),
            Some(HoldChange::Show)
        );
    }
}
