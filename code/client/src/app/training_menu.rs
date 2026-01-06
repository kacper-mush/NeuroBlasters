use crate::app::{AppContext, Transition, View, ViewId};
use crate::ui;
use ::rand::SeedableRng;
use ::rand::rngs::StdRng;
use burn::backend::NdArray;
use burn::module::Module;
use burn::record::{BinFileRecorder, FullPrecisionSettings};
use common::ai::BotContext;
use common::game::engine::GameEngine;
use common::protocol::{InputPayload, MapDefinition, Player, Team};
use common::rl::{BotBrain, extract_features};
use glam::Vec2;
use macroquad::prelude::*;

type ClientBackend = NdArray;

pub(crate) struct TrainingMenu {
    game_engine: GameEngine,
    bot_brain: Option<BotBrain<ClientBackend>>,
    rng: StdRng,
}

impl TrainingMenu {
    pub fn new() -> Self {
        let mut game_engine = GameEngine::new(MapDefinition::load());

        // FIX: Clone the spawn points first to avoid borrowing game_engine while mutating it
        let spawn_points = game_engine.map.spawn_points.clone();

        // 1. Spawn ALL Bots
        for (i, (team, pos)) in spawn_points.iter().enumerate() {
            let id = i as u64;
            let name = format!("{:?} Bot {}", team, id);
            game_engine.add_player(Player::new(id, name, *team, *pos));
        }

        // 2. Load the Model
        println!("Attempting to load model from assets/model...");
        let recorder = BinFileRecorder::<FullPrecisionSettings>::default();

        let bot_brain = match BotBrain::<ClientBackend>::new(&Default::default()).load_file(
            "assets/model",
            &recorder,
            &Default::default(),
        ) {
            Ok(brain) => {
                println!("Model loaded successfully!");
                Some(brain)
            }
            Err(e) => {
                eprintln!("Failed to load model: {}", e);
                None
            }
        };

        Self {
            game_engine,
            bot_brain,
            rng: StdRng::from_os_rng(),
        }
    }

    fn calc_transform(&self) -> (f32, f32, f32) {
        let (map_w, map_h) = (self.game_engine.map.width, self.game_engine.map.height);
        let (screen_w, screen_h) = (screen_width(), screen_height());
        let x_scaling = screen_w / map_w;
        let y_scaling = screen_h / map_h;

        if x_scaling < y_scaling {
            (x_scaling, 0., f32::abs(screen_h - map_h * x_scaling) / 2.)
        } else {
            (y_scaling, f32::abs(screen_w - map_w * y_scaling) / 2., 0.)
        }
    }

