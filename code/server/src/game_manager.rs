use std::collections::HashMap;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use tracing::{info};

use common::protocol::{ClientId, GameCode};
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

    pub fn create_game(&mut self, client_id: ClientId) -> GameCode {
        let code = self.generate_code();
        let mut game = Game::new(code.clone());

        // The creator automatically joins
        game.command_queue.push_back(GameCommand::Join(client_id));

        info!("Game created: {:?}", code);
        self.games.insert(code.clone(), game);
        code
    }

    pub fn join_game(&mut self, game_code: &GameCode, client_id: ClientId) -> Result<(), String> {
        let game = self.games.get_mut(game_code).ok_or("Game does not exist")?;

        // Only allow joining if in Waiting state
        match game.state {
            GameState::Waiting { .. } => {
                // We push the command to the game's queue. The game will process it
                // in its next tick() and emit a PlayerJoined event.
                game.command_queue.push_back(GameCommand::Join(client_id));
                Ok(())
            }
            _ => Err("Game has already started".to_string()),
        }
    }

    pub fn leave_game(&mut self, game_code: GameCode, client_id: ClientId) -> Result<(), String> {
        let game = self.games.get_mut(game_code).ok_or("Game does not exist")?;

        // Only allow leaving if in Waiting state
        match game.state {
            GameState::Waiting { .. } => {
                // We push the command to the game's queue. The game will process it
                // in its next tick() and emit a PlayerJoined event.
                game.command_queue.push_back(GameCommand::Leave(client_id));
                Ok(())
            }
            _ => Err("Game has already started".to_string()),
        }
    }

    fn generate_code(&mut self) -> GameCode {
        // Simple 4-digit code for MVP
        let code = self.rng.random_range(1000..9999).to_string();
        GameCode(code)
    }
}
