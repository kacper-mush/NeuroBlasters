use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;
use tracing::info;

use crate::game::{Game, GameCommand};
use common::protocol::{ClientId, GameCode, GameUpdate, GameState, MapId};

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

            let events = std::mem::take(&mut game.outgoing_events);

            let update = GameUpdate {
                snapshot: game.snapshot(),
                events
            };

            updates.push((game.client_ids(), update));
        }
        updates
    }

    pub fn create_game(&mut self, game_master: ClientId, map_id: MapId, rounds: u8) -> GameCode {
        let code = self.generate_code();
        self.games.insert(code.clone(), Game::new(game_master, map_id, rounds));
        info!("Game created: {:?}", code);  
        code
    }

    pub fn handle_game_command(
        &mut self,
        game_code: &GameCode,
        command: GameCommand,
    ) -> Result<(), String> {
        let game = self.games.get_mut(game_code).ok_or("Game does not exist")?;

        match (command, game.game_state_info()) {
            (GameCommand::Input { client_id, input }, _) => {
                game.handle_player_input(client_id, input);
            }
            (GameCommand::Leave { client_id }, _) => {
                game.remove_player(client_id)?;
            }
            (GameCommand::Join { client_id, nickname }, GameState::Waiting) => {
                game.add_player(client_id, nickname)?;
            }
            (GameCommand::StartCountdown { client_id }, GameState::Waiting) => {
                game.start_countdown(client_id)?;
            }
            _ => return Err("Invalid command in current state".to_string()),
        }

        Ok(())
    }

    fn generate_code(&mut self) -> GameCode {
        loop {
            let code = GameCode(self.rng.random_range(1000..9999).to_string());
            if !self.games.contains_key(&code) {
                break code;
            }
        }
    }
}
