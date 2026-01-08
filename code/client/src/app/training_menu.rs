use crate::app::{AppContext, Transition, View, ViewId};
use crate::ui;
use crate::ui::theme::{DARK_BG, NEON_CYAN, NEON_PINK, WALL_COLOR, WALL_OUTLINE};
use crate::ui::{BUTTON_H, BUTTON_W, Button, CANONICAL_SCREEN_HEIGHT, CANONICAL_SCREEN_WIDTH};
use ::rand::SeedableRng;
use ::rand::rngs::StdRng;
use burn::backend::Wgpu;
use burn::module::Module;
use burn::record::{BinFileRecorder, FullPrecisionSettings};
use common::ai::BotContext;
use common::game::engine::GameEngine;
use common::net::protocol::{InputPayload, MapDefinition, PlayerId, Tank, Team};
use common::rl::{BotBrain, extract_features};
use glam::Vec2;
use macroquad::miniquad::gl::{GL_SCISSOR_TEST, glDisable, glEnable, glScissor};
use macroquad::prelude::*;
use std::fs;

type ClientBackend = Wgpu;

#[derive(Clone, Copy, PartialEq, Debug)]
enum TrainingMode {
    Spectator,
    HumanVsAi,
}

enum TrainingState {
    ModelSelect {
        files: Vec<String>,
        error_msg: Option<String>,
        scroll: f32,
    },
    ModeSelect {
        model_name: String,
        brain: BotBrain<ClientBackend>,
    },
    Playing {
        game_engine: GameEngine,
        bot_brain: BotBrain<ClientBackend>,
        mode: TrainingMode,
        human_id: Option<PlayerId>,
    },
}

pub(crate) struct TrainingMenu {
    state: TrainingState,
    rng: StdRng,
    back_clicked: bool,
}

impl TrainingMenu {
    pub fn new() -> Self {
        Self::refresh_file_list(None)
    }

    fn refresh_file_list(error_msg: Option<String>) -> Self {
        let mut files = Vec::new();
        let path = "assets/models";

        let _ = fs::create_dir_all(path);

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(ft) = entry.file_type()
                    && ft.is_file()
                    && let Some(fname) = entry.file_name().to_str()
                    && fname.ends_with(".bin")
                {
                    files.push(fname.to_string());
                }
            }
        }
        files.sort();

        Self {
            state: TrainingState::ModelSelect {
                files,
                error_msg,
                scroll: 0.0,
            },
            rng: StdRng::from_os_rng(),
            back_clicked: false,
        }
    }

    fn init_game(brain: BotBrain<ClientBackend>, mode: TrainingMode) -> TrainingState {
        let mut game_engine = GameEngine::new(MapDefinition::load());
        let spawn_points = game_engine.map.spawn_points.clone();
        let mut human_id = None;

        match mode {
            TrainingMode::Spectator => {
                for i in 0..4 {
                    if let Some((team, pos)) = spawn_points.get(i + 4) {
                        game_engine.tanks.push(Tank::new(
                            common::game::player::PlayerInfo::new(
                                i as PlayerId,
                                format!("Blue {}", i),
                                *team,
                            ),
                            *pos,
                        ));
                    }
                }
                for i in 0..4 {
                    if let Some((team, pos)) = spawn_points.get(i) {
                        game_engine.tanks.push(Tank::new(
                            common::game::player::PlayerInfo::new(
                                (i + 4) as PlayerId,
                                format!("Red {}", i),
                                *team,
                            ),
                            *pos,
                        ));
                    }
                }
            }
            TrainingMode::HumanVsAi => {
                if let Some((team, pos)) = spawn_points.get(4) {
                    let pid = 0;
                    human_id = Some(pid);
                    game_engine.tanks.push(Tank::new(
                        common::game::player::PlayerInfo::new(pid, "Player".into(), *team),
                        *pos,
                    ));
                }
                for i in 0..4 {
                    if let Some((team, pos)) = spawn_points.get(i) {
                        let pid = (i + 1) as PlayerId;
                        game_engine.tanks.push(Tank::new(
                            common::game::player::PlayerInfo::new(pid, format!("Bot {}", i), *team),
                            *pos,
                        ));
                    }
                }
            }
        }

        TrainingState::Playing {
            game_engine,
            bot_brain: brain,
            mode,
            human_id,
        }
    }

    fn calc_transform(engine: &GameEngine) -> (f32, f32, f32) {
        ui::calc_transform(engine.map.width, engine.map.height)
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

    fn draw_game(game_engine: &GameEngine) {
        clear_background(DARK_BG);
        let (scaling, x_offset, y_offset) = Self::calc_transform(game_engine);
        let transform_x = |x: f32| x * scaling + x_offset;
        let transform_y = |y: f32| y * scaling + y_offset;
        let scale = |dim: f32| dim * scaling;

        for wall in &game_engine.map.walls {
            draw_rectangle(
                transform_x(wall.min.x),
                transform_y(wall.min.y),
                scale(wall.max.x - wall.min.x),
                scale(wall.max.y - wall.min.y),
                WALL_COLOR,
            );
            draw_rectangle_lines(
                transform_x(wall.min.x),
                transform_y(wall.min.y),
                scale(wall.max.x - wall.min.x),
                scale(wall.max.y - wall.min.y),
                2.0,
                WALL_OUTLINE,
            );
        }

        for player in &game_engine.tanks {
            let (main_color, glow_color) = if player.player_info.team == Team::Blue {
                (NEON_CYAN, Color::new(0.0, 1.0, 1.0, 0.2))
            } else {
                (NEON_PINK, Color::new(1.0, 0.0, 1.0, 0.2))
            };
            if player.health <= 0.0 {
                continue;
            }
            // Glow
            draw_circle(
                transform_x(player.position.x),
                transform_y(player.position.y),
                scale(player.radius) * 1.5,
                glow_color,
            );
            // Main Body
            draw_circle(
                transform_x(player.position.x),
                transform_y(player.position.y),
                scale(player.radius),
                main_color,
            );
            // Inner Core
            draw_circle(
                transform_x(player.position.x),
                transform_y(player.position.y),
                scale(player.radius) * 0.5,
                BLACK,
            );

            let aim_dir = Vec2::new(player.rotation.cos(), player.rotation.sin());
            draw_line(
                transform_x(player.position.x),
                transform_y(player.position.y),
                transform_x(player.position.x + aim_dir.x * 30.0),
                transform_y(player.position.y + aim_dir.y * 30.0),
                scale(3.0),
                DARKGRAY,
            );
            let hp_pct = player.health / 100.0;
            draw_rectangle(
                transform_x(player.position.x - 20.),
                transform_y(player.position.y - 30.),
                scale(40. * hp_pct),
                scale(5.),
                GREEN,
            );
        }

        for proj in &game_engine.projectiles {
            draw_circle(
                transform_x(proj.position.x),
                transform_y(proj.position.y),
                scale(proj.radius),
                YELLOW,
            );
        }
    }
}

