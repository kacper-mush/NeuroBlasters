use std::collections::HashSet;
use std::time::Instant;

use common::GameEvent;
use renet::ClientId;

use super::state_countdown::CountdownState;

#[derive(Default, Clone, Copy)]
pub struct WaitingState;

impl WaitingState {
    pub fn add_member(
        &mut self,
        members: &mut HashSet<ClientId>,
        pending_events: &mut Vec<GameEvent>,
        empty_since: &mut Option<Instant>,
        client_id: ClientId,
        nickname: String,
    ) -> bool {
        if members.insert(client_id) {
            pending_events.push(GameEvent::PlayerJoined { nickname });
            *empty_since = None;
            true
        } else {
            false
        }
    }

    pub fn start_countdown(
        &mut self,
        pending_events: &mut Vec<GameEvent>,
        seconds: u32,
    ) -> CountdownState {
        pending_events.push(GameEvent::CountdownStarted { seconds });
        CountdownState::new(seconds)
    }
}
