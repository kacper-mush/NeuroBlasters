use crate::countdown::Countdown;
use common::game::engine::GameEngine;
use common::protocol::{
    ClientId, GameCode, GameEvent, GameSnapshot, GameState as GameStateInfo, InitialGameInfo,
    InputPayload, MapDefinition, MapName, PlayerId, Team,
};
use rand::Rng;
use std::collections::HashMap;
use std::time::Duration;
use tracing::warn;

const ROUND_DURATION: Duration = Duration::from_secs(100);

pub struct Game {
    state: GameState,
    players: HashMap<ClientId, (PlayerId, String)>, // client -> (player_id, nickname)
    game_master: ClientId,
    engine: GameEngine,
    inputs: HashMap<PlayerId, InputPayload>,
    curr_round: u8,
    total_rounds: u8,
    map: MapName,
    pub outgoing_events: Vec<GameEvent>,
}

impl Game {
    pub fn new(game_master: ClientId, map: MapName, rounds: u8) -> Self {
        Self {
            state: GameState::Waiting,
            players: HashMap::new(),
            game_master,
            engine: GameEngine::new(MapDefinition::load_name(map)),
            inputs: HashMap::new(),
            curr_round: 1,
            total_rounds: rounds,
            map,
            outgoing_events: Vec::new(),
        }
    }

    pub fn snapshot(&self) -> GameSnapshot {
        GameSnapshot {
            engine: self.engine.snapshot(),
            state: self.game_state_info(),
            game_master: self.game_master,
            round_number: self.curr_round,
        }
    }

    pub fn initial_game_info(&self, game_code: GameCode, player_id: PlayerId) -> InitialGameInfo {
        InitialGameInfo {
            game_code,
            player_id,
            num_rounds: self.total_rounds,
            map_name: self.map,
            game_master: self.game_master,
        }
    }

    pub fn game_state_info(&self) -> GameStateInfo {
        match &self.state {
            GameState::Waiting => GameStateInfo::Waiting,
            GameState::Countdown(countdown) => GameStateInfo::Countdown(countdown.seconds_left()),
            GameState::Battle(countdown) => GameStateInfo::Battle(countdown.seconds_left()),
            GameState::Results(winner) => GameStateInfo::Results(*winner),
        }
    }

    pub fn client_ids(&self) -> Vec<ClientId> {
        self.players.keys().copied().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.players.is_empty()
    }

    pub fn add_player(&mut self, client_id: ClientId, nickname: String) -> Option<PlayerId> {
        let player_id = self.engine.add_player(nickname.clone()).ok()?;
        self.players
            .insert(client_id, (player_id, nickname.clone()));
        self.outgoing_events.push(GameEvent::PlayerJoined(nickname));
        Some(player_id)
    }

    pub fn remove_player(&mut self, client_id: ClientId) -> Option<PlayerId> {
        let (player_id, nickname) = self.players.remove(&client_id)?;
        self.engine.remove_player(player_id);
        self.outgoing_events.push(GameEvent::PlayerLeft(nickname));
        Some(player_id)
    }

    pub fn start_countdown(&mut self, client_id: ClientId) -> Result<(), StartCountdownError> {
        if !matches!(self.state, GameState::Waiting) {
            return Err(StartCountdownError::NotInWaitingState);
        }

        if client_id != self.game_master {
            return Err(StartCountdownError::NotTheGameMaster);
        }

        self.state = GameState::Countdown(Countdown::default());
        Ok(())
    }

    pub fn handle_player_input(&mut self, client_id: ClientId, input: InputPayload) {
        let Some((player_id, _)) = self.players.get(&client_id) else {
            warn!(%client_id, "Player not found, ignoring input");
            return;
        };
        let input = match self.state {
            GameState::Battle(_) => input,
            GameState::Waiting | GameState::Countdown(_) | GameState::Results(_) => InputPayload {
                shoot: false,
                ..input
            },
        };
        self.inputs.insert(*player_id, input);
    }

