use std::time::Duration;

/// Capped exponential backoff with a stability reset.
///
/// One instance tracks one chain's reconnect cadence. The delay doubles on each
/// consecutive failure up to `max`, and resets to `initial` once a connection
/// has stayed up for `reset_after` — otherwise a process that has been running
/// for weeks would still be waiting the maximum delay after a single blip.
pub struct Backoff {
    initial: Duration,
    max: Duration,
    reset_after: Duration,
    current: Duration,
}

impl Backoff {
    pub const fn new(initial: Duration, max: Duration, reset_after: Duration) -> Self {
        Self {
            initial,
            max,
            reset_after,
            current: initial,
        }
    }

    /// How long to wait before the next attempt, doubling the delay for the
    /// attempt after that.
    pub fn next_delay(&mut self) -> Duration {
        let delay = self.current;
        self.current = self.current.saturating_mul(2).min(self.max);
        delay
    }

    /// Feed back how long the attempt that just ended stayed up. A connection
    /// that survived `reset_after` counts as healthy, so the next failure starts
    /// backing off from `initial` again.
    pub fn record_uptime(&mut self, uptime: Duration) {
        if uptime >= self.reset_after {
            self.current = self.initial;
        }
    }
}

#[cfg(test)]
#[path = "tests/backoff.rs"]
mod tests;
