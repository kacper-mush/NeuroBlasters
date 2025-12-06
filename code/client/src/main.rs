use common::ai::{BotAgent, BotDifficulty};
use common::game_logic::{
    apply_player_physics, handle_shooting, resolve_combat, update_projectiles,
};
use common::protocol::{
    GameStateSnapshot, InputPayload, MapDefinition, PlayerId, PlayerState, RectWall, Team,
};
use macroquad::prelude::*;
use std::collections::HashMap;

// --- Helper: Create Map Locally for Testing ---
fn create_demo_map() -> MapDefinition {
    let width = 1600.0;
    let height = 900.0;
    let mut walls = Vec::new();

    // Central Horizontal Walls
    walls.push(RectWall {
        min: Vec2::new(400.0, 300.0),
        max: Vec2::new(1200.0, 320.0),
    });
    walls.push(RectWall {
        min: Vec2::new(400.0, 580.0),
        max: Vec2::new(1200.0, 600.0),
    });

    // Diagonal-like Barriers (Approximated with Rects)
    walls.push(RectWall {
        min: Vec2::new(200.0, 200.0),
        max: Vec2::new(220.0, 400.0),
    });
    walls.push(RectWall {
        min: Vec2::new(1380.0, 500.0),
        max: Vec2::new(1400.0, 700.0),
    });

    MapDefinition {
        width,
        height,
        walls,
    }
}

#[macroquad::main("Neuroblasters")]
async fn main() {
    // --- 1. SETUP WORLD ---
    let mut players = Vec::new();
    let mut projectiles = Vec::new();

    // Call the local map creation function
    let map = create_demo_map();

    let mut bot_agents = HashMap::new();
    let mut projectile_id_counter = 0;

    // Spawn Local Player (Blue)
    let p1_id = PlayerId(1);
    players.push(PlayerState {
        id: p1_id,
        team: Team::Blue,
        position: Vec2::new(150.0, 300.0),
        velocity: Vec2::ZERO,
        rotation: 0.0,
        radius: 15.0,
        speed: 250.0,
        health: 100.0,
        weapon_cooldown: 0.0,
    });

    // Spawn Wanderer Bot (Red)
    let p2_id = PlayerId(2);
    players.push(PlayerState {
        id: p2_id,
        team: Team::Red,
        position: Vec2::new(1450.0, 800.0),
        velocity: Vec2::ZERO,
        rotation: 0.0,
        radius: 15.0,
        speed: 250.0,
        health: 100.0,
        weapon_cooldown: 0.0,
    });
    let p3_id = PlayerId(3);
    players.push(PlayerState {
        id: p3_id,
        team: Team::Red,
        position: Vec2::new(200.0, 300.0),
        velocity: Vec2::ZERO,
        rotation: 0.0,
        radius: 15.0,
        speed: 0.0,
        health: 100.0,
        weapon_cooldown: 0.0,
    });
    bot_agents.insert(p2_id, BotAgent::new(BotDifficulty::Terminator, 666));
    bot_agents.insert(p3_id, BotAgent::new(BotDifficulty::Dummy, 666));
    // --- 2. GAME LOOP ---
    loop {
        let dt = get_frame_time();

        // --- A. GATHER INPUTS ---
        let mut inputs = HashMap::new();

        // Local Input
        let mut move_axis = Vec2::ZERO;
        if is_key_down(KeyCode::W) {
            move_axis.y -= 1.0;
        }
        if is_key_down(KeyCode::S) {
            move_axis.y += 1.0;
        }
        if is_key_down(KeyCode::A) {
            move_axis.x -= 1.0;
        }
        if is_key_down(KeyCode::D) {
            move_axis.x += 1.0;
        }

        let (mouse_x, mouse_y) = mouse_position();
        let aim_pos = Vec2::new(mouse_x, mouse_y);

        inputs.insert(
            p1_id,
            InputPayload {
                move_axis,
                aim_pos,
                shoot: is_mouse_button_down(MouseButton::Left),
            },
        );

        // Bot Inputs
        let snapshot = GameStateSnapshot {
            players: players.clone(),
            projectiles: projectiles.clone(),
            time_remaining: 0.0,
        };

        for (bot_id, agent) in &mut bot_agents {
            if let Some(bot_state) = players.iter().find(|p| p.id == *bot_id) {
                let input = agent.generate_input(bot_state, &snapshot, &map, dt);
                inputs.insert(*bot_id, input);
            }
        }

        // --- B. PHYSICS ---
        for player in &mut players {
            let default_input = InputPayload {
                move_axis: Vec2::ZERO,
                aim_pos: player.position,
                shoot: false,
            };
            let input = inputs.get(&player.id).unwrap_or(&default_input);

            apply_player_physics(player, input, &map, dt);

            if let Some(proj) = handle_shooting(player, input, dt, projectile_id_counter) {
                projectiles.push(proj);
                projectile_id_counter += 1;
            }
        }

        update_projectiles(&mut projectiles, &map, dt);
        resolve_combat(&mut players, &mut projectiles);

        // --- C. RENDER ---
        clear_background(LIGHTGRAY);

        // Draw Map Walls
        for wall in &map.walls {
            draw_rectangle(
                wall.min.x,
                wall.min.y,
                wall.max.x - wall.min.x,
                wall.max.y - wall.min.y,
                BLACK,
            );
        }

        // Draw Projectiles
        for p in &projectiles {
            draw_circle(p.position.x, p.position.y, p.radius, GOLD);
        }

        // Draw Players
        for p in &players {
            let color = if p.team == Team::Blue { BLUE } else { RED };
            draw_circle(p.position.x, p.position.y, p.radius, color);

            // Health bar
            let hp_pct = p.health / 100.0;
            draw_rectangle(p.position.x - 20.0, p.position.y - 25.0, 40.0, 5.0, RED);
            draw_rectangle(
                p.position.x - 20.0,
                p.position.y - 25.0,
                40.0 * hp_pct,
                5.0,
                GREEN,
            );

            // Bot Label
            if bot_agents.contains_key(&p.id) {
                draw_text(
                    "BOT",
                    p.position.x - 10.0,
                    p.position.y - 30.0,
                    20.0,
                    DARKGRAY,
                );
            }
        }

        // UI Overlay
        draw_text("MINIMAL BOT TEST", 20.0, 30.0, 30.0, DARKGRAY);
        draw_text(
            "Controls: WASD to Move, Click to Shoot",
            20.0,
            60.0,
            20.0,
            DARKGRAY,
        );
        draw_text("P2: Wanderer (Random Walk & Shoot)", 20.0, 90.0, 20.0, RED);

        next_frame().await;
    }
}
