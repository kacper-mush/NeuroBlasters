use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

use common::RoomEvent;
use renet::ClientId;

pub struct Room {
    members: HashSet<ClientId>,
    pending_events: Vec<RoomEvent>,
    countdown: Option<CountdownTimer>,
    empty_since: Option<Instant>,
}

impl Room {
    pub fn new() -> Self {
        Self {
            members: HashSet::new(),
            pending_events: Vec::new(),
            countdown: None,
            empty_since: None,
        }
    }

    pub fn add_member(&mut self, client_id: ClientId, nickname: String) -> bool {
        if self.members.insert(client_id) {
            self.pending_events
                .push(RoomEvent::PlayerJoined { nickname });
            self.empty_since = None;
            true
        } else {
            false
        }
    }

    pub fn remove_member(&mut self, client_id: ClientId, nickname: String, now: Instant) -> bool {
        if self.members.remove(&client_id) {
            self.pending_events.push(RoomEvent::PlayerLeft { nickname });
            if self.members.is_empty() {
                self.empty_since = Some(now);
            }
            true
        } else {
            false
        }
    }

    pub fn start_countdown(&mut self, seconds: u32) {
        self.countdown = Some(CountdownTimer::new(seconds));
        self.pending_events
            .push(RoomEvent::CountdownStarted { seconds });
    }

    pub fn advance_countdown(&mut self, delta: Duration) -> bool {
        let mut new_events = false;
        if let Some(timer) = self.countdown.as_mut() {
            let (events, finished) = timer.advance(delta);
            if !events.is_empty() {
                self.pending_events.extend(events);
                new_events = true;
            }
            if finished {
                self.countdown = None;
            }
        }
        new_events
    }

    pub fn has_pending_events(&self) -> bool {
        !self.pending_events.is_empty()
    }

    pub fn drain_events(&mut self) -> Vec<RoomEvent> {
        std::mem::take(&mut self.pending_events)
    }

    pub fn member_ids(&self) -> Vec<ClientId> {
        self.members.iter().copied().collect()
    }

    pub fn should_remove(&mut self, now: Instant, timeout: Duration) -> bool {
        if self.members.is_empty() {
            match self.empty_since {
                Some(since) => now.duration_since(since) >= timeout,
                None => {
                    self.empty_since = Some(now);
                    false
                }
            }
        } else {
            self.empty_since = None;
            false
        }
    }
}

struct CountdownTimer {
    remaining_seconds: f32,
}

impl CountdownTimer {
    fn new(seconds: u32) -> Self {
        Self {
            remaining_seconds: seconds as f32,
        }
    }

    fn advance(&mut self, delta: Duration) -> (Vec<RoomEvent>, bool) {
        if self.remaining_seconds <= 0.0 {
            return (Vec::new(), true);
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

    fn seconds_left(&self) -> u32 {
        self.remaining_seconds.ceil() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_remove_members_track_events() {
        let mut room = Room::new();
        assert!(room.add_member(1, "alpha".into()));
        assert!(room.has_pending_events());
        let events = room.drain_events();
        assert!(matches!(events[0], RoomEvent::PlayerJoined { .. }));

        let now = Instant::now();
        assert!(room.remove_member(1, "alpha".into(), now));
        let events = room.drain_events();
        assert!(matches!(events[0], RoomEvent::PlayerLeft { .. }));
    }

    #[test]
    fn countdown_emits_ticks_and_finishes() {
        let mut room = Room::new();
        room.start_countdown(2);
        assert!(room.has_pending_events());
        let events = room.drain_events();
        assert!(matches!(
            events[0],
            RoomEvent::CountdownStarted { seconds: 2 }
        ));

        assert!(room.advance_countdown(Duration::from_secs(1)));
        let events = room.drain_events();
        assert!(matches!(
            events[0],
            RoomEvent::CountdownTick { seconds_left: 1 }
        ));

        assert!(room.advance_countdown(Duration::from_secs(2)));
        let events = room.drain_events();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, RoomEvent::CountdownFinished))
        );
    }
}