    fn bot_action_to_input(actions: &[f32], ctx: &BotContext) -> InputPayload {
        let move_fwd = actions[0].tanh();
        let move_side = actions[1].tanh();
        let aim_fwd = actions[2];
        let aim_side = actions[3];
        let shoot_val = actions[4];

        let (sin, cos) = ctx.me.rotation.sin_cos();

        let world_move = Vec2::new(
            move_fwd * cos - move_side * sin,
            move_fwd * sin + move_side * cos,
        );

        let world_aim_dir = Vec2::new(
            aim_fwd * cos - aim_side * sin,
            aim_fwd * sin + aim_side * cos,
        );

        let final_aim_dir = if world_aim_dir.length_squared() < 0.001 {
            Vec2::new(cos, sin)
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
}

impl View for TrainingMenu {
    fn update(&mut self, _ctx: &mut AppContext) -> Transition {
        let dt = get_frame_time();

        let mut inputs = std::collections::HashMap::new();

        // RUN AI FOR EVERYONE
        if let Some(brain) = &self.bot_brain {
            // We need to iterate indices to avoid borrowing self.game_engine multiple times
            for player in &self.game_engine.players {
                if player.health <= 0.0 {
                    continue;
                }

                let ctx = BotContext {
                    me: player,
                    players: &self.game_engine.players,
                    projectiles: &self.game_engine.projectiles,
                    map: &self.game_engine.map,
                    dt,
                    rng: &mut self.rng,
                };

                let device = Default::default();
                let state = extract_features(&ctx, &device);

                let output = brain.forward(state);
                let values = output.into_data().to_vec::<f32>().unwrap();

                let input = Self::bot_action_to_input(&values, &ctx);
                inputs.insert(player.id, input);
            }
        }

        // Tick Game
        self.game_engine.tick(dt, &inputs);

        // Reset/Exit Logic
        if is_key_pressed(KeyCode::R) {
            *self = Self::new();
        }
        if is_key_pressed(KeyCode::Escape) {
            return Transition::Pop;
        }

        Transition::None
    }

    fn draw(&mut self, _ctx: &AppContext) {
        clear_background(LIGHTGRAY);

        let (scaling, x_offset, y_offset) = self.calc_transform();
        let transform_x = |x: f32| x * scaling + x_offset;
        let transform_y = |y: f32| y * scaling + y_offset;
        let scale = |dim: f32| dim * scaling;

        // Draw Map
        for wall in &self.game_engine.map.walls {
            draw_rectangle(
                transform_x(wall.min.x),
                transform_y(wall.min.y),
                scale(wall.max.x - wall.min.x),
                scale(wall.max.y - wall.min.y),
                BLACK,
            );
        }

        // Draw Players
        for player in &self.game_engine.players {
            let color = if player.team == Team::Blue { BLUE } else { RED };
            if player.health <= 0.0 {
                continue;
            }

            draw_circle(
                transform_x(player.position.x),
                transform_y(player.position.y),
                scale(player.radius),
                color,
            );

            // Aim line
            let aim_dir = Vec2::new(player.rotation.cos(), player.rotation.sin());
            draw_line(
                transform_x(player.position.x),
                transform_y(player.position.y),
                transform_x(player.position.x + aim_dir.x * 30.0),
                transform_y(player.position.y + aim_dir.y * 30.0),
                scale(3.0),
                DARKGRAY,
            );

            // Health Bar
            let hp_pct = player.health / 100.0;
            draw_rectangle(
                transform_x(player.position.x - 20.),
                transform_y(player.position.y - 30.),
                scale(40. * hp_pct),
                scale(5.),
                GREEN,
            );
        }

        // Draw Projectiles
        for proj in &self.game_engine.projectiles {
            draw_circle(
                transform_x(proj.position.x),
                transform_y(proj.position.y),
                scale(proj.radius),
                YELLOW,
            );
        }

        // UI Overlay
        ui::Text::new_simple(20).draw(
            "SPECTATOR MODE | Reset: R | Exit: ESC",
            screen_width() / 2.,
            30.,
        );

        let blue_count = self
            .game_engine
            .players
            .iter()
            .filter(|p| p.team == Team::Blue && p.health > 0.0)
            .count();
        let red_count = self
            .game_engine
            .players
            .iter()
            .filter(|p| p.team == Team::Red && p.health > 0.0)
            .count();
        ui::Text::new_simple(30).draw(
            &format!("Blue: {}  vs  Red: {}", blue_count, red_count),
            screen_width() / 2.,
            60.,
        );

        if self.bot_brain.is_none() {
            ui::Text::new_simple(30).draw(
                "MODEL NOT FOUND!",
                screen_width() / 2.,
                screen_height() / 2.,
            );
        } else if blue_count == 0 && red_count == 0 {
            ui::Text::new_simple(50).draw("DRAW", screen_width() / 2., screen_height() / 2.);
        } else if blue_count == 0 {
            ui::Text::new_simple(50).draw("RED WINS", screen_width() / 2., screen_height() / 2.);
        } else if red_count == 0 {
            ui::Text::new_simple(50).draw("BLUE WINS", screen_width() / 2., screen_height() / 2.);
        }
    }

    fn get_id(&self) -> ViewId {
        ViewId::TrainingMenu
    }
}