impl View for TrainingMenu {
    fn update(&mut self, _ctx: &mut AppContext) -> Transition {
        // Handle Back Button Logic
        if self.back_clicked {
            self.back_clicked = false;
            match &self.state {
                TrainingState::ModelSelect { .. } => return Transition::Pop,
                TrainingState::ModeSelect { .. } => {
                    *self = Self::refresh_file_list(None);
                    return Transition::None;
                }
                _ => {} // Playing handles back differently (ESC)
            }
        }

        match &mut self.state {
            TrainingState::ModelSelect { files, scroll, .. } => {
                let (_, y_scroll) = mouse_wheel();
                if y_scroll != 0.0 {
                    *scroll -= y_scroll * 30.0;

                    let layout_padding = 15.0;
                    let item_height = BUTTON_H + layout_padding;
                    let content_height = files.len() as f32 * item_height;
                    let view_height = CANONICAL_SCREEN_HEIGHT - 250.0;
                    let max_scroll = (content_height - view_height + 50.0).max(0.0); // +50 padding for last item visibility

                    *scroll = scroll.clamp(0.0, max_scroll);
                }
                Transition::None
            }
            TrainingState::ModeSelect { .. } => Transition::None,

            TrainingState::Playing {
                game_engine,
                bot_brain,
                human_id,
                ..
            } => {
                let dt = get_frame_time();
                let mut inputs = std::collections::HashMap::new();

                if let Some(hid) = human_id {
                    let (scaling, x_offset, y_offset) = Self::calc_transform(game_engine);
                    let inv_transform_x = |x: f32| (x - x_offset) / scaling;
                    let inv_transform_y = |y: f32| (y - y_offset) / scaling;
                    let mouse_pos = mouse_position();
                    let aim_pos =
                        (inv_transform_x(mouse_pos.0), inv_transform_y(mouse_pos.1)).into();

                    let input = InputPayload {
                        move_axis: {
                            let mut axis = (0.0f32, 0.0f32);
                            if is_key_down(KeyCode::W) {
                                axis.1 -= 1.0;
                            }
                            if is_key_down(KeyCode::S) {
                                axis.1 += 1.0;
                            }
                            if is_key_down(KeyCode::A) {
                                axis.0 -= 1.0;
                            }
                            if is_key_down(KeyCode::D) {
                                axis.0 += 1.0;
                            }
                            if axis.0 != 0. || axis.1 != 0. {
                                let len = (axis.0 * axis.0 + axis.1 * axis.1).sqrt();
                                axis.0 /= len;
                                axis.1 /= len;
                            }
                            axis.into()
                        },
                        aim_pos,
                        shoot: is_mouse_button_down(MouseButton::Left)
                            || is_key_down(KeyCode::Space),
                    };
                    inputs.insert(*hid, input);
                }

                for player in &game_engine.tanks {
                    if player.health <= 0.0 {
                        continue;
                    }
                    if Some(player.player_info.id) == *human_id {
                        continue;
                    }

                    let ctx = BotContext {
                        me: player,
                        players: &game_engine.tanks,
                        projectiles: &game_engine.projectiles,
                        map: &game_engine.map,
                        dt,
                        rng: &mut self.rng,
                    };
                    let output = bot_brain.forward(extract_features(&ctx, &Default::default()));
                    let values = output.into_data().to_vec::<f32>().unwrap();
                    inputs.insert(
                        player.player_info.id,
                        Self::bot_action_to_input(&values, &ctx),
                    );
                }

                game_engine.tick(dt, inputs);

                if is_key_pressed(KeyCode::R) {
                    return Transition::Pop;
                }
                if is_key_pressed(KeyCode::Escape) {
                    return Transition::Pop;
                }
                Transition::None
            }
        }
    }

