use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;
use tracing::info;

use crate::game::{Game, GameCommand};
use common::protocol::{ClientId, GameCode, GameUpdate};

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
            let state = game.get_snapshot();
            let events = std::mem::take(&mut game.outgoing_events);

            let update = GameUpdate {
                players,
                state,
                events,
            };

            let recipients = game.players.keys().copied().collect();

            updates.push((recipients, update));
        }
        updates
    }

    pub fn create_game(&mut self) -> GameCode {
        let code = self.generate_code();
        self.games.insert(code.clone(), Game::new());
        info!("Game created: {:?}", code);
        code
    }

    pub fn handle_game_command(
        &mut self,
        game_code: &GameCode,
        command: GameCommand,
    ) -> Result<(), String> {
        let game = self.games.get_mut(game_code).ok_or("Game does not exist")?;
        game.handle_command(command)?;
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
