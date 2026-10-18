//! The bounded waits of the agent on its threads: none panics, whatever the requested delay.

use std::sync::{Condvar, MutexGuard};
use std::time::{Duration, Instant};

/// The longest delay actually waited: a larger delay, up to `Duration::MAX`, is brought down to
/// this one.
const MAX_DELAY: Duration = Duration::from_secs(24 * 60 * 60);

/// The instant at which a delay counted from now has elapsed. `Instant::now() + delay` panics when
/// the sum overflows: the delay is bounded, and failing that the deadline is immediate.
pub(crate) fn deadline(delay: Duration) -> Instant {
    let now = Instant::now();
    now.checked_add(delay.min(MAX_DELAY)).unwrap_or(now)
}

/// Waits, on the condition, until `done` is true of what the lock protects, for at most the given
/// delay. True if it is. A lock poisoned by a thread that panicked remains readable.
pub(crate) fn wait_until<T>(condition: &Condvar, mut guard: MutexGuard<'_, T>, delay: Duration, done: impl Fn(&T) -> bool) -> bool {
    let deadline = deadline(delay);
    while !done(&guard) {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return false;
        }
        guard = condition.wait_timeout(guard, remaining).map(|(guard, _)| guard).unwrap_or_else(|poisoned| poisoned.into_inner().0);
    }
    true
}