    fn draw(&mut self, _ctx: &AppContext) {
        let x_mid = CANONICAL_SCREEN_WIDTH / 2.;
        let mut next_state: Option<TrainingState> = None;

        self.back_clicked = false;

        match &mut self.state {
            TrainingState::ModelSelect {
                files,
                error_msg,
                scroll,
            } => {
                let mut layout = ui::Layout::new(80., 15.);
                ui::Text::new_title().draw("Select Model", x_mid, layout.next());
                layout.add(60.);

                if let Some(err) = error_msg {
                    ui::Text::new_scaled(ui::TEXT_MID).draw(err, x_mid, layout.next());
                    layout.add(30.);
                }

                ui::Text::new_scaled(ui::TEXT_MID).draw("Existing Models:", x_mid, layout.next());
                layout.add(30.);

                // --- SCROLLABLE AREA START ---
                let list_start_y = layout.next();
                let back_button_y = CANONICAL_SCREEN_HEIGHT - 80.0; // Fixed position for Back button
                let list_end_y = back_button_y - 20.0;

                // Define clipping region (Scissor)
                let (scale, x_off, y_off) =
                    ui::calc_transform(CANONICAL_SCREEN_WIDTH, CANONICAL_SCREEN_HEIGHT);
                let sc_y_start = list_start_y * scale + y_off;

                // Calculate dynamic height for the view area
                let view_height_canonical = list_end_y - list_start_y;
                let sc_h = view_height_canonical * scale;

                let sc_x = 0.0 * scale + x_off; // Full width
                let sc_w = CANONICAL_SCREEN_WIDTH * scale;

                let screen_h_px = screen_height();
                let gl_y = screen_h_px - (sc_y_start + sc_h);

                unsafe {
                    get_internal_gl().flush();
                    glScissor(sc_x as i32, gl_y as i32, sc_w as i32, sc_h as i32);
                    glEnable(GL_SCISSOR_TEST);
                }

                // Draw List
                let mut list_layout = ui::Layout::new(list_start_y - *scroll, 15.);
                let mut picked_file = None;

                for file in files.iter() {
                    // Culling: Only draw if roughly in view
                    let item_y = list_layout.next();
                    // Draw if the item is at least partially visible
                    if item_y + BUTTON_H > list_start_y
                        && item_y < list_end_y
                        && Button::default()
                            .draw_centered(x_mid, item_y, BUTTON_W * 1.5, BUTTON_H, Some(file))
                            .poll()
                    {
                        picked_file = Some(file.clone());
                    }
                    list_layout.add(BUTTON_H);
                }

                unsafe {
                    get_internal_gl().flush();
                    glDisable(GL_SCISSOR_TEST);
                }
                // --- SCROLLABLE AREA END ---

                // Back Button (Fixed)
                if Button::default()
                    .draw_centered(x_mid, back_button_y, BUTTON_W, BUTTON_H, Some("Back"))
                    .poll()
                {
                    self.back_clicked = true;
                }

                if let Some(fname) = picked_file {
                    let full_path = format!("assets/models/{}", fname);
                    let load_name = if fname.ends_with(".bin") {
                        &full_path[..full_path.len() - 4]
                    } else {
                        &full_path
                    };
                    let recorder = BinFileRecorder::<FullPrecisionSettings>::default();
                    if let Ok(brain) = BotBrain::<ClientBackend>::new(&Default::default())
                        .load_file(load_name, &recorder, &Default::default())
                    {
                        next_state = Some(TrainingState::ModeSelect {
                            model_name: fname,
                            brain,
                        });
                    }
                }
            }

            TrainingState::ModeSelect { model_name, brain } => {
                let mut layout = ui::Layout::new(100., 30.);
                ui::Text::new_title().draw("Select Mode", x_mid, layout.next());
                layout.add(50.);
                ui::Text::new_scaled(ui::TEXT_MID).draw(
                    &format!("Model: {}", model_name),
                    x_mid,
                    layout.next(),
                );
                layout.add(50.);

                if Button::default()
                    .draw_centered(
                        x_mid,
                        layout.next(),
                        BUTTON_W * 1.5,
                        BUTTON_H,
                        Some("Spectator (4v4)"),
                    )
                    .poll()
                {
                    next_state = Some(Self::init_game(brain.clone(), TrainingMode::Spectator));
                }
                layout.add(BUTTON_H);

                if Button::default()
                    .draw_centered(
                        x_mid,
                        layout.next(),
                        BUTTON_W * 1.5,
                        BUTTON_H,
                        Some("Play Solo vs 4 Bots"),
                    )
                    .poll()
                {
                    next_state = Some(Self::init_game(brain.clone(), TrainingMode::HumanVsAi));
                }
                layout.add(BUTTON_H);

                layout.add(20.);
                if Button::default()
                    .draw_centered(x_mid, layout.next(), BUTTON_W, BUTTON_H, Some("Back"))
                    .poll()
                {
                    self.back_clicked = true;
                }
            }

            TrainingState::Playing {
                game_engine, mode, ..
            } => {
                Self::draw_game(game_engine);
                let mode_str = match mode {
                    TrainingMode::Spectator => "SPECTATOR",
                    TrainingMode::HumanVsAi => "PLAYING",
                };
                ui::Text::new_scaled(20).draw(
                    &format!("{} | Reset: R | Exit: ESC", mode_str),
                    screen_width() / 2.,
                    30.,
                );
            }
        }

        if let Some(ns) = next_state {
            self.state = ns;
        }
    }

