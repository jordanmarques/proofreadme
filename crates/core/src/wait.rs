//! The bounded waits of the agent on its threads and its locks: none panics, whatever the requested
//! delay, and none is lost to a thread that panicked.

use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use crate::settings::MAX_TIMEOUT;

/// What the wait for a correction grants the provider beyond its timeout, to report its own cause.
pub(crate) const GRACE: Duration = Duration::from_secs(5);

/// The longest delay actually waited: the longest wait for a correction. A larger delay, up to
/// `Duration::MAX`, is brought down to this one.
const MAX_DELAY: Duration = MAX_TIMEOUT.saturating_add(GRACE);

/// The instant at which a delay counted from now has elapsed. `Instant::now() + delay` panics when
/// the sum overflows: the delay is bounded, and failing that the deadline is immediate.
pub(crate) fn deadline(delay: Duration) -> Instant {
    let now = Instant::now();
    now.checked_add(delay.min(MAX_DELAY)).unwrap_or(now)
}

/// The value the mutex protects, even if a thread panicked while holding it.
pub(crate) fn lock<T: ?Sized>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
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
