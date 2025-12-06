use crate::countdown::CountdownTimer;
use common::GameEvent;

use super::MIN_PLAYERS_TO_START;

pub struct CountdownState {
    pub timer: CountdownTimer,
}

pub enum CountdownAdvance {
    Continue,
    Finished,
    Cancelled,
}

impl CountdownState {
    pub fn new(seconds: u32) -> Self {
        Self {
            timer: CountdownTimer::new(seconds),
        }
    }

    pub fn advance(
        &mut self,
        delta: std::time::Duration,
        member_count: usize,
        pending_events: &mut Vec<GameEvent>,
    ) -> CountdownAdvance {
        if member_count < MIN_PLAYERS_TO_START {
            return CountdownAdvance::Cancelled;
        }

        let (events, finished) = self.timer.advance(delta);
        pending_events.extend(events);

        if finished {
            CountdownAdvance::Finished
        } else {
            CountdownAdvance::Continue
        }
    }

    pub fn handle_player_left(&mut self, member_count: usize) -> CountdownAdvance {
        if member_count < MIN_PLAYERS_TO_START {
            CountdownAdvance::Cancelled
        } else {
            CountdownAdvance::Continue
        }
    }
}
