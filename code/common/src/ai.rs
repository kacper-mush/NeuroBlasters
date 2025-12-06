use crate::protocol::{GameStateSnapshot, InputPayload, MapDefinition, PlayerState};
use glam::Vec2;
use rand::SeedableRng;
use rand::rngs::StdRng;

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
    pub me: &'a PlayerState,
    pub world: &'a GameStateSnapshot,
    pub map: &'a MapDefinition,
    pub dt: f32,
    pub rng: &'a mut StdRng,
}

/// Allows us to swap "Scripted Logic" with "Neural Networks" instantly.
pub trait Policy: Send + Sync {
    fn compute_input(&mut self, ctx: &BotContext) -> InputPayload;
}

pub struct BotAgent {
    pub difficulty: BotDifficulty,
    policy: Box<dyn Policy>, // The active brain
    rng: StdRng,
}

impl BotAgent {
    pub fn new(difficulty: BotDifficulty, seed: u64) -> Self {
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
            difficulty,
            policy,
            rng,
        }
    }

    /// The Server calls this once per tick for every bot.
    pub fn generate_input(
        &mut self,
        me: &PlayerState,
        world: &GameStateSnapshot,
        map: &MapDefinition,
        dt: f32,
    ) -> InputPayload {
        let ctx = BotContext {
            me,
            world,
            map,
            dt,
            rng: &mut self.rng,
        };
        self.policy.compute_input(&ctx)
    }
}

#[derive(Clone, Copy)]
enum ScriptedBehavior {
    Turret,
    Wanderer,
    Hunter,
    Terminator,
}

struct ScriptedPolicy {
    behavior: ScriptedBehavior,
    // Add internal memory here if needed (e.g., timers)
    state_timer: f32,
}

impl ScriptedPolicy {
    fn new(behavior: ScriptedBehavior) -> Self {
        Self {
            behavior,
            state_timer: 0.0,
        }
    }
}

impl Policy for ScriptedPolicy {
    fn compute_input(&mut self, ctx: &BotContext) -> InputPayload {
        self.state_timer += ctx.dt;

        match self.behavior {
            ScriptedBehavior::Turret => {
                // TODO: Implement "Find target -> Aim with noise -> Shoot"
                InputPayload {
                    move_axis: Vec2::ZERO,
                    aim_pos: Vec2::ZERO,
                    shoot: false,
                }
            }
            ScriptedBehavior::Wanderer => {
                // TODO: Implement "Pick random point -> Walk -> Shoot if seen"
                InputPayload {
                    move_axis: Vec2::ZERO,
                    aim_pos: Vec2::ZERO,
                    shoot: false,
                }
            }
            ScriptedBehavior::Hunter => {
                // TODO: Implement "Chasing logic"
                InputPayload {
                    move_axis: Vec2::ZERO,
                    aim_pos: Vec2::ZERO,
                    shoot: false,
                }
            }
            ScriptedBehavior::Terminator => {
                // TODO: Implement "Predictive aiming"
                InputPayload {
                    move_axis: Vec2::ZERO,
                    aim_pos: Vec2::ZERO,
                    shoot: false,
                }
            }
        }
    }
}

// --- Dummy Policy ---

struct DummyPolicy;
impl Policy for DummyPolicy {
    fn compute_input(&mut self, _ctx: &BotContext) -> InputPayload {
        // Literally do nothing
        InputPayload {
            move_axis: Vec2::ZERO,
            aim_pos: Vec2::ZERO,
            shoot: false,
        }
    }
}

// ---  RL Policy (Mocked) ---

#[derive(Default)]
struct RlPolicy {
    // Future: This will hold your 'burn' model
}

impl Policy for RlPolicy {
    fn compute_input(&mut self, _ctx: &BotContext) -> InputPayload {
        // For now, it just mocks a Dummy
        InputPayload {
            move_axis: Vec2::ZERO,
            aim_pos: Vec2::ZERO,
            shoot: false,
        }
    }
}
