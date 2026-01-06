use crate::countdown::Countdown;
use common::game::engine::GameEngine;
use common::protocol::{
    ClientId, GameEvent, GameSnapshot, GameState as GameStateInfo, InputPayload, MapDefinition,
    MapId, PlayerId,
};
use std::collections::HashMap;

pub struct Game {
    state: GameState,
    players: HashMap<ClientId, (PlayerId, String)>, // client -> (player_id, nickname)
    game_master: ClientId,
    engine: GameEngine,
    inputs: HashMap<PlayerId, InputPayload>,
    rounds_left: u8,
    map_id: MapId,
    pub outgoing_events: Vec<GameEvent>,
}

impl Game {
    pub fn new(game_master: ClientId, map_id: MapId, rounds_left: u8) -> Self {
        Self {
            state: GameState::Waiting,
            players: HashMap::new(),
            game_master,
            engine: GameEngine::new(MapDefinition::load(map_id)),
            inputs: HashMap::new(),
            rounds_left,
            map_id,
            outgoing_events: Vec::new(),
        }
    }

    pub fn snapshot(&self) -> GameSnapshot {
        GameSnapshot {
            players: self.engine.players.clone(),
            projectiles: self.engine.projectiles.clone(),
            map_id: self.map_id,
            state: self.game_state_info(),
        }
    }

    pub fn game_state_info(&self) -> GameStateInfo {
        match &self.state {
            GameState::Waiting => GameStateInfo::Waiting,
            GameState::Countdown(countdown) => GameStateInfo::Countdown(countdown.seconds_left()),
            GameState::Battle => GameStateInfo::Battle,
        }
    }

    pub fn client_ids(&self) -> Vec<ClientId> {
        self.players.keys().copied().collect()
    }

    pub fn add_player(
        &mut self,
        client_id: ClientId,
        nickname: String,
    ) -> Result<PlayerId, String> {
        let player_id = self.engine.add_player(nickname.clone())?;
        self.players
            .insert(client_id, (player_id, nickname.clone()));
        self.outgoing_events.push(GameEvent::PlayerJoined(nickname));
        Ok(player_id)
    }

    pub fn remove_player(&mut self, client_id: ClientId) -> Result<(), String> {
        let (player_id, nickname) = self
            .players
            .remove(&client_id)
            .ok_or("Player not found".to_string())?;
        self.engine.remove_player(player_id);
        self.outgoing_events.push(GameEvent::PlayerLeft(nickname));
        Ok(())
    }

    pub fn start_countdown(&mut self, client_id: ClientId) -> Result<(), String> {
        if self.players.len() < 2 {
            return Err("At least 2 players needed to start the game".to_string());
        }

        if client_id != self.game_master {
            return Err("Only the game master can start the countdown".to_string());
        }

        self.state = GameState::Countdown(Countdown::default());
        Ok(())
    }

    pub fn handle_player_input(&mut self, client_id: ClientId, input: InputPayload) {
        let Some((player_id, _)) = self.players.get(&client_id) else {
            return;
        };
        let input = if let GameState::Battle = self.state {
            input
        } else {
            // Players can't shoot if not in battle.
            InputPayload {
                shoot: false,
                ..input
            }
        };
        self.inputs.insert(*player_id, input);
    }

    pub fn tick(&mut self, dt: f32) {
        let result = self.engine.tick(dt, self.inputs.clone());
        self.inputs.clear();

        match &mut self.state {
            GameState::Countdown(countdown) => {
                if countdown.tick() {
                    self.state = GameState::Battle;
                    let active_player_ids: Vec<PlayerId> = self
                        .players
                        .values()
                        .map(|(player_id, _)| *player_id)
                        .collect();
                    self.engine.move_players_to_spawnpoints(&active_player_ids);
                }
            }
            GameState::Battle => {
                let mut kill_events = result
                    .kills
                    .iter()
                    .map(|kill_event| GameEvent::Kill(kill_event.clone()))
                    .collect();

                self.outgoing_events.append(&mut kill_events);

                if let Some(winner) = result.winner {
                    self.outgoing_events.push(GameEvent::RoundEnded(winner));
                    self.rounds_left -= 1;
                    if self.rounds_left == 0 {
                        todo!()
                    }
                    self.state = GameState::Countdown(Countdown::default());
                }
            }
            GameState::Waiting => {}
        }
    }
}

enum GameState {
    Waiting,
    Countdown(Countdown),
    Battle,
}
