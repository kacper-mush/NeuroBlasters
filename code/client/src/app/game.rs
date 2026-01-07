use common::{
    game::{InputPayload, MapDefinition, Team, engine::GameEngine},
    protocol::{ClientMessage, GameState, GameUpdate, InitialGameInfo},
};

use crate::{
    server::Server,
    ui::{TEXT_SMALL, Text, calc_transform},
};
use macroquad::prelude::*;

pub(crate) struct Game {
    initial_game_info: InitialGameInfo,
    game_engine: GameEngine,
    game_state: GameState,
    is_host: bool,
}

impl Game {
    pub fn new(initial_game_info: InitialGameInfo, is_host: bool) -> Self {
        let map = MapDefinition::load_name(initial_game_info.map_name);
        let game_engine = GameEngine::new(map);
        Self {
            initial_game_info,
            game_engine,
            game_state: GameState::Waiting,
            is_host,
        }
    }

    pub fn update(&mut self, game_update: GameUpdate, server: &mut Server) {
        self.game_engine.apply_snapshot(game_update.snapshot.engine);
        self.game_state = game_update.snapshot.state;
        self.is_host = game_update.snapshot.game_master == server.get_client_id();

        let map = self.game_engine.map();
        let (scaling, x_offset, y_offset) = calc_transform(map.width, map.height);
        let inv_transform_x = |x: f32| (x - x_offset) / scaling;
        let inv_transform_y = |y: f32| (y - y_offset) / scaling;

        let mouse_pos = mouse_position();
        let aim_pos = (inv_transform_x(mouse_pos.0), inv_transform_y(mouse_pos.1)).into();

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
                let len_sq = axis.0 * axis.0 + axis.1 * axis.1;
                if len_sq > 0.0 {
                    let len = len_sq.sqrt();
                    axis.0 /= len;
                    axis.1 /= len;
                }
                axis.into()
            },
            aim_pos,
            shoot: is_mouse_button_down(MouseButton::Left) || is_key_down(KeyCode::Space),
        };

        server.send_client_message(ClientMessage::GameInput(input));
    }

    pub fn draw(&self) {
        clear_background(LIGHTGRAY);

        let map = self.game_engine.map();
        let (scaling, x_offset, y_offset) = calc_transform(map.width, map.height);
        let transform_x = |x: f32| x * scaling + x_offset;
        let transform_y = |y: f32| y * scaling + y_offset;
        let scale = |dim: f32| dim * scaling;

        // Draw map space
        draw_rectangle(
            transform_x(0.),
            transform_y(0.),
            scale(map.width),
            scale(map.height),
            GRAY,
        );

        // Draw Map
        for wall in &map.walls {
            draw_rectangle(
                transform_x(wall.min.x),
                transform_y(wall.min.y),
                scale(wall.max.x - wall.min.x),
                scale(wall.max.y - wall.min.y),
                BLACK,
            );
        }

        for tank in self.game_engine.tanks() {
            draw_circle(
                transform_x(tank.position.x),
                transform_y(tank.position.y),
                scale(tank.radius),
                if tank.team == Team::Blue { BLUE } else { RED },
            );

            if tank.id == self.initial_game_info.player_id {
                // Outline our player
                draw_circle_lines(
                    transform_x(tank.position.x),
                    transform_y(tank.position.y),
                    scale(tank.radius),
                    scale(5.),
                    if tank.team == Team::Blue { RED } else { BLUE },
                );
            }

            let aim_dir = Vec2::new(tank.rotation.cos(), tank.rotation.sin());
            draw_line(
                transform_x(tank.position.x),
                transform_y(tank.position.y),
                transform_x(tank.position.x + aim_dir.x * 30.0),
                transform_y(tank.position.y + aim_dir.y * 30.0),
                scale(3.0),
                RED,
            );

            // Display health bar
            let (hb_w, hb_h) = (50., 10.);

            draw_rectangle(
                transform_x(tank.position.x - hb_w / 2.),
                transform_y(tank.position.y - tank.radius - hb_h - 10.),
                scale(hb_w),
                scale(hb_h),
                DARKGRAY,
            );

            // Hardcoded max health
            let health_percentage = tank.health / 100.;
            draw_rectangle(
                transform_x(tank.position.x - hb_w / 2.),
                transform_y(tank.position.y - tank.radius - hb_h - 10.),
                scale(hb_w * health_percentage),
                scale(hb_h),
                GREEN,
            );

            // Draw nick
            Text::new_simple(TEXT_SMALL, scaling).draw_no_scaling(
                &tank.nickname,
                transform_x(tank.position.x),
                transform_y(tank.position.y - tank.radius - hb_h - 30.),
            );
        }

        for projectile in self.game_engine.projectiles() {
            draw_circle(
                transform_x(projectile.position.x),
                transform_y(projectile.position.y),
                scale(projectile.radius),
                YELLOW,
            )
        }

        Text::new_scaled(TEXT_SMALL).draw(&get_fps().to_string(), 10., 10.);
    }

    pub fn can_user_start_game(&self) -> bool {
        self.is_host && matches!(self.game_state, GameState::Waiting)
    }

    pub fn get_game_code(&self) -> &str {
        &self.initial_game_info.game_code.0
    }
}