    pub fn tick(&mut self, dt: f32) {
        let result = self.engine.tick(dt, self.inputs.clone());
        self.inputs.clear();

        match &mut self.state {
            GameState::Countdown(countdown) => {
                if countdown.tick(Duration::from_secs_f32(dt)) {
                    self.state = GameState::Battle(Countdown::new(ROUND_DURATION));
                    self.engine.prepare_new_round();
                }
            }
            GameState::Battle(countdown) => {
                let mut round_ended = false;
                let mut winner = None;

                if countdown.tick(Duration::from_secs_f32(dt)) {
                    winner = Some(self.resolve_winner_by_hp());
                    round_ended = true;
                }

                if !round_ended {
                    let mut kill_events = result
                        .kills
                        .iter()
                        .map(|kill_event| GameEvent::Kill(kill_event.clone()))
                        .collect();

                    self.outgoing_events.append(&mut kill_events);

                    if let Some(w) = result.winner {
                        winner = Some(w);
                    }
                }

                if let Some(winner) = winner {
                    self.outgoing_events.push(GameEvent::RoundEnded(winner));
                    self.curr_round += 1;
                    if self.curr_round <= self.total_rounds {
                        self.state = GameState::Countdown(Countdown::default());
                    } else {
                        self.state = GameState::Results(winner);
                        self.engine.clear_projectiles();
                    }
                }
            }
            GameState::Waiting => {}
            GameState::Results(_winner) => {}
        }
    }

    fn resolve_winner_by_hp(&self) -> Team {
        let mut red_hp = 0.0;
        let mut blue_hp = 0.0;
        for t in self.engine.tanks() {
            match t.player_info.team {
                Team::Red => red_hp += t.health,
                Team::Blue => blue_hp += t.health,
            }
        }

        if red_hp > blue_hp {
            Team::Red
        } else if blue_hp > red_hp {
            Team::Blue
        } else if rand::rng().random_bool(0.5) {
            Team::Red
        } else {
            Team::Blue
        }
    }
}

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum StartCountdownError {
    NotTheGameMaster,
    NotInWaitingState,
}

enum GameState {
    Waiting,
    Countdown(Countdown),
    Battle(Countdown),
    Results(Team),
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::{
        game::{Tank, player::PlayerInfo},
        protocol::{EngineSnapshot, GameEvent},
    };
    use glam::Vec2;

    fn input_shooting_towards(to: Vec2) -> InputPayload {
        InputPayload {
            move_axis: Vec2::ZERO,
            aim_pos: to,
            shoot: true,
        }
    }

