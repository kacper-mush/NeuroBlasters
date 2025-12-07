use std::collections::HashMap;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use tracing::{info, warn};

use common::protocol::{ClientId, GameCode, InputPayload, GameUpdate, GameStateSnapshot, GameEvent};
use crate::game::{Game, GameCommand, GameState};

pub struct GameManager {
    games: HashMap<GameCode, Game>,
    rng: StdRng,
}

impl GameManager {
    pub fn new() -> Self {
        Self {
            games: HashMap::new(),
            rng: StdRng::from_os_rng(),
        }
    }

    /// Advances all games by `dt`.
    /// Returns a list of (Recipients, UpdatePacket) pairs to be broadcasted.
    pub fn tick(&mut self, dt: f32) -> Vec<(Vec<ClientId>, GameUpdate)> {
        let mut updates = Vec::new();

        for game in self.games.values_mut() {
            game.tick(dt);

            let events = std::mem::take(game.outgoing_events);

            let update_packet = match &game.state {
                GameState::Battle { engine } => GameUpdate {
                    state: engine.state.clone(),
                    events
                },
                GameState::Waiting { players } => {
                    let players = players.iter().map(|(client_id, nickname)| nickname).collect();
                    GameUpdate {
                        state: GameStateSnapshot::Waiting { players },
                        events
                    }
                },
                GameState::Ended { .. } => continue, 
            };

            let recipients = Self::get_players_in_game(game);
            
            updates.push((recipients, update_packet));
        }
        updates
    }

    fn get_players_in_game(game: &Game) -> Vec<ClientId> {
        match &game.state {
            GameState::Waiting { players } => players.clone(),
            GameState::Battle { engine } => engine.state.players.iter().map(|p| p.id).collect(),
            GameState::Ended { .. } => vec![], 
        }
    }

    pub fn handle_input(&mut self, game_code: &GameCode, client_id: ClientId, input: InputPayload) {
        if let Some(game) = self.games.get_mut(game_code) {
            game.command_queue.push_back(GameCommand::Input(client_id, input));
        } else {
            // TODO: error handling
            warn!("Input received for non-existent game: {:?}", game_code);
        }
    }

    pub fn create_game(&mut self, client_id: ClientId) -> GameCode {
        let code = self.generate_code();
        let mut game = Game::new(code.clone());
        game.command_queue.push_back(GameCommand::Join(client_id));
        info!("Game created: {:?}", code);
        self.games.insert(code.clone(), game);
        code
    }

    pub fn join_game(&mut self, game_code: &GameCode, client_id: ClientId) -> Result<(), String> {
        let game = self.games.get_mut(game_code).ok_or("Game does not exist")?;
        match game.state {
            GameState::Waiting { .. } => {
                game.command_queue.push_back(GameCommand::Join(client_id));
                Ok(())
            }
            _ => Err("Game has already started".to_string()),
        }
    }

    pub fn leave_game(&mut self, game_code: GameCode, client_id: ClientId) -> Result<(), String> {
        let game = self.games.get_mut(&game_code).ok_or("Game does not exist")?;
        game.command_queue.push_back(GameCommand::Leave(client_id));
        Ok(())
    }

    fn generate_code(&mut self) -> GameCode {
        let code = self.rng.random_range(1000..9999).to_string();
        GameCode(code)
    }
}