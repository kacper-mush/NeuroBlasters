use std::time::Duration;

use common::protocol::RoomEvent;

#[derive(Default)]
pub struct CountdownAdvance {
    pub emitted_events: bool,
    pub finished: bool,
}

pub struct CountdownTimer {
    remaining_seconds: f32,
}

impl CountdownTimer {
    pub fn new(seconds: u32) -> Self {
        Self {
            remaining_seconds: seconds as f32,
        }
    }

    pub fn advance(&mut self, delta: Duration) -> (Vec<RoomEvent>, bool) {
        if self.remaining_seconds <= 0.0 {
            panic!("Advancing countdown timer past zero");
        }

        let mut events = Vec::new();
        let previous_whole = self.seconds_left();
        self.remaining_seconds = (self.remaining_seconds - delta.as_secs_f32()).max(0.0);
        let current_whole = self.seconds_left();

        if current_whole < previous_whole {
            events.push(RoomEvent::CountdownTick {
                seconds_left: current_whole,
            });
        }

        if self.remaining_seconds <= 0.0 {
            events.push(RoomEvent::CountdownFinished);
            return (events, true);
        }

        (events, false)
    }

    pub fn seconds_left(&self) -> u32 {
        self.remaining_seconds.ceil() as u32
    }
}
