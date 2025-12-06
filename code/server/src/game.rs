use std::collections::HashMap;
use std::time::{Duration, Instant};

use common::game_logic::{
    apply_player_physics, find_spawn_position, handle_shooting, resolve_combat, update_projectiles,
};
use common::{
    GameEvent, GameStateSnapshot, GameUpdate, InputPayload, MapDefinition, PlayerId, PlayerState,
    RectWall, Team, TickId,
};
use glam::Vec2;
use rand::rngs::StdRng;
use renet::ClientId;

use crate::connection::SessionInfo;

const DEFAULT_TIME_LIMIT: f32 = 180.0;
const PLAYER_RADIUS: f32 = 16.0;
const PLAYER_SPEED: f32 = 225.0;
const PLAYER_HEALTH: f32 = 100.0;

pub struct GameStartContext {
    pub initial_tick_id: TickId,
    pub map: MapDefinition,
    pub initial_update: GameUpdate,
}

pub struct GameInstance {
    next_tick_id: TickId,
    map: MapDefinition,
    state: GameStateSnapshot,
    inputs: HashMap<PlayerId, InputPayload>,
    client_players: HashMap<ClientId, PlayerId>,
    next_projectile_id: u64,
}

impl GameInstance {
    pub fn start(
        _started_at: Instant,
        members: Vec<(ClientId, SessionInfo)>,
        rng: &mut StdRng,
    ) -> Option<(Self, GameStartContext)> {
        if members.is_empty() {
            return None;
        }

        let map = default_map();
        let mut players = Vec::with_capacity(members.len());
        let mut client_players = HashMap::new();
        let mut inputs = HashMap::new();

        for (index, (client_id, session)) in members.into_iter().enumerate() {
            let player_id = PlayerId(session.session_id.0);
            let spawn = find_spawn_position(&map, PLAYER_RADIUS, rng)
                .unwrap_or_else(|| fallback_spawn(index));
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
            client_players.insert(client_id, player_id);
            inputs.insert(player_id, InputPayload::default());
        }

        let state = GameStateSnapshot {
            players,
            projectiles: Vec::new(),
            time_remaining: DEFAULT_TIME_LIMIT,
        };

        let initial_tick_id = TickId(0);
        let initial_update = GameUpdate {
            state: state.clone(),
            events: Vec::new(),
        };

        let instance = Self {
            next_tick_id: TickId(initial_tick_id.0 + 1),
            map: map.clone(),
            state,
            inputs,
            client_players,
            next_projectile_id: 1,
        };

        let context = GameStartContext {
            initial_tick_id,
            map,
            initial_update,
        };

        Some((instance, context))
    }

    pub fn submit_input(&mut self, client_id: ClientId, payload: InputPayload) {
        if let Some(player_id) = self.client_players.get(&client_id) {
            self.inputs.insert(*player_id, payload);
        }
    }

    pub fn advance(&mut self, delta: Duration) -> Option<(TickId, GameUpdate)> {
        if self.state.players.is_empty() {
            return None;
        }

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

        let tick_id = self.next_tick_id;
        self.next_tick_id.0 += 1;

        let update = GameUpdate {
            state: self.state.clone(),
            events,
        };

        Some((tick_id, update))
    }

    pub fn remove_client(&mut self, client_id: ClientId) -> bool {
        if let Some(player_id) = self.client_players.remove(&client_id) {
            self.inputs.remove(&player_id);
            self.state.players.retain(|player| player.id != player_id);
            self.state
                .projectiles
                .retain(|projectile| projectile.owner_id != player_id);
        }
        self.client_players.is_empty()
    }
}

fn fallback_spawn(index: usize) -> Vec2 {
    let spacing = PLAYER_RADIUS * 4.0;
    let row = (index / 4) as f32;
    let col = (index % 4) as f32;
    Vec2::new(
        PLAYER_RADIUS + col * spacing,
        PLAYER_RADIUS + row * spacing + 50.0,
    )
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
