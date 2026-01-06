#![recursion_limit = "256"]
use burn::backend::{Autodiff, Wgpu};
use burn::module::Module;
use burn::record::{BinFileRecorder, FullPrecisionSettings};
use burn::tensor::backend::Backend;
use common::ai::BotContext;
use common::game::engine::GameEngine;
use common::net::protocol::{InputPayload, MapDefinition, Player, Team};
use common::rl::{extract_features, BotBrain};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

const GENERATIONS: usize = 1000;
const POPULATION_SIZE: usize = 64;
const MUTATION_RATE: f32 = 0.05;

type MyBackend = Wgpu;

#[tokio::main]
async fn main() {
    println!("Starting Vector-Based Evolution on GPU...");
    let device = Default::default();

    let mut population: Vec<BotBrain<MyBackend>> = (0..POPULATION_SIZE)
        .map(|_| BotBrain::new(&device))
        .collect();

    for gen in 1..=GENERATIONS {
        let mut next_gen_parents = Vec::new();
        fastrand::shuffle(&mut population);

        for (match_idx, match_chunk) in population.chunks(8).enumerate() {
            if match_chunk.len() < 8 {
                break;
            }

            let blue_team = &match_chunk[0..4];
            let red_team = &match_chunk[4..8];

            // Run match
            let stats = run_4v4_match(blue_team, red_team, &device);
            let blue_kills: i32 = stats
                .iter()
                .filter(|s| s.team == Team::Blue)
                .map(|s| s.kills)
                .sum();
            let red_kills: i32 = stats
                .iter()
                .filter(|s| s.team == Team::Red)
                .map(|s| s.kills)
                .sum();
            println!(
                "  > Match {}/8 finished. Kills - Blue: {}, Red: {}",
                match_idx + 1,
                blue_kills,
                red_kills
            );
            // Winner Calculation
            let blue_survivors = stats
                .iter()
                .filter(|s| s.team == Team::Blue && s.alive)
                .count();
            let red_survivors = stats
                .iter()
                .filter(|s| s.team == Team::Red && s.alive)
                .count();
            let winning_team = if blue_survivors > red_survivors {
                Team::Blue
            } else if red_survivors > blue_survivors {
                Team::Red
            } else {
                let bk: i32 = stats
                    .iter()
                    .filter(|s| s.team == Team::Blue)
                    .map(|s| s.kills)
                    .sum();
                let rk: i32 = stats
                    .iter()
                    .filter(|s| s.team == Team::Red)
                    .map(|s| s.kills)
                    .sum();
                if bk >= rk {
                    Team::Blue
                } else {
                    Team::Red
                }
            };

            // Selection Logic
            let mut winners: Vec<&BotStats> =
                stats.iter().filter(|s| s.team == winning_team).collect();
            winners.sort_by(|a, b| {
                let score_a = a.kills - a.friendly_kills;
                let score_b = b.kills - b.friendly_kills;
                match score_b.cmp(&score_a) {
                    std::cmp::Ordering::Equal => {
                        if a.alive && !b.alive {
                            std::cmp::Ordering::Less
                        } else if !a.alive && b.alive {
                            std::cmp::Ordering::Greater
                        } else {
                            std::cmp::Ordering::Equal
                        }
                    }
                    other => other,
                }
            });

            for i in 0..2 {
                if i < winners.len() {
                    next_gen_parents.push(match_chunk[winners[i].original_index].clone());
                }
            }
        }

        // Reproduction
        let mut new_pop = Vec::with_capacity(POPULATION_SIZE);
        for parent in &next_gen_parents {
            new_pop.push(parent.clone());
        }

        let mut rng = rand::rng();
        if next_gen_parents.is_empty() {
            new_pop = (0..POPULATION_SIZE)
                .map(|_| BotBrain::new(&device))
                .collect();
        } else {
            while new_pop.len() < POPULATION_SIZE {
                let parent = &next_gen_parents[rng.random_range(0..next_gen_parents.len())];
                new_pop.push(parent.mutate(MUTATION_RATE));
            }
        }
        population = new_pop;
        println!("Gen {} Complete.", gen);
        if gen % 10 == 0 {
            save_model(&population[0]);
        }
    }
}

// --- HELPER STRUCTURES ---
#[derive(Debug)]
struct BotStats {
    original_index: usize,
    team: Team,
    kills: i32,
    friendly_kills: i32,
    alive: bool,
}

