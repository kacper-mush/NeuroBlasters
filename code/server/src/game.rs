use crate::countdown::Countdown;
use common::game::engine::GameEngine;
use common::protocol::{
    ClientId, GameEvent, GameSnapshot, GameState as GameStateInfo, InputPayload, MapDefinition,
    MapName, PlayerId,
};
use std::collections::HashMap;
use std::time::Duration;

pub struct Game {
    state: GameState,
    players: HashMap<ClientId, (PlayerId, String)>, // client -> (player_id, nickname)
    game_master: ClientId,
    engine: GameEngine,
    inputs: HashMap<PlayerId, InputPayload>,
    rounds_left: u8,
    map: MapName,
    pub outgoing_events: Vec<GameEvent>,
}

impl Game {
    pub fn new(game_master: ClientId, map: MapName, rounds_left: u8) -> Self {
        Self {
            state: GameState::Waiting,
            players: HashMap::new(),
            game_master,
            engine: GameEngine::new(MapDefinition::load_name(map)),
            inputs: HashMap::new(),
            rounds_left,
            map,
            outgoing_events: Vec::new(),
        }
    }

    pub fn snapshot(&self) -> GameSnapshot {
        GameSnapshot {
            players: self.engine.players.clone(),
            projectiles: self.engine.projectiles.clone(),
            map: self.map,
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

    pub fn is_empty(&self) -> bool {
        self.players.is_empty()
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
                if countdown.tick(Duration::from_secs_f32(dt)) {
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
                    if self.rounds_left > 0 {
                        self.state = GameState::Countdown(Countdown::default());
                    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use common::protocol::GameEvent;
    use glam::Vec2;

    fn input_shooting_towards(to: Vec2) -> InputPayload {
        InputPayload {
            move_axis: Vec2::ZERO,
            aim_pos: to,
            shoot: true,
        }
    }

    #[test]
    fn add_and_remove_player_emits_events() {
        let master: ClientId = 1;
        let mut g = Game::new(master, MapName::Basic, 3);

        let _p1 = g.add_player(master, "p1".to_string()).unwrap();
        assert!(matches!(
            g.outgoing_events.as_slice(),
            [GameEvent::PlayerJoined(n)] if n == "p1"
        ));

        g.outgoing_events.clear();
        g.remove_player(master).unwrap();
        assert!(matches!(
            g.outgoing_events.as_slice(),
            [GameEvent::PlayerLeft(n)] if n == "p1"
        ));
    }

    #[test]
    fn start_countdown_requires_two_players_and_master() {
        let master: ClientId = 1;
        let other: ClientId = 2;
        let mut g = Game::new(master, MapName::Basic, 3);

        g.add_player(master, "p1".to_string()).unwrap();
        assert!(g.start_countdown(master).is_err(), "needs 2 players");

        g.add_player(other, "p2".to_string()).unwrap();
        assert!(g.start_countdown(other).is_err(), "only master can start");

        g.start_countdown(master).unwrap();
        assert!(matches!(g.game_state_info(), GameStateInfo::Countdown(_)));
    }

    #[test]
    fn countdown_transition_to_battle_after_enough_time() {
        let master: ClientId = 1;
        let other: ClientId = 2;
        let mut g = Game::new(master, MapName::Basic, 3);

        g.add_player(master, "p1".to_string()).unwrap();
        g.add_player(other, "p2".to_string()).unwrap();
        g.start_countdown(master).unwrap();

        // Default countdown is 5s, so 6s should finish it in one tick.
        g.tick(6.0);
        assert!(matches!(g.game_state_info(), GameStateInfo::Battle));
    }

    #[test]
    fn cannot_shoot_during_countdown_but_can_in_battle() {
        let master: ClientId = 1;
        let other: ClientId = 2;
        let mut g = Game::new(master, MapName::Basic, 3);

        g.add_player(master, "p1".to_string()).unwrap();
        g.add_player(other, "p2".to_string()).unwrap();
        g.start_countdown(master).unwrap();

        // Aim at something different than our current position.
        let my_pos = g
            .snapshot()
            .players
            .iter()
            .find(|p| p.nickname == "p1")
            .unwrap()
            .position;

        g.handle_player_input(master, input_shooting_towards(my_pos + Vec2::X * 10.0));
        g.tick(0.0);

        // Countdown suppresses shooting.
        assert_eq!(g.snapshot().projectiles.len(), 0);

        // Transition to battle.
        g.tick(6.0);
        assert!(matches!(g.game_state_info(), GameStateInfo::Battle));

        let my_pos = g
            .snapshot()
            .players
            .iter()
            .find(|p| p.nickname == "p1")
            .unwrap()
            .position;

        g.handle_player_input(master, input_shooting_towards(my_pos + Vec2::X * 10.0));
        g.tick(0.0);

        assert!(g.snapshot().projectiles.len() >= 1);
    }

    #[test]
    fn handle_player_input_ignores_unknown_client() {
        let master: ClientId = 1;
        let mut g = Game::new(master, MapName::Basic, 3);

        // Unknown client should be ignored (no panic, no input recorded).
        g.handle_player_input(
            999,
            InputPayload {
                move_axis: Vec2::ZERO,
                aim_pos: Vec2::ZERO,
                shoot: true,
            },
        );
        assert!(g.inputs.is_empty());
    }

    fn make_player(
        id: PlayerId,
        nickname: &str,
        team: common::protocol::Team,
    ) -> common::protocol::Player {
        common::protocol::Player {
            id,
            nickname: nickname.to_string(),
            team,
            position: Vec2::new(100.0, 100.0),
            velocity: Vec2::ZERO,
            rotation: 0.0,
            radius: 10.0,
            speed: 100.0,
            health: 100.0,
            weapon_cooldown: 0.0,
        }
    }

    #[test]
    fn client_ids_and_is_empty_reflect_players() {
        let master: ClientId = 1;
        let other: ClientId = 2;
        let mut g = Game::new(master, MapName::Basic, 3);

        assert!(g.is_empty());
        assert!(g.client_ids().is_empty());

        g.add_player(master, "p1".to_string()).unwrap();
        g.add_player(other, "p2".to_string()).unwrap();

        let mut ids = g.client_ids();
        ids.sort();
        assert_eq!(ids, vec![master, other]);
        assert!(!g.is_empty());
    }

    #[test]
    fn remove_player_unknown_client_is_error() {
        let master: ClientId = 1;
        let mut g = Game::new(master, MapName::Basic, 3);
        g.add_player(master, "p1".to_string()).unwrap();

        let err = g.remove_player(999).unwrap_err();
        assert!(err.contains("Player not found"));
    }

    #[test]
    fn battle_tick_emits_kill_events() {
        use common::protocol::{KillEvent, Projectile, Team};

        let master: ClientId = 1;
        let mut g = Game::new(master, MapName::Basic, 3);

        // Force battle state and inject players/projectile so resolve_combat produces a kill.
        g.state = GameState::Battle;
        g.engine.players = vec![make_player(0, "killer", Team::Blue), {
            let mut p = make_player(1, "victim", Team::Red);
            p.position = Vec2::new(200.0, 200.0);
            p.health = 1.0;
            p
        }];
        g.engine.projectiles = vec![Projectile {
            id: 1,
            owner_id: 0,
            position: Vec2::new(200.0, 200.0), // hits victim immediately
            velocity: Vec2::ZERO,
            radius: 5.0,
        }];

        g.tick(0.0);

        assert!(g.outgoing_events.iter().any(|e| matches!(
            e,
            GameEvent::Kill(KillEvent {
                killer_id: 0,
                victim_id: 1
            })
        )));
    }

    #[test]
    fn battle_tick_emits_round_end_and_transitions_to_countdown_when_rounds_left_remain() {
        let master: ClientId = 1;
        let mut g = Game::new(master, MapName::Basic, 2);

        // Force battle state and an immediate winner by having only one team alive.
        g.state = GameState::Battle;
        g.engine.players = vec![make_player(0, "p1", common::protocol::Team::Red)];

        g.tick(0.0);

        assert!(
            g.outgoing_events
                .iter()
                .any(|e| matches!(e, GameEvent::RoundEnded(_)))
        );
        assert!(matches!(g.game_state_info(), GameStateInfo::Countdown(_)));
    }

    #[test]
    fn battle_tick_emits_round_end_and_stays_in_battle_when_no_rounds_left() {
        let master: ClientId = 1;
        let mut g = Game::new(master, MapName::Basic, 1);

        g.state = GameState::Battle;
        g.engine.players = vec![make_player(0, "p1", common::protocol::Team::Red)];

        g.tick(0.0);

        assert!(
            g.outgoing_events
                .iter()
                .any(|e| matches!(e, GameEvent::RoundEnded(_)))
        );
        assert!(matches!(g.game_state_info(), GameStateInfo::Battle));
    }
}
