use super::{
    DamageEvent, apply_player_physics, check_round_winner, handle_shooting, resolve_combat,
    resolve_player_collisions, update_projectiles,
};
use crate::net::protocol::{
    ClientId, InputPayload, KillEvent, MapDefinition, Player, Projectile, Team,
};
use glam::Vec2;
use std::collections::HashMap;

#[derive(Clone)]
pub struct GameEngine {
    pub players: Vec<Player>,
    pub projectiles: Vec<Projectile>,
    pub map: MapDefinition,
    projectile_id_counter: u64,
}

pub struct GameTickResult {
    pub kills: Vec<KillEvent>,
    pub damage: Vec<DamageEvent>,
    pub winner: Option<Team>,
}

impl GameEngine {
    pub fn new(map: MapDefinition) -> Self {
        Self {
            players: Vec::new(),
            projectiles: Vec::new(),
            map,
            projectile_id_counter: 0,
        }
    }

    /// Updates the game world by one tick.
    ///
    /// * `dt`: Delta time in seconds.
    /// * `inputs`: A map of inputs for each player. If a player has no input in this map, they stay still.
    ///
    /// Returns a list of kills that happened during this tick.
    pub fn tick(&mut self, dt: f32, inputs: &HashMap<ClientId, InputPayload>) -> GameTickResult {
        for player in &mut self.players {
            let default_input = InputPayload {
                move_axis: Vec2::ZERO,
                aim_pos: player.position,
                shoot: false,
            };

            // Get input or use default (idle)
            let input = inputs.get(&player.id).unwrap_or(&default_input);

            apply_player_physics(player, input, &self.map, dt);

            // We use the engine's internal counter to assign IDs to new projectiles.
            if let Some(proj) = handle_shooting(player, input, dt, self.projectile_id_counter) {
                self.projectiles.push(proj);
                self.projectile_id_counter += 1;
            }
        }

        // Resolves collisions between players (prevent overlapping)
        resolve_player_collisions(&mut self.players);

        // Process Projectiles (Move & Collide with walls)
        update_projectiles(&mut self.projectiles, &self.map, dt);

        // Resolve Combat (Projectiles hitting Players)
        // This function modifies health, removes dead players/bullets, and returns KillEvents.
        let (kills, damage) = resolve_combat(&mut self.players, &mut self.projectiles);
        let winner = check_round_winner(&self.players);

        GameTickResult {
            kills,
            damage,
            winner,
        }
    }

    /// Helper to inject a player (e.g. on spawn)
    pub fn add_player(&mut self, player: Player) {
        self.players.push(player);
    }
}