// --- SIMULATION ---
fn run_4v4_match<B: Backend>(
    blue_brains: &[BotBrain<B>],
    red_brains: &[BotBrain<B>],
    device: &B::Device,
) -> Vec<BotStats> {
    let mut engine = GameEngine::new(MapDefinition::load());

    // Spawn Logic
    for (i, _) in blue_brains.iter().enumerate() {
        if i + 4 < engine.map.spawn_points.len() {
            let spawn = engine.map.spawn_points[i + 4].1;
            engine.add_player(Player::new(
                i as u64,
                format!("Blue_{}", i),
                Team::Blue,
                spawn,
            ));
        }
    }
    for (i, _) in red_brains.iter().enumerate() {
        if i < engine.map.spawn_points.len() {
            let spawn = engine.map.spawn_points[i].1;
            engine.add_player(Player::new(
                (i + 4) as u64,
                format!("Red_{}", i),
                Team::Red,
                spawn,
            ));
        }
    }

    let mut stats: Vec<BotStats> = (0..8)
        .map(|i| BotStats {
            original_index: i,
            team: if i < 4 { Team::Blue } else { Team::Red },
            kills: 0,
            friendly_kills: 0,
            alive: true,
        })
        .collect();

    for _ in 0..1000 {
        if engine
            .players
            .iter()
            .filter(|p| p.team == Team::Blue && p.health > 0.0)
            .count()
            == 0
        {
            break;
        }
        if engine
            .players
            .iter()
            .filter(|p| p.team == Team::Red && p.health > 0.0)
            .count()
            == 0
        {
            break;
        }

        let mut inputs = std::collections::HashMap::new();
        let mut rng = StdRng::seed_from_u64(0);

        for (i, player) in engine.players.iter().enumerate() {
            if player.health <= 0.0 {
                continue;
            }
            let ctx = BotContext {
                me: player,
                players: &engine.players,
                projectiles: &engine.projectiles,
                map: &engine.map,
                dt: 0.033,
                rng: &mut rng,
            };
            let brain = if i < 4 {
                &blue_brains[i]
            } else {
                &red_brains[i - 4]
            };

            // Get 5 Outputs
            let output_tensor = brain.forward(extract_features(&ctx, device));
            let values = output_tensor.into_data().to_vec::<f32>().unwrap();

            let input = action_to_input(&values, &ctx);
            inputs.insert(player.id, input);
        }

        let result = engine.tick(0.033, &inputs);

        for kill in result.kills {
            println!(
                "    [Kill] Player {} killed Player {}",
                kill.killer_id, kill.victim_id
            );
            if let Some(stat) = stats
                .iter_mut()
                .find(|s| s.original_index == kill.killer_id as usize)
            {
                let victim_team = if kill.victim_id < 4 {
                    Team::Blue
                } else {
                    Team::Red
                };
                if stat.team == victim_team {
                    stat.friendly_kills += 1;
                } else {
                    stat.kills += 1;
                }
            }
        }
    }

    for player in &engine.players {
        if player.id < 8 {
            stats[player.id as usize].alive = player.health > 0.0;
        }
    }
    stats
}

// --- NEW VECTOR INPUT LOGIC ---
fn action_to_input(actions: &[f32], ctx: &BotContext) -> InputPayload {
    // 1. Movement (Relative to Bot)
    let move_fwd = actions[0].tanh();
    let move_side = actions[1].tanh();

    // 2. Aiming (Relative Vector)
    // Values [2] and [3] form a vector direction relative to where we are currently looking.
    // e.g. (1, 0) means "Keep looking forward", (0, 1) means "Look Left"
    let aim_fwd = actions[2];
    let aim_side = actions[3];
    let shoot_val = actions[4];

    // Current Rotation
    let (sin, cos) = ctx.me.rotation.sin_cos();

    // A. Rotate Move Vector to World Space
    let world_move = glam::Vec2::new(
        move_fwd * cos - move_side * sin,
        move_fwd * sin + move_side * cos,
    );

    // B. Rotate Aim Vector to World Space
    // We treat aim_fwd/aim_side as a local vector
    let world_aim_dir = glam::Vec2::new(
        aim_fwd * cos - aim_side * sin,
        aim_fwd * sin + aim_side * cos,
    );

    // Safety: If vector is zero, just aim forward (world_aim_dir = current facing)
    let final_aim_dir = if world_aim_dir.length_squared() < 0.001 {
        glam::Vec2::new(cos, sin)
    } else {
        world_aim_dir.normalize()
    };

    // Calculate Aim Point (100 units away)
    let aim_pos = ctx.me.position + (final_aim_dir * 100.0);

    InputPayload {
        move_axis: world_move,
        aim_pos,
        shoot: shoot_val > 0.0,
    }
}

fn save_model<B: Backend>(model: &BotBrain<B>) {
    let recorder = BinFileRecorder::<FullPrecisionSettings>::default();
    let _ = std::fs::create_dir_all("assets");
    let _ = model.clone().save_file("assets/model", &recorder);
}
