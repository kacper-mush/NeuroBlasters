pub mod pathfinding;

use self::pathfinding::find_path_a_star;
use crate::game::PROJECTILE_SPEED;
use crate::net::protocol::objects::{InputPayload, MapDefinition, Player, PlayerId, Projectile};
use glam::Vec2;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BotDifficulty {
    Dummy,         // Does nothing
    Turret,        // Static, shoots when he sees you
    Wanderer,      // Moves randomly, shoots
    Hunter,        // Hunts you down
    Terminator,    // Hunts you down but better
    TrainedKiller, // Neural Network (RL bot)
}

/// Everything a bot is allowed to know to make a decision.
pub struct BotContext<'a> {
    pub me: &'a Player,
    pub players: &'a Vec<Player>,
    pub projectiles: &'a Vec<Projectile>,
    pub map: &'a MapDefinition,
    pub dt: f32,
    pub rng: &'a mut StdRng,
}

// Clone support for Policy
pub trait PolicyClone {
    fn clone_box(&self) -> Box<dyn Policy>;
}

impl<T> PolicyClone for T
where
    T: 'static + Policy + Clone,
{
    fn clone_box(&self) -> Box<dyn Policy> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn Policy> {
    fn clone(&self) -> Box<dyn Policy> {
        self.clone_box()
    }
}

/// Allows us to swap "Scripted Logic" with "Neural Networks" instantly.
pub trait Policy: Send + Sync + PolicyClone {
    fn compute_input(&mut self, ctx: &mut BotContext) -> InputPayload;
}

#[derive(Clone)]
pub struct BotAgent {
    pub id: PlayerId,
    pub difficulty: BotDifficulty,
    policy: Box<dyn Policy>, // The active brain
    rng: StdRng,
}

impl BotAgent {
    pub fn new(id: PlayerId, difficulty: BotDifficulty, seed: u64) -> Self {
        let rng = StdRng::seed_from_u64(seed);

        // Factory: Pick the right brain based on difficulty
        let policy: Box<dyn Policy> = match difficulty {
            BotDifficulty::Dummy => Box::new(DummyPolicy),
            BotDifficulty::Turret => Box::new(ScriptedPolicy::new(ScriptedBehavior::Turret)),
            BotDifficulty::Wanderer => Box::new(ScriptedPolicy::new(ScriptedBehavior::Wanderer)),
            BotDifficulty::Hunter => Box::new(ScriptedPolicy::new(ScriptedBehavior::Hunter)),
            BotDifficulty::Terminator => {
                Box::new(ScriptedPolicy::new(ScriptedBehavior::Terminator))
            }
            BotDifficulty::TrainedKiller => Box::new(RlPolicy::default()),
        };

        Self {
            id,
            difficulty,
            policy,
            rng,
        }
    }

    /// The Server calls this once per tick for every bot.
    pub fn generate_input(
        &mut self,
        me: &Player,
        players: &Vec<Player>,
        projectiles: &Vec<Projectile>,
        map: &MapDefinition,
        dt: f32,
    ) -> InputPayload {
        let mut ctx = BotContext {
            me,
            players,
            projectiles,
            map,
            dt,
            rng: &mut self.rng,
        };
        self.policy.compute_input(&mut ctx)
    }
}

// ---- Helper functions for scripted behaviours ----

/// Finds the closest living enemy to the bot.
fn find_closest_enemy<'a>(ctx: &BotContext<'a>) -> Option<&'a Player> {
    ctx.players
        .iter()
        .filter(|p| {
            p.health > 0.0                  // Alive
            && p.id != ctx.me.id            // Not me
            && p.team != ctx.me.team // Not teammate
        })
        .min_by(|p1, p2| {
            let d1 = ctx.me.position.distance_squared(p1.position);
            let d2 = ctx.me.position.distance_squared(p2.position);
            d1.partial_cmp(&d2).unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// Checks if a straight line between p1 and p2 is clear of walls AND other players.
fn has_line_of_sight(ctx: &BotContext, p1: Vec2, p2: Vec2) -> bool {
    let diff = p2 - p1;
    let dist = diff.length();

    if dist < 1.0 {
        return true;
    }

    let dir = diff / dist;
    let step_size = 10.0; // Check every 10 units
    let num_steps = (dist / step_size).ceil() as usize;

    for i in 1..num_steps {
        let sample_point = p1 + dir * (i as f32 * step_size);

        // Check Walls (Existing logic)
        for wall in &ctx.map.walls {
            if sample_point.x >= wall.min.x
                && sample_point.x <= wall.max.x
                && sample_point.y >= wall.min.y
                && sample_point.y <= wall.max.y
            {
                return false; // Blocked by wall
            }
        }

        // Check Players
        // We don't want to shoot if ANY player (teammate or enemy) is in the way.
        // Obviously, we ignore the shooter (ctx.me) and the target (at p2).
        for player in ctx.players {
            if player.id == ctx.me.id {
                continue; // Ignore self
            }

            // Simple Circle Collision Check
            // If the sample point is inside a player's radius, the shot is blocked.
            let collision_radius = player.radius;
            if sample_point.distance_squared(player.position) < collision_radius * collision_radius
            {
                // If this is the target we are aiming at, it's fine!
                // But we are checking points along the path. If we hit a player
                // before the end of the ray, it's an obstruction.
                // Since we step from start to end, checking if distance to player < distance to target
                // is implicitly handled by the loop order, but to be safe:
                if player.position.distance_squared(p1) < dist * dist {
                    return false; // Blocked by a player
                }
            }
        }
    }

    true
}

/// Calculates where to aim to hit a moving target (Interception).
fn predict_aim_position(shooter_pos: Vec2, target_pos: Vec2, target_vel: Vec2) -> Vec2 {
    let to_target = target_pos - shooter_pos;
    let target_speed_sq = target_vel.length_squared();
    let proj_speed_sq = PROJECTILE_SPEED * PROJECTILE_SPEED;

    // Quadratic equation coefficients: a*t^2 + b*t + c = 0
    let a = target_speed_sq - proj_speed_sq;
    let b = 2.0 * to_target.dot(target_vel);
    let c = to_target.length_squared();

    // If target is stationary, aim directly
    if target_speed_sq < 0.001 {
        return target_pos;
    }

    // Solve quadratic
    let discriminant = b * b - 4.0 * a * c;

    let mut t = 0.0;
    if discriminant >= 0.0 {
        let sqrt_d = discriminant.sqrt();
        let t1 = (-b - sqrt_d) / (2.0 * a);
        let t2 = (-b + sqrt_d) / (2.0 * a);

        // We want the smallest positive time
        if t1 > 0.0 && t2 > 0.0 {
            t = t1.min(t2);
        } else if t1 > 0.0 {
            t = t1;
        } else if t2 > 0.0 {
            t = t2;
        }
    }

    // If no solution (discriminant < 0) or negative time, fallback to direct aim
    if t <= 0.0 {
        return target_pos;
    }

    // Predicted Position = Current + Velocity * Time
    target_pos + target_vel * t
}

#[derive(Clone, Copy)]
enum ScriptedBehavior {
    Turret,
    Wanderer,
    Hunter,
    Terminator,
}

#[derive(Clone)]
struct ScriptedPolicy {
    behavior: ScriptedBehavior,
    state_timer: f32,
    target_pos: Option<Vec2>,

    // Pathfinding State
    path: Vec<Vec2>,
    path_recalc_timer: f32,
}

impl ScriptedPolicy {
    fn new(behavior: ScriptedBehavior) -> Self {
        Self {
            behavior,
            state_timer: 0.0,
            target_pos: None,
            path: Vec::new(),
            path_recalc_timer: 0.0,
        }
    }

    // --- LOGIC IMPLEMENTATIONS ---

    fn turret_logic(&self, ctx: &BotContext) -> InputPayload {
        // (Same as before: Scan visible, shoot closest visible, else idle)
        let mut visible_enemies = Vec::new();

        for p in ctx.players {
            if p.health > 0.0 && p.id != ctx.me.id && p.team != ctx.me.team {
                let dist_sq = ctx.me.position.distance_squared(p.position);
                if has_line_of_sight(ctx, ctx.me.position, p.position) {
                    visible_enemies.push((dist_sq, p.position));
                }
            }
        }

        if let Some((_, pos)) = visible_enemies
            .iter()
            .min_by(|(d1, _), (d2, _)| d1.partial_cmp(d2).unwrap_or(std::cmp::Ordering::Equal))
        {
            return InputPayload {
                move_axis: Vec2::ZERO,
                aim_pos: *pos,
                shoot: true,
            };
        }

        InputPayload {
            move_axis: Vec2::ZERO,
            aim_pos: ctx.me.position,
            shoot: false,
        }
    }

    fn wanderer_logic(&mut self, ctx: &mut BotContext) -> InputPayload {
        // Movement: Pick a random spot and go there
        if self.target_pos.is_none() {
            // Pick a random valid point on the map
            // (Simple retry logic to avoid spawning inside walls)
            for _ in 0..10 {
                let x = ctx.rng.random_range(15.0..ctx.map.width - 15.0);
                let y = ctx.rng.random_range(15.0..ctx.map.height - 15.0);
                let candidate = Vec2::new(x, y);

                // Check if the point is safe (not inside a wall)
                // and if we can walk straight to it (line of sight)
                if crate::game::is_position_safe(candidate, ctx.me.radius, ctx.map)
                    && has_line_of_sight(ctx, ctx.me.position, candidate)
                {
                    self.target_pos = Some(candidate);
                    break;
                }
            }
        }

        let mut move_axis = Vec2::ZERO;
        if let Some(target) = self.target_pos {
            let diff = target - ctx.me.position;
            if diff.length_squared() < 100.0 {
                // Reached within 10 units
                self.target_pos = None; // Pick new target next frame
            } else {
                move_axis = diff.normalize_or_zero();
            }
        }

        // Combat: Scan for targets while walking
        // Wanderer only shoots if it has a clear shot, otherwise it just walks.
        let mut shoot = false;
        let mut aim_pos = ctx.me.position + move_axis * 100.0; // Look where walking by default

        // Reuse helper: Find closest enemy
        if let Some(enemy) = find_closest_enemy(ctx) {
            let enemy_pos = enemy.position;
            if has_line_of_sight(ctx, ctx.me.position, enemy_pos) {
                aim_pos = enemy_pos;
                shoot = true;
            }
        }

        InputPayload {
            move_axis,
            aim_pos,
            shoot,
        }
    }

    fn hunter_logic(&mut self, ctx: &mut BotContext) -> InputPayload {
        if let Some(enemy) = find_closest_enemy(ctx) {
            let mut move_axis = Vec2::ZERO;
            let mut shoot = false;
            let enemy_pos = enemy.position;
            let aim_pos = enemy_pos; // Always look at enemy

            // Pathfinding Logic
            self.path_recalc_timer -= ctx.dt;

            // Recalculate path periodically (e.g., every 0.2 seconds)
            if self.path_recalc_timer <= 0.0 {
                self.path_recalc_timer = 0.2;
                // Now using the imported function
                self.path = find_path_a_star(ctx.me.position, enemy_pos, ctx.map);
            }

            // Follow Path
            if let Some(waypoint) = self.path.first() {
                let diff = *waypoint - ctx.me.position;
                if diff.length_squared() < 10.0 * 10.0 {
                    self.path.remove(0);
                } else {
                    move_axis = diff.normalize_or_zero();
                }
            } else {
                let diff = enemy_pos - ctx.me.position;
                if diff.length_squared() > 100.0 * 100.0 {
                    move_axis = diff.normalize_or_zero();
                }
            }

            // Shoot Logic
            if has_line_of_sight(ctx, ctx.me.position, enemy_pos) {
                shoot = true;
            }

            InputPayload {
                move_axis,
                aim_pos,
                shoot,
            }
        } else {
            self.wanderer_logic(ctx)
        }
    }

    fn terminator_logic(&mut self, ctx: &mut BotContext) -> InputPayload {
        if let Some(enemy) = find_closest_enemy(ctx) {
            let mut move_axis = Vec2::ZERO;
            let mut shoot = false;

            // Movement
            self.path_recalc_timer -= ctx.dt;
            if self.path_recalc_timer <= 0.0 {
                self.path_recalc_timer = 0.2;
                self.path = find_path_a_star(ctx.me.position, enemy.position, ctx.map);
            }

            if let Some(waypoint) = self.path.first() {
                let diff = *waypoint - ctx.me.position;
                if diff.length_squared() < 10.0 * 10.0 {
                    self.path.remove(0);
                } else {
                    move_axis = diff.normalize_or_zero();
                }
            } else {
                let diff = enemy.position - ctx.me.position;
                if diff.length_squared() > 100.0 * 100.0 {
                    move_axis = diff.normalize_or_zero();
                }
            }

            // Predictive Aiming (The "Terminator" part)
            let aim_pos = predict_aim_position(ctx.me.position, enemy.position, enemy.velocity);

            // Fire
            if has_line_of_sight(ctx, ctx.me.position, aim_pos) {
                shoot = true;
            }

            InputPayload {
                move_axis,
                aim_pos,
                shoot,
            }
        } else {
            self.wanderer_logic(ctx)
        }
    }
}

impl Policy for ScriptedPolicy {
    fn compute_input(&mut self, ctx: &mut BotContext) -> InputPayload {
        self.state_timer += ctx.dt;

        match self.behavior {
            ScriptedBehavior::Turret => self.turret_logic(ctx),
            ScriptedBehavior::Wanderer => self.wanderer_logic(ctx),
            ScriptedBehavior::Hunter => self.hunter_logic(ctx),
            ScriptedBehavior::Terminator => self.terminator_logic(ctx),
        }
    }
}
// --- Dummy Policy ---

#[derive(Clone)]
struct DummyPolicy;
impl Policy for DummyPolicy {
    fn compute_input(&mut self, _ctx: &mut BotContext) -> InputPayload {
        // Literally do nothing
        InputPayload {
            move_axis: Vec2::ZERO,
            aim_pos: Vec2::ZERO,
            shoot: false,
        }
    }
}

// ---  RL Policy (Mocked) ---

#[derive(Default, Clone)]
struct RlPolicy {
    // Future: This will hold your 'burn' model
}

impl Policy for RlPolicy {
    fn compute_input(&mut self, _ctx: &mut BotContext) -> InputPayload {
        // For now, it just mocks a Dummy
        InputPayload {
            move_axis: Vec2::ZERO,
            aim_pos: Vec2::ZERO,
            shoot: false,
        }
    }
}
