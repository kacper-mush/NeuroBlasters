#![recursion_limit = "256"]
use burn::backend::Wgpu;
use burn::module::Module;
use burn::record::{BinFileRecorder, FullPrecisionSettings};
use burn::tensor::backend::Backend;
use common::ai::BotContext;
use common::game::engine::GameEngine;
use common::net::protocol::{InputPayload, MapDefinition, Player, Team};
use common::rl::{extract_features, BotBrain};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::sync::{Arc, Mutex};
use std::thread;

const GENERATIONS: usize = 1000;
const POPULATION_SIZE: usize = 64;
const MUTATION_RATE: f32 = 0.05;
const MAX_TICKS: usize = 1000;

type MyBackend = Wgpu;

#[tokio::main]
async fn main() {
    println!("Starting Spartan Evolution on GPU (Parallel)...");
    let device = Default::default();

    let mut population: Vec<BotBrain<MyBackend>> = (0..POPULATION_SIZE)
        .map(|_| BotBrain::new(&device))
        .collect();

    for gen in 1..=GENERATIONS {
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

                s.spawn(move || {
                    let stats = run_4v4_match(&blue_team, &red_team, &device);

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

                    // 1. Determine Winner (Alive Count > Kill Count)
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
                        // Tie-breaker: Kills
                        if blue_kills > red_kills {
                            Some(Team::Blue)
                        } else if red_kills > blue_kills {
                            Some(Team::Red)
                        } else {
                            None
                        } // True Draw
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

                    // 2. The Spartan Selection
                    // Filter: ONLY consider bots from the winning team.
                    // "No Winning - No Points"
                    let mut candidates: Vec<&BotStats> = if let Some(winner) = winning_team {
                        stats.iter().filter(|s| s.team == winner).collect()
                    } else {
                        // If Draw: Pick high performers from both sides to avoid extinction
                        // (Or return empty vec to force extinction if you want to be really strict)
                        stats.iter().collect()
                    };

                    // 3. Sort by Contribution
                    // The score represents pure contribution (Damage + Kills).
                    // We select the MVPs of the winning team.
                    candidates.sort_by(|a, b| b.total_score.partial_cmp(&a.total_score).unwrap());

                    // 4. Pick Top 2 Parents
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
        let mut new_pop = Vec::with_capacity(POPULATION_SIZE);

        for parent in &next_gen_parents {
            new_pop.push(parent.clone());
        }

        let mut rng = rand::rng();
        if next_gen_parents.is_empty() {
            println!("  ! Extinction (No Winners). Resetting population.");
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
        save_model(&population[0]);
    }
}

#[derive(Debug)]
struct BotStats {
    original_index: usize,
    team: Team,
    kills: i32,
    friendly_kills: i32,
    alive: bool,
    total_score: f32, // Represents "Contribution" (Damage + Kills)
}

fn run_4v4_match<B: Backend>(
    blue_brains: &[BotBrain<B>],
    red_brains: &[BotBrain<B>],
    device: &B::Device,
) -> Vec<BotStats> {
    let mut engine = GameEngine::new(MapDefinition::load());

    // Spawn Blue (Face East 0.0)
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
    // Spawn Red (Face West PI)
    for (i, _) in red_brains.iter().enumerate() {
        if i < engine.map.spawn_points.len() {
            let spawn = engine.map.spawn_points[i].1;
            let mut p = Player::new((i + 4) as u64, format!("Red_{}", i), Team::Red, spawn);
            p.rotation = std::f32::consts::PI;
            engine.add_player(p);
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

    for _tick in 0..MAX_TICKS {
        // Stop if team wiped
        let blue_cnt = engine
            .players
            .iter()
            .filter(|p| p.team == Team::Blue && p.health > 0.0)
            .count();
        let red_cnt = engine
            .players
            .iter()
            .filter(|p| p.team == Team::Red && p.health > 0.0)
            .count();
        if blue_cnt == 0 || red_cnt == 0 {
            break;
        }

        let mut inputs = std::collections::HashMap::new();
        let mut rng = StdRng::seed_from_u64(0);

        for (i, player) in engine.players.iter().enumerate() {
            if player.health <= 0.0 {
                continue;
            }

            // NOTE: No "Survival Reward". Merely existing contributes nothing to winning.

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
            let output = brain.forward(extract_features(&ctx, device));
            let values = output.into_data().to_vec::<f32>().unwrap();
            inputs.insert(player.id, action_to_input(&values, &ctx));
        }

        let result = engine.tick(0.033, &inputs);

        // --- CALCULATE CONTRIBUTION SCORE ---

        // 1. Damage Contribution
        for dmg in result.damage {
            let victim_team = if dmg.victim_id < 4 {
                Team::Blue
            } else {
                Team::Red
            };

            // We use direct indexing because player IDs 0..7 map to stats indices 0..7
            if let Some(attacker) = stats.get_mut(dmg.attacker_id as usize) {
                if attacker.team != victim_team {
                    // +1.0 Score per 1 Damage Dealt
                    // This is the primary differentiator for "Who carried the team?"
                    attacker.total_score += dmg.amount;
                } else {
                    // Friendly Fire Penalty (Anti-Griefing)
                    // Even if you win, if you shot your teammates, you might not be the MVP.
                    attacker.total_score -= dmg.amount * 2.0;
                    attacker.friendly_kills += 1;
                }
            }
        }

        // 2. Kill Contribution
        for kill in result.kills {
            let victim_team = if kill.victim_id < 4 {
                Team::Blue
            } else {
                Team::Red
            };

            if let Some(killer) = stats.get_mut(kill.killer_id as usize) {
                if killer.team != victim_team {
                    killer.kills += 1;
                    // Bonus for finishing the job
                    killer.total_score += 500.0;
                } else {
                    // Severe penalty for Team Killing
                    killer.total_score -= 1000.0;
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

fn save_model<B: Backend>(model: &BotBrain<B>) {
    let recorder = BinFileRecorder::<FullPrecisionSettings>::default();
    let _ = std::fs::create_dir_all("assets");
    let _ = model.clone().save_file("assets/model", &recorder);
}