    fn make_player(id: u16, nickname: &str, team: Team) -> Tank {
        Tank::new(PlayerInfo::new(id, nickname.to_string(), team), Vec2::ZERO)
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
    fn start_countdown_requires_master() {
        let master: ClientId = 1;
        let other: ClientId = 2;
        let mut g = Game::new(master, MapName::Basic, 3);

        g.add_player(master, "p1".to_string()).unwrap();
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

        g.tick(6.0);
        assert!(matches!(g.game_state_info(), GameStateInfo::Battle(_)));
    }

    #[test]
    fn battle_timeout_declares_winner_by_health() {
        let master: ClientId = 1;
        let other: ClientId = 2;
        let mut g = Game::new(master, MapName::Basic, 1);
        g.add_player(master, "p1".to_string()).unwrap();
        g.add_player(other, "p2".to_string()).unwrap(); // Auto Blue

        g.state = GameState::Battle(Countdown::new(Duration::from_secs(1)));
        g.engine.apply_snapshot(EngineSnapshot {
            tanks: vec![
                make_player(0, "p1", common::protocol::Team::Red), // 100 HP
                {
                    let mut p = make_player(1, "p2", common::protocol::Team::Blue);
                    p.health = 50.0;
                    p
                },
            ],
            projectiles: Vec::new(),
        });

        // Tick 1.0s to finish countdown
        g.tick(1.0);

        // Should see RoundEnded with Red winner
        assert!(
            g.outgoing_events
                .iter()
                .any(|e| matches!(e, GameEvent::RoundEnded(common::protocol::Team::Red)))
        );
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
            .engine
            .tanks
            .iter()
            .find(|p| p.player_info.nickname == "p1")
            .unwrap()
            .position;

        g.handle_player_input(master, input_shooting_towards(my_pos + Vec2::X * 10.0));
        g.tick(0.0);

        // Countdown suppresses shooting.
        assert_eq!(g.snapshot().engine.projectiles.len(), 0);

        // Transition to battle.
        g.tick(6.0);
        assert!(matches!(g.game_state_info(), GameStateInfo::Battle(_)));

        let my_pos = g
            .snapshot()
            .engine
            .tanks
            .iter()
            .find(|p| p.player_info.nickname == "p1")
            .unwrap()
            .position;

        g.handle_player_input(master, input_shooting_towards(my_pos + Vec2::X * 10.0));
        g.tick(0.0);

        assert!(!g.snapshot().engine.projectiles.is_empty());
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

        assert!(g.remove_player(999).is_none());
    }

    #[test]
    fn battle_tick_emits_kill_events() {
        use common::protocol::{KillEvent, Projectile, Team};

        let master: ClientId = 1;
        let mut g = Game::new(master, MapName::Basic, 3);

        let infos = [
            PlayerInfo::new(0, "killer".into(), Team::Blue),
            PlayerInfo::new(1, "victim".into(), Team::Red),
        ];

        // Force battle state and inject players/projectile so resolve_combat produces a kill.
        g.state = GameState::Battle(Countdown::new(ROUND_DURATION));
        g.engine.apply_snapshot(EngineSnapshot {
            tanks: vec![Tank::new(infos[0].clone(), Vec2::ZERO), {
                let mut p = Tank::new(infos[1].clone(), Vec2::new(200.0, 200.0));
                p.health = 1.0;
                p
            }],
            projectiles: vec![Projectile {
                id: 1,
                owner_info: infos[0].clone(),
                position: Vec2::new(200.0, 200.0), // hits victim immediately
                velocity: Vec2::ZERO,
                radius: 5.0,
            }],
        });

        g.tick(0.0);

        assert!(g.outgoing_events.iter().any(|e| {
            if let GameEvent::Kill(KillEvent {
                killer_info,
                victim_info,
            }) = e
            {
                killer_info == &infos[0] && victim_info == &infos[1]
            } else {
                false
            }
        }));
    }

    #[test]
    fn battle_tick_emits_round_end_and_transitions_to_countdown_when_rounds_left_remain() {
        let master: ClientId = 1;
        let mut g = Game::new(master, MapName::Basic, 2);

        // Force battle state and an immediate winner by having only one team alive.
        g.state = GameState::Battle(Countdown::new(ROUND_DURATION));
        g.engine.apply_snapshot(EngineSnapshot {
            tanks: vec![Tank::new(
                PlayerInfo::new(0, "p1".into(), common::protocol::Team::Red),
                Vec2::ZERO,
            )],
            projectiles: Vec::new(),
        });

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

        g.state = GameState::Battle(Countdown::new(ROUND_DURATION));
        g.engine.apply_snapshot(EngineSnapshot {
            tanks: vec![Tank::new(
                PlayerInfo::new(0, "p1".into(), common::protocol::Team::Red),
                Vec2::ZERO,
            )],
            projectiles: Vec::new(),
        });

        g.tick(0.0);

        assert!(
            g.outgoing_events
                .iter()
                .any(|e| matches!(e, GameEvent::RoundEnded(_)))
        );
        assert!(matches!(g.game_state_info(), GameStateInfo::Results(_)));

        // Winner remains true on subsequent ticks; this must NOT underflow rounds_left or emit more RoundEnded.
        g.outgoing_events.clear();
        g.tick(0.0);
        assert!(
            !g.outgoing_events
                .iter()
                .any(|e| matches!(e, GameEvent::RoundEnded(_)))
        );
    }
}
