//! System clock adapter — implements [`application::ports::Clock`] using
//! [`chrono::Utc::now`].
//!
//! Wired into the [`crate::inbound::http::AppState`] by the `viz_api`
//! composition root so use cases (e.g. `GetCandles` defaulting `to` to "now")
//! never read the wall clock directly.

use application::ports::Clock;
use chrono::{DateTime, Utc};

/// Production [`Clock`] implementation reading the system wall clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_clock_returns_monotonic_or_equal_now() {
        let a = SystemClock.now();
        let b = SystemClock.now();
        assert!(b >= a, "system clock must not move backwards: a={a}, b={b}");
    }
}
