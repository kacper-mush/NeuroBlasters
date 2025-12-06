use std::collections::HashMap;
use std::time::Duration;

use common::game_logic::{
    apply_player_physics, check_round_winner, find_spawn_position, handle_shooting, resolve_combat,
    update_projectiles,
};
use common::{
    GameEvent, GameStateSnapshot, InputPayload, MapDefinition, PlayerId, PlayerState, RectWall,
    Team,
};
use glam::Vec2;
use rand::rngs::StdRng;
use renet::ClientId;
use thiserror::Error;

const DEFAULT_TIME_LIMIT: f32 = 180.0;
const PLAYER_RADIUS: f32 = 16.0;
const PLAYER_SPEED: f32 = 225.0;
const PLAYER_HEALTH: f32 = 100.0;

#[derive(Debug, Error)]
pub enum GameStartError {
    #[error("game requires at least one player")]
    NoPlayers,
    #[error("unable to find spawn position for player {player_index}")]
    SpawnPositionUnavailable { player_index: usize },
}

pub struct GameFrame {
    pub state: GameStateSnapshot,
    pub events: Vec<GameEvent>,
    pub winner: Option<Team>,
}

pub struct GameInstance {
    map: MapDefinition,
    state: GameStateSnapshot,
    inputs: HashMap<PlayerId, InputPayload>,
    client_players: HashMap<ClientId, PlayerId>,
    next_projectile_id: u64,
}

impl GameInstance {
    pub fn start(members: &[ClientId], rng: &mut StdRng) -> Result<Self, GameStartError> {
        if members.len() < 2 {
            return Err(GameStartError::NoPlayers);
        }

        let map = default_map();
        let mut players = Vec::with_capacity(members.len());
        let mut client_players = HashMap::new();
        let mut inputs = HashMap::new();

        for (index, client_id) in members.iter().enumerate() {
            let player_id = PlayerId(*client_id);
            let spawn = find_spawn_position(&map, PLAYER_RADIUS, rng).ok_or(
                GameStartError::SpawnPositionUnavailable {
                    player_index: index,
                },
            )?;
            let team = if index % 2 == 0 {
                Team::Blue
            } else {
                Team::Red
            };
            let player = PlayerState {
                id: player_id,
                team,
                position: spawn,
                velocity: Vec2::ZERO,
                rotation: 0.0,
                radius: PLAYER_RADIUS,
                speed: PLAYER_SPEED,
                health: PLAYER_HEALTH,
                weapon_cooldown: 0.0,
            };
            players.push(player);
            client_players.insert(*client_id, player_id);
            inputs.insert(player_id, InputPayload::default());
        }

        let state = GameStateSnapshot {
            players,
            projectiles: Vec::new(),
            time_remaining: DEFAULT_TIME_LIMIT,
        };

        let instance = Self {
            map: map.clone(),
            state,
            inputs,
            client_players,
            next_projectile_id: 1,
        };

        Ok(instance)
    }

    pub fn submit_input(&mut self, client_id: ClientId, payload: InputPayload) {
        if let Some(player_id) = self.client_players.get(&client_id) {
            self.inputs.insert(*player_id, payload);
        }
    }

    pub fn advance(&mut self, delta: Duration) -> GameFrame {
        let dt = delta.as_secs_f32();

        for player in &mut self.state.players {
            let input = self.inputs.get(&player.id).cloned().unwrap_or_default();
            apply_player_physics(player, &input, &self.map, dt);
            if let Some(projectile) = handle_shooting(player, &input, dt, self.next_projectile_id) {
                self.next_projectile_id += 1;
                self.state.projectiles.push(projectile);
            }
        }

        update_projectiles(&mut self.state.projectiles, &self.map, dt);
        let kills = resolve_combat(&mut self.state.players, &mut self.state.projectiles);
        let events = kills.into_iter().map(GameEvent::Kill).collect();

        self.state.time_remaining = (self.state.time_remaining - dt).max(0.0);

        let winner = check_round_winner(&self.state.players);
        GameFrame {
            state: self.state.clone(),
            events,
            winner,
        }
    }

    pub fn remove_client(&mut self, client_id: ClientId) {
        if let Some(player_id) = self.client_players.remove(&client_id) {
            self.inputs.remove(&player_id);
            self.state.players.retain(|player| player.id != player_id);
            self.state
                .projectiles
                .retain(|projectile| projectile.owner_id != player_id);
        }
    }

    pub fn get_map(&self) -> &MapDefinition {
        &self.map
    }

    pub fn get_state(&self) -> &GameStateSnapshot {
        &self.state
    }

    pub fn is_empty(&self) -> bool {
        self.client_players.is_empty()
    }
}

fn default_map() -> MapDefinition {
    MapDefinition {
        width: 1000.0,
        height: 1000.0,
        walls: vec![RectWall {
            min: Vec2::new(400.0, 400.0),
            max: Vec2::new(600.0, 600.0),
        }],
    }
}
