use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;
use tracing::info;

use crate::game::{Game, StartCountdownError};
use common::protocol::{
    ClientId, CreateGameResponse, GameCode, GameState, GameUpdate, InitialGameInfo, InputPayload,
    JoinGameResponse, MapName, StartCountdownResponse,
};

pub struct GameManager {
    pub games: HashMap<GameCode, Game>,
    rng: StdRng,
}

const MAX_GAMES: usize = 128;

impl GameManager {
    pub fn new() -> Self {
        Self {
            games: HashMap::new(),
            rng: StdRng::from_os_rng(),
        }
    }

    /// Create a game manager with deterministic randomness (useful for tests).
    #[cfg(test)]
    #[allow(dead_code)]
    pub fn new_seeded(seed: u64) -> Self {
        Self {
            games: HashMap::new(),
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Create a game manager with a provided RNG (useful for tests).
    #[cfg(test)]
    #[allow(dead_code)]
    pub fn new_with_rng(rng: StdRng) -> Self {
        Self {
            games: HashMap::new(),
            rng,
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
    ) -> Result<CreateGameResponse, String> {
        if self.games.len() >= MAX_GAMES {
            return Ok(CreateGameResponse::TooManyGames);
        }

        let game_code = self.generate_code();

        let mut game = Game::new(game_master, map, rounds);

        let player_id = game
            .add_player(game_master, nickname)
            .ok_or("Failed to add player to game")?;

        self.games.insert(game_code.clone(), game);
        info!("Game created: {:?}", game_code);

        Ok(CreateGameResponse::Ok(InitialGameInfo {
            game_code,
            player_id,
            num_rounds: rounds,
            map_name: map,
        }))
    }

    pub fn join_game(
        &mut self,
        game_code: &GameCode,
        client_id: ClientId,
        nickname: String,
    ) -> JoinGameResponse {
        let Some(game) = self.games.get_mut(game_code) else {
            return JoinGameResponse::InvalidCode;
        };

        if game.game_state_info() != GameState::Waiting {
            return JoinGameResponse::GameStarted;
        }

        match game.add_player(client_id, nickname) {
            Some(player_id) => {
                JoinGameResponse::Ok(game.initial_game_info(game_code.clone(), player_id))
            }
            None => JoinGameResponse::GameFull,
        }
    }

    pub fn leave_game(&mut self, game_code: &GameCode, client_id: ClientId) -> Result<(), String> {
        let Some(game) = self.games.get_mut(game_code) else {
            return Err("Game does not exist".to_string());
        };

        game.remove_player(client_id)
            .ok_or("Player not found in game")?;

        if game.is_empty() {
            self.games.remove(game_code);
            info!("Game removed (no players left): {:?}", game_code);
        }

        Ok(())
    }

    pub fn start_countdown(
        &mut self,
        game_code: &GameCode,
        client_id: ClientId,
    ) -> Result<StartCountdownResponse, String> {
        let Some(game) = self.games.get_mut(game_code) else {
            return Err("Game does not exist".to_string());
        };

        match game.start_countdown(client_id) {
            Ok(()) => Ok(StartCountdownResponse::Ok),
            Err(StartCountdownError::NotEnoughPlayers) => {
                Ok(StartCountdownResponse::NotEnoughPlayers)
            }
            Err(StartCountdownError::NotTheGameMaster) => {
                Err("Only the game master can start the countdown".to_string())
            }
            Err(StartCountdownError::NotInWaitingState) => {
                Err("Game is not in waiting state".to_string())
            }
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
        game.remove_player(client_id)
            .ok_or("Player not found in game")?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use common::protocol::{CreateGameResponse, GameEvent};
    use rand::SeedableRng;

    fn unwrap_game_code(resp: Result<CreateGameResponse, String>) -> GameCode {
        match resp.expect("create_game should not unexpectedly fail") {
            CreateGameResponse::Ok(info) => info.game_code,
            _ => unreachable!("create_game should succeed for MapName::Basic"),
        }
    }

    #[test]
    fn create_game_adds_game_and_returns_ok() {
        let mut gm = GameManager::new_seeded(123);
        let host: ClientId = 1;

        let resp = gm.create_game(host, "host".to_string(), MapName::Basic, 3);
        let (game_code, player_id) = match resp {
            Ok(CreateGameResponse::Ok(info)) => (info.game_code, info.player_id),
            _ => unreachable!("create_game should succeed for MapName::Basic"),
        };

        assert!(gm.games.contains_key(&game_code));
        // Basic sanity: 4 digit numeric code.
        let n: u32 = game_code.0.parse().expect("game_code should be numeric");
        assert!((1000..9999).contains(&n));
        let _ = player_id; // We only care that it was returned successfully.
    }

    #[test]
    fn new_with_rng_constructs() {
        let rng = StdRng::seed_from_u64(0);
        let gm = GameManager::new_with_rng(rng);
        assert!(gm.games.is_empty());
    }

    #[test]
    fn join_nonexistent_game_is_error() {
        let mut gm = GameManager::new_seeded(0);
        let resp = gm.join_game(&GameCode("9999".to_string()), 1, "p1".to_string());
        assert!(matches!(resp, JoinGameResponse::InvalidCode));
    }

    #[test]
    fn join_game_errors_when_player_limit_reached() {
        let mut gm = GameManager::new_seeded(0);
        let host: ClientId = 1;

        let game_code =
            unwrap_game_code(gm.create_game(host, "host".to_string(), MapName::Basic, 3));

        // MapName::Basic has 8 spawn points. create_game added 1 player already,
        // so 7 more joins should succeed, and the 9th should fail.
        for i in 0..7 {
            let client_id: ClientId = 10 + i;
            let join = gm.join_game(&game_code, client_id, format!("p{client_id}"));
            assert!(matches!(join, JoinGameResponse::Ok(_)));
        }

        let join = gm.join_game(&game_code, 999, "too_many".to_string());
        assert!(matches!(join, JoinGameResponse::GameFull));
    }

    #[test]
    fn leave_game_removes_game_when_last_player_leaves() {
        let mut gm = GameManager::new_seeded(1);
        let host: ClientId = 1;

        let game_code =
            unwrap_game_code(gm.create_game(host, "host".to_string(), MapName::Basic, 3));

        gm.leave_game(&game_code, host).unwrap();
        assert!(!gm.games.contains_key(&game_code));
    }

    #[test]
    fn leave_nonexistent_game_is_error() {
        let mut gm = GameManager::new_seeded(0);
        let leave = gm.leave_game(&GameCode("9999".to_string()), 1);
        assert!(leave.is_err());
    }

    #[test]
    fn leave_game_errors_when_client_not_in_game() {
        let mut gm = GameManager::new_seeded(0);
        let host: ClientId = 1;

        let game_code =
            unwrap_game_code(gm.create_game(host, "host".to_string(), MapName::Basic, 3));

        let leave = gm.leave_game(&game_code, 999);
        assert!(leave.is_err());
        assert!(gm.games.contains_key(&game_code));
    }

    #[test]
    fn start_countdown_rejects_when_game_not_in_lobby_state() {
        let mut gm = GameManager::new_seeded(2);
        let host: ClientId = 1;
        let joiner: ClientId = 2;

        let game_code =
            unwrap_game_code(gm.create_game(host, "host".to_string(), MapName::Basic, 3));

        let join = gm.join_game(&game_code, joiner, "joiner".to_string());
        assert!(matches!(join, JoinGameResponse::Ok(_)));

        let start = gm.start_countdown(&game_code, host);
        assert!(matches!(start, Ok(StartCountdownResponse::Ok)));

        // Second attempt should be rejected (no longer in lobby state).
        let start_again = gm.start_countdown(&game_code, host);
        assert!(start_again.is_err());
    }

    #[test]
    fn start_countdown_nonexistent_game_is_error() {
        let mut gm = GameManager::new_seeded(0);
        let start = gm.start_countdown(&GameCode("9999".to_string()), 1);
        assert!(start.is_err());
    }

    #[test]
    fn submit_input_nonexistent_game_is_err() {
        let mut gm = GameManager::new_seeded(0);
        let err = gm
            .submit_input(
                &GameCode("9999".to_string()),
                1,
                InputPayload {
                    move_axis: glam::Vec2::ZERO,
                    aim_pos: glam::Vec2::ZERO,
                    shoot: false,
                },
            )
            .unwrap_err();
        assert!(err.contains("Game does not exist"));
    }

    #[test]
    fn remove_player_nonexistent_game_is_err() {
        let mut gm = GameManager::new_seeded(0);
        let err = gm
            .remove_player(&GameCode("9999".to_string()), 1)
            .unwrap_err();
        assert!(err.contains("Game does not exist"));
    }

    #[test]
    fn tick_drains_outgoing_events_into_updates() {
        let mut gm = GameManager::new_seeded(3);
        let host: ClientId = 1;

        let game_code =
            unwrap_game_code(gm.create_game(host, "host".to_string(), MapName::Basic, 3));

        // create_game adds PlayerJoined event.
        assert!(
            gm.games[&game_code]
                .outgoing_events
                .iter()
                .any(|e| matches!(e, GameEvent::PlayerJoined(_)))
        );

        let updates = gm.tick(0.0);
        assert_eq!(updates.len(), 1);
        let (_recipients, update) = &updates[0];
        assert!(
            update
                .events
                .iter()
                .any(|e| matches!(e, GameEvent::PlayerJoined(_)))
        );

        // Should be drained from the game.
        assert!(gm.games[&game_code].outgoing_events.is_empty());
    }
}
