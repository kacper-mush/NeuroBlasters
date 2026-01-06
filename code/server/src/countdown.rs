use std::time::Duration;

pub struct Countdown {
    remaining: Duration,
}

impl Countdown {
    pub fn new(duration: Duration) -> Self {
        Self { remaining: duration }
    }

    pub fn default() -> Self {
        Self::new(Duration::from_secs(5))
    }

    /// Returns true if the countdown is finished, false otherwise.
    pub fn tick(&mut self, dt: Duration) -> bool {
        self.remaining = self.remaining.saturating_sub(dt);
        self.remaining.is_zero()
    }

    pub fn seconds_left(&self) -> u64 {
        // Round up so the UI doesn't show 0 while we still have fractional time left.
        (self.remaining.as_millis().div_ceil(1000)) as u64
    }
}
