use std::collections::HashMap;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use tracing::{info};

use common::protocol::{ClientId, GameCode, GameUpdate};
use crate::game::{Game, GameCommand, GameState};

pub struct GameManager {
    pub games: HashMap<GameCode, Game>,
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

            let players = game.players.clone().into_iter().collect();
            let state = game.state.get_snapshot();
            let events = std::mem::take(&mut game.outgoing_events);

            let update = GameUpdate {
                players,
                state,
                events
            };

            let recipients = game.players.iter().map(|(client_id, _)| *client_id).collect();
            
            updates.push((recipients, update));
        }
        updates
    }

    pub fn create_game(&mut self) -> GameCode {
        let code = self.generate_code();
        let game = Game::new(code.clone());
        info!("Game created: {:?}", code);
        self.games.insert(code.clone(), game);
        code
    }

    pub fn handle_game_command(&mut self, game_code: &GameCode, command: GameCommand) -> Result<(), String> {
        let game = self.games.get_mut(game_code).ok_or("Game does not exist")?;
        game.handle_command(command)?;
        Ok(())
    }

    pub fn join_game(&mut self, game_code: &GameCode, client_id: ClientId, nickname: String) -> Result<(), String> {
        let game = self.games.get_mut(game_code).ok_or("Game does not exist")?;
        match game.state {
            GameState::Waiting { .. } => {
                game.command_queue.push_back(GameCommand::Join(client_id, nickname));
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