use std::time::Duration;

#[derive(Debug)]
pub struct Countdown {
    remaining: Duration,
}

impl Countdown {
    pub fn new(duration: Duration) -> Self {
        Self {
            remaining: duration,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seconds_left_rounds_up() {
        let c = Countdown::new(Duration::from_millis(1));
        assert_eq!(c.seconds_left(), 1);

        let c = Countdown::new(Duration::from_millis(999));
        assert_eq!(c.seconds_left(), 1);

        let c = Countdown::new(Duration::from_millis(1000));
        assert_eq!(c.seconds_left(), 1);

        let c = Countdown::new(Duration::from_millis(1001));
        assert_eq!(c.seconds_left(), 2);

        let c = Countdown::new(Duration::from_millis(1999));
        assert_eq!(c.seconds_left(), 2);
    }

    #[test]
    fn tick_counts_down_and_finishes() {
        let mut c = Countdown::new(Duration::from_secs(2));
        assert_eq!(c.seconds_left(), 2);

        assert!(!c.tick(Duration::from_millis(1)));
        assert_eq!(c.seconds_left(), 2);

        assert!(!c.tick(Duration::from_secs(1)));
        assert_eq!(c.seconds_left(), 1);

        // Overshoot should saturate to zero and finish.
        assert!(c.tick(Duration::from_secs(10)));
        assert_eq!(c.seconds_left(), 0);
    }
}
