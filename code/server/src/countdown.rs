use std::time::{Duration, Instant};

pub struct Countdown {
    pub start_time: Instant,
    pub duration: Duration,
}

impl Countdown {
    pub fn new(duration: Duration) -> Self {
        Self {
            start_time: Instant::now(),
            duration,
        }
    }

    pub fn default() -> Self {
        Self::new(Duration::from_secs(5))
    }

    /// Returns true if the countdown is finished, false otherwise.
    pub fn tick(&mut self) -> bool {
        self.start_time.elapsed() >= self.duration
    }

    pub fn seconds_left(&self) -> u64 {
        self.duration
            .as_secs()
            .saturating_sub(self.start_time.elapsed().as_secs())
    }
}
