#![recursion_limit = "256"]
use burn::backend::Wgpu;
use burn::module::Module;
use burn::record::{BinFileRecorder, FullPrecisionSettings};
use burn::tensor::backend::Backend;
use clap::Parser;
use common::ai::BotContext;
use common::game::engine::GameEngine;
use common::net::protocol::{InputPayload, MapDefinition, PlayerId, Tank, Team};
use common::rl::{extract_features, BotBrain};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

type MyBackend = Wgpu;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value_t = 1000)]
    generations: usize,

    #[arg(long, default_value_t = 64)]
    population_size: usize,

    #[arg(long, default_value_t = 0.05)]
    mutation_rate: f32,

    #[arg(long, default_value_t = 1000)]
    max_ticks: usize,

    /// Name of the model to load/save (without extension).
    /// Saved to assets/models/<name>.bin
    #[arg(long, default_value = "default_model")]
    model_name: String,
}

/// Helper logic to locate the assets directory.
/// Recursively checks parent directories until "assets" is found.
/// 1. checks ./assets
/// 2. checks ../assets
/// 3. checks ../../assets
/// ...
fn resolve_assets_path(start_dir: &Path) -> PathBuf {
    for ancestor in start_dir.ancestors() {
        let candidate = ancestor.join("assets");
        if candidate.exists() && candidate.is_dir() {
            return candidate;
        }
    }

    // Fallback: If absolutely nothing is found, default to ./assets
    // This allows the program to create it there if needed.
    start_dir.join("assets")
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    println!("Starting Spartan Evolution on GPU (Parallel)...");
    println!("Configuration: {:?}", args);

    let device = Default::default();
    let recorder = BinFileRecorder::<FullPrecisionSettings>::default();

    // --- Dynamic Path Resolution ---
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let assets_root = resolve_assets_path(&current_dir);
    let models_dir = assets_root.join("models");

    // Canonicalize for pretty printing (resolves .. and .), fallback to raw path if fails
    let pretty_path = assets_root
        .canonicalize()
        .unwrap_or_else(|_| assets_root.clone());
    println!("Resolved assets directory: {:?}", pretty_path);

    // Ensure directory exists
    if let Err(e) = std::fs::create_dir_all(&models_dir) {
        eprintln!(
            "Failed to create models directory at {:?}: {}",
            models_dir, e
        );
        return;
    }

    // Path base: <assets_root>/models/<name>
    // Burn often expects &str path prefixes, so we convert here
    let model_path_buf = models_dir.join(&args.model_name);
    let model_path_str = model_path_buf
        .to_str()
        .expect("Path contains invalid unicode")
        .to_string();

    // 1. Initialize Population
    // Try to load existing model to start with, otherwise random
    let initial_brain =
        match BotBrain::<MyBackend>::new(&device).load_file(&model_path_str, &recorder, &device) {
            Ok(brain) => {
                println!("Loaded existing model: {}.bin", args.model_name);
                brain
            }
            Err(_) => {
                println!(
                    "No existing model found at {}.bin. Starting from scratch.",
                    model_path_str
                );
                BotBrain::new(&device)
            }
        };

    let mut population: Vec<BotBrain<MyBackend>> = (0..args.population_size)
        .map(|_| initial_brain.mutate(args.mutation_rate)) // Slight mutation from base to create diversity
        .collect();

    if population.is_empty() {
        population.push(BotBrain::new(&device));
    }

    for gen in 1..=args.generations {
        let next_gen_parents = Arc::new(Mutex::new(Vec::new()));

        fastrand::shuffle(&mut population);

        thread::scope(|s| {
            for (match_idx, match_chunk) in population.chunks(8).enumerate() {
                if match_chunk.len() < 8 {
                    continue;
                }

                let parents_handle = next_gen_parents.clone();
                let device = device.clone();
                let blue_team = match_chunk[0..4].to_vec();
                let red_team = match_chunk[4..8].to_vec();
                let max_ticks = args.max_ticks;

                s.spawn(move || {
                    let stats = run_4v4_match(&blue_team, &red_team, &device, max_ticks);

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

                    let blue_alive = stats
                        .iter()
                        .filter(|s| s.team == Team::Blue && s.alive)
                        .count();
                    let red_alive = stats
                        .iter()
                        .filter(|s| s.team == Team::Red && s.alive)
                        .count();

                    let winning_team = if blue_alive > red_alive {
                        Some(Team::Blue)
                    } else if red_alive > blue_alive {
                        Some(Team::Red)
                    } else {
                        if blue_kills > red_kills {
                            Some(Team::Blue)
                        } else if red_kills > blue_kills {
                            Some(Team::Red)
                        } else {
                            None
                        }
                    };

                    if blue_kills > 0 || red_kills > 0 {
                        println!(
                            "  > Match {} ended. Kills B:{}/R:{}. Winner: {:?}",
                            match_idx + 1,
                            blue_kills,
                            red_kills,
                            winning_team
                        );
                    }

                    let mut candidates: Vec<&BotStats> = if let Some(winner) = winning_team {
                        stats.iter().filter(|s| s.team == winner).collect()
                    } else {
                        stats.iter().collect()
                    };

                    candidates.sort_by(|a, b| b.total_score.partial_cmp(&a.total_score).unwrap());

                    let mut guard = parents_handle.lock().unwrap();
                    for i in 0..2 {
                        if let Some(stat) = candidates.get(i) {
                            let original_idx = stat.original_index;

                            let brain = if original_idx < 4 {
                                &blue_team[original_idx]
                            } else {
                                &red_team[original_idx - 4]
                            };
                            guard.push(brain.clone());
                        }
                    }
                });
            }
        });

        let next_gen_parents = Arc::try_unwrap(next_gen_parents)
            .unwrap()
            .into_inner()
            .unwrap();

        let mut new_pop = Vec::with_capacity(args.population_size);

        for parent in &next_gen_parents {
            new_pop.push(parent.clone());
        }

        let mut rng = rand::rng();
        if next_gen_parents.is_empty() {
            println!("  ! Extinction (No Winners). Resetting population.");
            new_pop = (0..args.population_size)
                .map(|_| BotBrain::new(&device))
                .collect();
        } else {
            while new_pop.len() < args.population_size {
                let parent = &next_gen_parents[rng.random_range(0..next_gen_parents.len())];
                new_pop.push(parent.mutate(args.mutation_rate));
            }
        }

        population = new_pop;
        println!("Gen {} Complete. Saving to {}", gen, model_path_str);

        // --- Atomic Save ---
        // Save to a temporary file first, then rename to ensure the client doesn't read a partial file.
        // Burn's save_file appends .bin, so if we provide "name_tmp", it writes "name_tmp.bin"

        let temp_name = format!("{}_tmp", args.model_name);

        // Construct paths using PathBuf for robustness
        let temp_file_path = models_dir.join(format!("{}.bin", temp_name));
        let final_file_path = models_dir.join(format!("{}.bin", args.model_name));

        // Base path string for Burn (it appends .bin)
        let temp_base_path = models_dir.join(&temp_name);
        let temp_base_str = temp_base_path.to_str().expect("Invalid path string");

        if let Ok(_) = population[0].clone().save_file(temp_base_str, &recorder) {
            // Rename overwrites atomically on POSIX, and usually works on Windows if target exists.
            let _ = std::fs::rename(temp_file_path, final_file_path);
        }
    }
}

