use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;
use tracing::info;

use crate::game::Game;
use common::protocol::{
    ClientId, CrateGameReponse, GameCode, GameState, GameUpdate, InputPayload, JoinGameResponse,
    LeaveGameResponse, MapName, StartCountdownResponse,
};

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
                events,
            };

            updates.push((game.client_ids(), update));
        }
        updates
    }

    pub fn create_game(
        &mut self,
        game_master: ClientId,
        nickname: String,
        map: MapName,
        rounds: u8,
    ) -> CrateGameReponse {
        let game_code = self.generate_code();

        let mut game = Game::new(game_master, map, rounds);

        let player_id = match game.add_player(game_master, nickname) {
            Ok(id) => id,
            Err(e) => return CrateGameReponse::Error(e),
        };

        self.games.insert(game_code.clone(), game);
        info!("Game created: {:?}", game_code);

        CrateGameReponse::Ok {
            game_code,
            player_id,
        }
    }

    pub fn join_game(
        &mut self,
        game_code: &GameCode,
        client_id: ClientId,
        nickname: String,
    ) -> JoinGameResponse {
        let Some(game) = self.games.get_mut(game_code) else {
            return JoinGameResponse::Error("Game does not exist".to_string());
        };

        if game.game_state_info() != GameState::Waiting {
            return JoinGameResponse::Error("Game is not in lobby state".to_string());
        }

        match game.add_player(client_id, nickname) {
            Ok(player_id) => JoinGameResponse::Ok { player_id },
            Err(e) => JoinGameResponse::Error(e),
        }
    }

    pub fn leave_game(&mut self, game_code: &GameCode, client_id: ClientId) -> LeaveGameResponse {
        let Some(game) = self.games.get_mut(game_code) else {
            return LeaveGameResponse::Error("Game does not exist".to_string());
        };

        let result = match game.remove_player(client_id) {
            Ok(()) => LeaveGameResponse::Ok,
            Err(e) => return LeaveGameResponse::Error(e),
        };

        if game.is_empty() {
            self.games.remove(game_code);
            info!("Game removed (no players left): {:?}", game_code);
        }

        result
    }

    pub fn start_countdown(
        &mut self,
        game_code: &GameCode,
        client_id: ClientId,
    ) -> StartCountdownResponse {
        let Some(game) = self.games.get_mut(game_code) else {
            return StartCountdownResponse::Error("Game does not exist".to_string());
        };

        if game.game_state_info() != GameState::Waiting {
            return StartCountdownResponse::Error("Game is not in lobby state".to_string());
        }

        match game.start_countdown(client_id) {
            Ok(()) => StartCountdownResponse::Ok,
            Err(e) => StartCountdownResponse::Error(e),
        }
    }

    pub fn submit_input(
        &mut self,
        game_code: &GameCode,
        client_id: ClientId,
        input: InputPayload,
    ) -> Result<(), String> {
        let game = self.games.get_mut(game_code).ok_or("Game does not exist")?;
        game.handle_player_input(client_id, input);
        Ok(())
    }

    pub fn remove_player(
        &mut self,
        game_code: &GameCode,
        client_id: ClientId,
    ) -> Result<(), String> {
        let game = self.games.get_mut(game_code).ok_or("Game does not exist")?;
        game.remove_player(client_id)?;

        if game.is_empty() {
            self.games.remove(game_code);
            info!("Game removed (no players left): {:?}", game_code);
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
