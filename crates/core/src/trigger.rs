//! The trigger detector (ADR 0003): press, release, press, release of the trigger modifier, the
//! two presses within 300 ms, no other key down in between. Fires on the second release, so that
//! the modifier is no longer held when the copy is synthesized.

use std::time::{Duration, Instant};

use crate::os::{Direction, Key, KeyEvent, Modifier};

/// The longest delay between the two presses of a trigger.
pub const DOUBLE_PRESS_WINDOW: Duration = Duration::from_millis(300);

#[derive(Debug, Clone, Copy)]
enum Phase {
    /// Nothing in progress.
    Rest,
    /// The modifier is down for the first time since this instant.
    FirstDown(Instant),
    /// The modifier went down then up; the first press was at this instant.
    FirstUp(Instant),
    /// The modifier is down for the second time, in time.
    SecondDown,
}

/// Follows the key events and says when they make a trigger.
#[derive(Debug)]
pub(crate) struct Detector {
    phase: Phase,
}

impl Detector {
    pub(crate) fn new() -> Self {
        Self { phase: Phase::Rest }
    }

    /// Takes the next key event; true when it completes a trigger of the given modifier.
    pub(crate) fn feed(&mut self, event: &KeyEvent, trigger: Modifier) -> bool {
        if event.injected {
            return false;
        }
        if event.key != Key::Modifier(trigger) {
            if event.direction == Direction::Down {
                self.phase = Phase::Rest;
            }
            return false;
        }
        let (phase, fires) = match (self.phase, event.direction) {
            (Phase::Rest, Direction::Down) => (Phase::FirstDown(event.at), false),
            // A held key repeats its press: still the same press.
            (Phase::FirstDown(first), Direction::Down) => (Phase::FirstDown(first), false),
            (Phase::FirstDown(first), Direction::Up) => (Phase::FirstUp(first), false),
            (Phase::FirstUp(first), Direction::Down) => {
                if event.at.saturating_duration_since(first) <= DOUBLE_PRESS_WINDOW {
                    (Phase::SecondDown, false)
                } else {
                    (Phase::FirstDown(event.at), false)
                }
            }
            (Phase::SecondDown, Direction::Down) => (Phase::SecondDown, false),
            (Phase::SecondDown, Direction::Up) => (Phase::Rest, true),
            (Phase::Rest | Phase::FirstUp(_), Direction::Up) => (Phase::Rest, false),
        };
        self.phase = phase;
        fires
    }
}
