use super::{
    apply_player_physics, check_round_winner, handle_shooting, resolve_combat,
    resolve_player_collisions, update_projectiles,
};
use crate::ai::{BotAgent, BotDifficulty};
use crate::game::player::HumanInfo;
use crate::net::protocol::{
    InputPayload, KillEvent, MapDefinition, PlayerId, Projectile, Tank, Team,
};
use glam::Vec2;
use std::collections::HashMap;

#[derive(Clone)]
pub struct GameEngine {
    pub tanks: Vec<Tank>,
    pub projectiles: Vec<Projectile>,
    pub map: MapDefinition,
    humans: Vec<HumanInfo>,
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
            tanks: Vec::new(),
            projectiles: Vec::new(),
            map,
            humans: Vec::new(),
            bots: Vec::new(),
            next_player_id: 0,
            projectile_id_counter: 0,
        }
    }

    /// Updates the game world by one tick.
    ///
    /// * `dt`: Delta time in seconds.
    /// * `inputs`: A map of inputs for each human tank. Missing entries mean idle.
    ///
    /// Returns a list of kills that happened during this tick.
    pub fn tick(&mut self, dt: f32, mut inputs: HashMap<PlayerId, InputPayload>) -> GameTickResult {
        self.inject_bot_inputs(&mut inputs, dt);

        for tank in &mut self.tanks {
            let default_input = InputPayload {
                move_axis: Vec2::ZERO,
                aim_pos: tank.position,
                shoot: false,
            };

            // Get input or use default (idle)
            let input = inputs.get(&tank.id).unwrap_or(&default_input);

            apply_player_physics(tank, input, &self.map, dt);

            // We use the engine's internal counter to assign IDs to new projectiles.
            if let Some(proj) = handle_shooting(tank, input, dt, self.projectile_id_counter) {
                self.projectiles.push(proj);
                self.projectile_id_counter += 1;
            }
        }

        // Resolves collisions between players (prevent overlapping)
        resolve_player_collisions(&mut self.tanks);

        // Process Projectiles (Move & Collide with walls)
        update_projectiles(&mut self.projectiles, &self.map, dt);

        // Resolve Combat (Projectiles hitting Players)
        // This function modifies health, removes dead players/bullets, and returns KillEvents.
        let kills = resolve_combat(&mut self.tanks, &mut self.projectiles);
        let winner = check_round_winner(&self.tanks);

        GameTickResult { kills, winner }
    }

    pub fn prepare_new_round(&mut self) {
        // Clear transient round state.
        self.tanks.clear();
        self.projectiles.clear();
        self.projectile_id_counter = 0;

        // Clear bots and recreate them to fill spawnpoints each round.
        self.bots.clear();

        // Split spawnpoints by team; order within a team doesn't matter.
        let mut red_spawns: Vec<Vec2> = Vec::new();
        let mut blue_spawns: Vec<Vec2> = Vec::new();
        for (team, pos) in &self.map.spawn_points {
            match team {
                Team::Red => red_spawns.push(*pos),
                Team::Blue => blue_spawns.push(*pos),
            }
        }

        // Spawn humans first (team fixed on join).
        for human in &self.humans {
            let id = human.id;
            let team = human.team;
            let nickname = human.nickname.clone();
            let pos = match team {
                Team::Red => red_spawns.pop(),
                Team::Blue => blue_spawns.pop(),
            }
            .or_else(|| self.random_free_position())
            .unwrap_or(Vec2::new(self.map.width * 0.5, self.map.height * 0.5));

            self.tanks.push(Tank::new(id, nickname, team, pos));
        }

        // Fill remaining spawnpoints with bots.
        for pos in red_spawns {
            self.spawn_bot(Team::Red, pos);
        }
        for pos in blue_spawns {
            self.spawn_bot(Team::Blue, pos);
        }
    }

    fn spawn_bot(&mut self, team: Team, pos: Vec2) {
        let bot_id = self.next_player_id;
        self.next_player_id += 1;

        let nickname = format!("Bot {}", bot_id);
        let bot = BotAgent::new(bot_id, BotDifficulty::Hunter, bot_id as u64);
        self.bots.push(bot);
        self.tanks.push(Tank::new(bot_id, nickname, team, pos));
    }

    /// Helper to inject a player (e.g. on spawn)
    pub fn add_player(&mut self, nickname: String) -> Result<PlayerId, String> {
        if self.humans.len() >= self.map.spawn_points.len() {
            return Err("Player limit reached".to_string());
        }

        let id = self.next_player_id;
        self.next_player_id += 1;

        let position = self
            .random_free_position()
            .ok_or("Failed to find a free position")?;
        let team = if self.humans.len().is_multiple_of(2) {
            Team::Blue
        } else {
            Team::Red
        };

        self.humans.push(HumanInfo::new(id, nickname.clone(), team));

        let tank = Tank::new(id, nickname, team, position);
        self.tanks.push(tank);
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
                .tanks
                .iter()
                .any(|p| p.position.distance_squared(candidate) < min_dist_sq);
            if !occupied {
                return Some(candidate);
            }
        }

        None
    }

    pub fn remove_player(&mut self, player_id: PlayerId) {
        self.humans.retain(|h| h.id != player_id);
        self.bots.retain(|b| b.id != player_id);
        self.tanks.retain(|tank| tank.id != player_id);
        self.projectiles.retain(|proj| proj.owner_id != player_id);
    }

    fn inject_bot_inputs(&mut self, inputs: &mut HashMap<PlayerId, InputPayload>, dt: f32) {
        // Snapshot borrows used during input generation.
        let tanks = &self.tanks;
        let projectiles = &self.projectiles;
        let map = &self.map;

        for bot in &mut self.bots {
            let me_id = bot.id;
            if let Some(me_index) = tanks.iter().position(|t| t.id == me_id) {
                let me = &tanks[me_index];
                let input = bot.generate_input(me, tanks, projectiles, map, dt);
                inputs.insert(me_id, input);
            }
        }
    }
}
