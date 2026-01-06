use super::{
    apply_player_physics, check_round_winner, handle_shooting, resolve_combat, update_projectiles,
};
use crate::ai::{BotAgent, BotDifficulty};
use crate::net::protocol::{
    InputPayload, KillEvent, MapDefinition, Player, PlayerId, Projectile, Team,
};
use glam::Vec2;
use std::collections::HashMap;

#[derive(Clone)]
pub struct GameEngine {
    pub players: Vec<Player>,
    pub projectiles: Vec<Projectile>,
    pub map: MapDefinition,
    pub bots: Vec<BotAgent>,
    next_player_id: PlayerId,
    projectile_id_counter: u64,
}

pub struct GameTickResult {
    pub kills: Vec<KillEvent>,
    pub winner: Option<Team>,
}

impl GameEngine {
    pub fn new(map: MapDefinition) -> Self {
        Self {
            players: Vec::new(),
            projectiles: Vec::new(),
            map,
            bots: Vec::new(),
            next_player_id: 0,
            projectile_id_counter: 0,
        }
    }

    /// Updates the game world by one tick.
    ///
    /// * `dt`: Delta time in seconds.
    /// * `inputs`: A map of inputs for each player. If a player has no input in this map, they stay still.
    ///
    /// Returns a list of kills that happened during this tick.
    pub fn tick(
        &mut self,
        dt: f32,
        mut all_inputs: HashMap<PlayerId, InputPayload>,
    ) -> GameTickResult {
        for i in 0..self.bots.len() {
            let bot = &mut self.bots[i];
            let me_id = bot.id;
            let players = &self.players;
            let projectiles = &self.projectiles;
            let map = &self.map;

            if let Some(me_index) = players.iter().position(|p| p.id == me_id) {
                let me = &players[me_index];
                let input = self.bots[i].generate_input(me, players, projectiles, map, dt);
                all_inputs.insert(me_id, input);
            }
        }

        for player in &mut self.players {
            let default_input = InputPayload {
                move_axis: Vec2::ZERO,
                aim_pos: player.position,
                shoot: false,
            };

            // Get input or use default (idle)
            let input = all_inputs.get(&player.id).unwrap_or(&default_input);

            apply_player_physics(player, input, &self.map, dt);

            // We use the engine's internal counter to assign IDs to new projectiles.
            if let Some(proj) = handle_shooting(player, input, dt, self.projectile_id_counter) {
                self.projectiles.push(proj);
                self.projectile_id_counter += 1;
            }
        }

        // Process Projectiles (Move & Collide with walls)
        update_projectiles(&mut self.projectiles, &self.map, dt);

        // Resolve Combat (Projectiles hitting Players)
        // This function modifies health, removes dead players/bullets, and returns KillEvents.
        let kills = resolve_combat(&mut self.players, &mut self.projectiles);
        let winner = check_round_winner(&self.players);

        GameTickResult { kills, winner }
    }

    pub fn move_players_to_spawnpoints(&mut self, current_player_ids: &[PlayerId]) {
        self.bots.clear();

        self.players.retain(|p| current_player_ids.contains(&p.id));

        let spawn_points = self.map.spawn_points.clone();

        let mut used_indices = Vec::new();

        for (i, player) in self.players.iter_mut().enumerate() {
            if i < spawn_points.len() {
                let (team, pos) = spawn_points[i];
                player.position = pos;
                player.team = team;
                player.velocity = Vec2::ZERO;
                player.health = 100.0;
                used_indices.push(i);
            }
        }

        for (i, (team, pos)) in spawn_points.iter().enumerate() {
            if !used_indices.contains(&i) {
                let bot_id = self.next_player_id;
                self.next_player_id += 1;

                let nickname = format!("Bot {}", bot_id);
                let player = Player::new(bot_id, nickname, *team, *pos);
                self.players.push(player);

                let bot = BotAgent::new(bot_id, BotDifficulty::Hunter, bot_id as u64);
                self.bots.push(bot);
            }
        }
    }

    /// Helper to inject a player (e.g. on spawn)
    pub fn add_player(&mut self, nickname: String) -> Result<PlayerId, String> {
        if self.players.len() >= self.map.spawn_points.len() {
            return Err("Player limit reached".to_string());
        }

        let id = self.next_player_id;
        self.next_player_id += 1;

        let position = self
            .random_free_position()
            .ok_or("Failed to find a free position")?;
        let team = if self.players.len().is_multiple_of(2) {
            Team::Blue
        } else {
            Team::Red
        };

        let player = Player::new(id, nickname, team, position);
        self.players.push(player);
        Ok(id)
    }

    fn random_free_position(&self) -> Option<Vec2> {
        use rand::Rng;
        let mut rng = rand::rng();
        let max_attempts = 50;
        let map_padding = 20.0;
        let min_dist_sq = 35.0 * 35.0;

        let map_width = self.map.width;
        let map_height = self.map.height;

        for _ in 0..max_attempts {
            let x = rng.random_range(map_padding..(map_width - map_padding));
            let y = rng.random_range(map_padding..(map_height - map_padding));
            let candidate = Vec2::new(x, y);

            let occupied = self
                .players
                .iter()
                .any(|p| p.position.distance_squared(candidate) < min_dist_sq);
            if !occupied {
                return Some(candidate);
            }
        }

        None
    }

    pub fn remove_player(&mut self, player_id: PlayerId) {
        self.players.retain(|player| player.id != player_id);
    }
}