    fn get_id(&self) -> ViewId {
        ViewId::TrainingMenu
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_refresh_file_list() {
        let test_filename = "test_model_12345.bin";
        let path = format!("assets/models/{}", test_filename);

        let _ = fs::create_dir_all("assets/models");
        {
            let mut file = File::create(&path).expect("Failed to create test file");
            file.write_all(b"dummy data").unwrap();
        }

        let menu = TrainingMenu::refresh_file_list(None);

        // Assert
        if let TrainingState::ModelSelect { files, .. } = menu.state {
            assert!(
                files.contains(&test_filename.to_string()),
                "File list should contain created test file"
            );
        } else {
            panic!("Menu not in ModelSelect state");
        }

        // Cleanup
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_init_game_spectator() {
        let device = Default::default();
        let brain = BotBrain::<ClientBackend>::new(&device);

        let state = TrainingMenu::init_game(brain, TrainingMode::Spectator);

        if let TrainingState::Playing {
            game_engine,
            mode,
            human_id,
            ..
        } = state
        {
            assert_eq!(mode, TrainingMode::Spectator);
            assert!(human_id.is_none());
            // 4 vs 4
            assert_eq!(game_engine.tanks.len(), 8);
        } else {
            panic!("State should be Playing");
        }
    }

    #[test]
    fn test_init_game_human_vs_ai() {
        let device = Default::default();
        let brain = BotBrain::<ClientBackend>::new(&device);

        let state = TrainingMenu::init_game(brain, TrainingMode::HumanVsAi);

        if let TrainingState::Playing {
            game_engine,
            mode,
            human_id,
            ..
        } = state
        {
            assert_eq!(mode, TrainingMode::HumanVsAi);
            assert_eq!(human_id, Some(0));
            assert_eq!(game_engine.tanks.len(), 5);
        } else {
            panic!("State should be Playing");
        }
    }
}
