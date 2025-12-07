use super::{apply_player_physics, handle_shooting, resolve_combat, update_projectiles};
use crate::net::protocol::{GameStateSnapshot, InputPayload, KillEvent, MapDefinition, ClientId};
use glam::Vec2;
use std::collections::HashMap;

pub struct GameEngine {
    pub state: GameStateSnapshot,
    pub map: MapDefinition,
    projectile_id_counter: u64,
}

impl GameEngine {
    pub fn new(map: MapDefinition) -> Self {
        Self {
            state: GameStateSnapshot {
                players: Vec::new(),
                projectiles: Vec::new(),
                time_remaining: 0.0,
            },
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
    pub fn tick(&mut self, dt: f32, inputs: &HashMap<ClientId, InputPayload>) -> Vec<KillEvent> {
        for player in &mut self.state.players {
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
                self.state.projectiles.push(proj);
                self.projectile_id_counter += 1;
            }
        }

        // Process Projectiles (Move & Collide with walls)
        update_projectiles(&mut self.state.projectiles, &self.map, dt);

        // Resolve Combat (Projectiles hitting Players)
        // This function modifies health, removes dead players/bullets, and returns KillEvents.
        resolve_combat(&mut self.state.players, &mut self.state.projectiles)
    }

    /// Overwrites the current state with a snapshot.
    /// Used by the Client to snap to the Server's authoritative state (Reconciliation).
    pub fn sync_state(&mut self, snapshot: GameStateSnapshot) {
        self.state = snapshot;
        // Note: If visual smoothness is required later, we might want to
        // interpolate positions here instead of hard-overwriting.
    }

    /// Helper to inject a player (e.g. on spawn)
    pub fn add_player(&mut self, player: crate::protocol::PlayerState) {
        self.state.players.push(player);
    }
}