#[derive(Debug)]
struct BotStats {
    original_index: usize,
    team: Team,
    kills: i32,
    friendly_kills: i32,
    alive: bool,
    total_score: f32,
}

fn run_4v4_match<B: Backend>(
    blue_brains: &[BotBrain<B>],
    red_brains: &[BotBrain<B>],
    device: &B::Device,
    max_ticks: usize,
) -> Vec<BotStats> {
    let mut engine = GameEngine::new(MapDefinition::load());

    // Spawn Blue (Face East 0.0)
    for (i, _) in blue_brains.iter().enumerate() {
        if i + 4 < engine.map.spawn_points.len() {
            let spawn = engine.map.spawn_points[i + 4].1;
            engine.tanks.push(Tank::new(
                common::game::player::PlayerInfo::new(
                    i as PlayerId,
                    format!("Blue_{}", i),
                    Team::Blue,
                ),
                spawn,
            ));
        }
    }
    // Spawn Red (Face West PI)
    for (i, _) in red_brains.iter().enumerate() {
        if i < engine.map.spawn_points.len() {
            let spawn = engine.map.spawn_points[i].1;
            let mut p = Tank::new(
                common::game::player::PlayerInfo::new(
                    (i + 4) as PlayerId,
                    format!("Red_{}", i),
                    Team::Red,
                ),
                spawn,
            );
            p.rotation = std::f32::consts::PI;
            engine.tanks.push(p);
        }
    }

    let mut stats: Vec<BotStats> = (0..8)
        .map(|i| BotStats {
            original_index: i,
            team: if i < 4 { Team::Blue } else { Team::Red },
            kills: 0,
            friendly_kills: 0,
            alive: true,
            total_score: 0.0,
        })
        .collect();

    for _tick in 0..max_ticks {
        let blue_cnt = engine
            .tanks
            .iter()
            .filter(|p| p.player_info.team == Team::Blue && p.health > 0.0)
            .count();
        let red_cnt = engine
            .tanks
            .iter()
            .filter(|p| p.player_info.team == Team::Red && p.health > 0.0)
            .count();
        if blue_cnt == 0 || red_cnt == 0 {
            break;
        }

        let mut inputs = std::collections::HashMap::new();
        let mut rng = StdRng::seed_from_u64(0);

        for (i, player) in engine.tanks.iter().enumerate() {
            if player.health <= 0.0 {
                continue;
            }

            let ctx = BotContext {
                me: player,
                players: &engine.tanks,
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
            let output = brain.forward(extract_features(&ctx, device));
            let values = output.into_data().to_vec::<f32>().unwrap();
            inputs.insert(player.player_info.id, action_to_input(&values, &ctx));
        }

        let result = engine.tick(0.033, inputs);

        for dmg in result.damage {
            let victim_team = if dmg.victim_id < 4 {
                Team::Blue
            } else {
                Team::Red
            };
            if let Some(attacker) = stats.get_mut(dmg.attacker_id as usize) {
                if attacker.team != victim_team {
                    attacker.total_score += dmg.amount;
                } else {
                    attacker.total_score -= dmg.amount * 2.0;
                    attacker.friendly_kills += 1;
                }
            }
        }

        for kill in result.kills {
            let victim_team = if kill.victim_info.id < 4 {
                Team::Blue
            } else {
                Team::Red
            };
            if let Some(killer) = stats.get_mut(kill.killer_info.id as usize) {
                if killer.team != victim_team {
                    killer.kills += 1;
                    killer.total_score += 500.0;
                } else {
                    killer.total_score -= 1000.0;
                }
            }
        }
    }

    for player in &engine.tanks {
        if player.player_info.id < 8 {
            stats[player.player_info.id as usize].alive = player.health > 0.0;
        }
    }
    stats
}

fn action_to_input(actions: &[f32], ctx: &BotContext) -> InputPayload {
    let move_fwd = actions[0].tanh();
    let move_side = actions[1].tanh();
    let aim_fwd = actions[2];
    let aim_side = actions[3];
    let shoot_val = actions[4];

    let (sin, cos) = ctx.me.rotation.sin_cos();

    let world_move = glam::Vec2::new(
        move_fwd * cos - move_side * sin,
        move_fwd * sin + move_side * cos,
    );

    let world_aim_dir = glam::Vec2::new(
        aim_fwd * cos - aim_side * sin,
        aim_fwd * sin + aim_side * cos,
    );

    let final_aim_dir = if world_aim_dir.length_squared() < 0.001 {
        glam::Vec2::new(cos, sin)
    } else {
        world_aim_dir.normalize()
    };

    let aim_pos = ctx.me.position + (final_aim_dir * 100.0);

    InputPayload {
        move_axis: world_move,
        aim_pos,
        shoot: shoot_val > 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::game::engine::GameEngine;
    use common::net::protocol::Team;
    use glam::Vec2;
    use std::fs;

    #[test]
    fn test_resolve_assets_path_logic_deeply_nested() {
        // Create a temporary directory structure for testing:
        // Root
        // ├── assets
        // └── level1
        //     └── level2 (we start here)

        let root = std::env::temp_dir().join("spartan_deep_test");
        let _ = fs::remove_dir_all(&root); // cleanup
        fs::create_dir_all(&root).unwrap();

        let root_assets = root.join("assets");
        fs::create_dir_all(&root_assets).unwrap();

        let level2 = root.join("level1").join("level2");
        fs::create_dir_all(&level2).unwrap();

        // Should find root assets from 2 levels deep
        let resolved = resolve_assets_path(&level2);

        // Canonicalize to ignore symlinks or relative path differences in comparison
        let resolved_canon = resolved.canonicalize().unwrap();
        let root_assets_canon = root_assets.canonicalize().unwrap();

        assert_eq!(
            resolved_canon, root_assets_canon,
            "Should find ../../assets"
        );

        // Cleanup
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_action_to_input_basic() {
        // Setup a dummy player facing East (0 radians)
        let mut player = Player::new(0, "TestBot".to_string(), Team::Blue, Vec2::ZERO);
        player.rotation = 0.0;

        // Create context dependencies
        let engine = GameEngine::new(MapDefinition::load());
        let mut rng = StdRng::seed_from_u64(123);

        let ctx = BotContext {
            me: &player,
            players: &engine.players,
            projectiles: &engine.projectiles,
            map: &engine.map,
            dt: 0.033,
            rng: &mut rng,
        };

        // Action: [Forward(1.0), Side(0.0), AimFwd(1.0), AimSide(0.7), Shoot(1.0)]
        let actions = vec![1.0, 0.0, 1.0, 0.0, 1.0];
        let input = action_to_input(&actions, &ctx);

        // Verify Movement (Tanh applied)
        // tanh(1.0) ~= 0.7615
        assert!(input.move_axis.x > 0.7 && input.move_axis.x < 0.8);
        assert!(input.move_axis.y.abs() < 0.001);

        // Verify Shooting
        assert!(input.shoot);
    }

    #[test]
    fn test_run_4v4_sanity() {
        // Use Wgpu as in main code
        let device = Default::default();

        let blue_brains: Vec<BotBrain<Wgpu>> = (0..4).map(|_| BotBrain::new(&device)).collect();
        let red_brains: Vec<BotBrain<Wgpu>> = (0..4).map(|_| BotBrain::new(&device)).collect();

        // Run a very short match (5 ticks) just to ensure no panics and stats are returned
        let stats = run_4v4_match(&blue_brains, &red_brains, &device, 5);

        assert_eq!(stats.len(), 8, "Should return stats for all 8 players");

        // Basic validation of stats
        for stat in &stats {
            assert!(stat.kills >= 0);
            assert!(stat.original_index < 8);
            // Teams should match index logic
            if stat.original_index < 4 {
                assert_eq!(stat.team, Team::Blue);
            } else {
                assert_eq!(stat.team, Team::Red);
            }
        }
    }
}
