use std::time::Duration;

use common::{GameEvent, GameStateSnapshot, InputPayload, Team, game_logic};
use renet::ClientId;

use super::simulation::{GameFrame, GameInstance};

pub enum StartedAdvance {
    Continue,
    Ended {
        winner: Option<Team>,
        snapshot: GameStateSnapshot,
    },
}

pub struct StartedState {
    pub instance: GameInstance,
}

impl StartedState {
    pub fn submit_input(&mut self, client_id: ClientId, payload: InputPayload) {
        self.instance.submit_input(client_id, payload);
    }

    pub fn advance(
        &mut self,
        delta: Duration,
        pending_events: &mut Vec<GameEvent>,
    ) -> StartedAdvance {
        let GameFrame {
            state,
            events,
            winner,
        } = self.instance.advance(delta);
        pending_events.extend(events);

        if let Some(winner) = winner {
            StartedAdvance::Ended {
                winner: Some(winner),
                snapshot: state,
            }
        } else if self.instance.is_empty() {
            StartedAdvance::Ended {
                winner: None,
                snapshot: state,
            }
        } else {
            StartedAdvance::Continue
        }
    }

    pub fn handle_player_left(&mut self, client_id: ClientId) -> StartedAdvance {
        self.instance.remove_client(client_id);
        let snapshot = self.instance.get_state().clone();
        if let Some(winner) = game_logic::check_round_winner(&snapshot.players) {
            StartedAdvance::Ended {
                winner: Some(winner),
                snapshot,
            }
        } else if self.instance.is_empty() {
            StartedAdvance::Ended {
                winner: None,
                snapshot,
            }
        } else {
            StartedAdvance::Continue
        }
    }

    pub fn snapshot(&self) -> GameStateSnapshot {
        self.instance.get_state().clone()
    }
}
