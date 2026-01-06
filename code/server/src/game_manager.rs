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

#[cfg(test)]
mod tests {
    use super::*;
    use common::protocol::{CrateGameReponse, GameEvent};

    #[test]
    fn create_game_adds_game_and_returns_ok() {
        let mut gm = GameManager::new_seeded(123);
        let host: ClientId = 1;

        let resp = gm.create_game(host, "host".to_string(), MapName::Basic, 3);
        let (game_code, player_id) = match resp {
            CrateGameReponse::Ok {
                game_code,
                player_id,
            } => (game_code, player_id),
            other => panic!("unexpected response: {other:?}"),
        };

        assert!(gm.games.contains_key(&game_code));
        // Basic sanity: 4 digit numeric code.
        let n: u32 = game_code.0.parse().expect("game_code should be numeric");
        assert!((1000..9999).contains(&n));
        let _ = player_id; // We only care that it was returned successfully.
    }

    #[test]
    fn join_nonexistent_game_is_error() {
        let mut gm = GameManager::new_seeded(0);
        let resp = gm.join_game(&GameCode("9999".to_string()), 1, "p1".to_string());
        assert!(matches!(resp, JoinGameResponse::Error(_)));
    }

    #[test]
    fn leave_game_removes_game_when_last_player_leaves() {
        let mut gm = GameManager::new_seeded(1);
        let host: ClientId = 1;

        let resp = gm.create_game(host, "host".to_string(), MapName::Basic, 3);
        let game_code = match resp {
            CrateGameReponse::Ok { game_code, .. } => game_code,
            other => panic!("unexpected response: {other:?}"),
        };

        let leave = gm.leave_game(&game_code, host);
        assert!(matches!(leave, LeaveGameResponse::Ok));
        assert!(!gm.games.contains_key(&game_code));
    }

    #[test]
    fn start_countdown_rejects_when_game_not_in_lobby_state() {
        let mut gm = GameManager::new_seeded(2);
        let host: ClientId = 1;
        let joiner: ClientId = 2;

        let resp = gm.create_game(host, "host".to_string(), MapName::Basic, 3);
        let game_code = match resp {
            CrateGameReponse::Ok { game_code, .. } => game_code,
            other => panic!("unexpected response: {other:?}"),
        };

        let join = gm.join_game(&game_code, joiner, "joiner".to_string());
        assert!(matches!(join, JoinGameResponse::Ok { .. }));

        let start = gm.start_countdown(&game_code, host);
        assert!(matches!(start, StartCountdownResponse::Ok));

        // Second attempt should be rejected (no longer in lobby state).
        let start_again = gm.start_countdown(&game_code, host);
        assert!(matches!(start_again, StartCountdownResponse::Error(_)));
    }

    #[test]
    fn tick_drains_outgoing_events_into_updates() {
        let mut gm = GameManager::new_seeded(3);
        let host: ClientId = 1;

        let resp = gm.create_game(host, "host".to_string(), MapName::Basic, 3);
        let game_code = match resp {
            CrateGameReponse::Ok { game_code, .. } => game_code,
            other => panic!("unexpected response: {other:?}"),
        };

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
