use crate::app::winner_screen::WinnerScreen;
use crate::app::{AppContext, Transition, View, ViewId};
use crate::server::ClientState;
use crate::ui::{Button, Field, TEXT_LARGE, TEXT_SMALL, Text};
use common::game::engine::GameEngine;
use common::protocol::{ClientMessage, InputPayload, Team};
use macroquad::miniquad::window::screen_size;
use macroquad::prelude::*;

pub(crate) struct Game {
    game_engine: GameEngine,
}

impl Game {
    pub fn new(game_engine: GameEngine) -> Self {
        Self { game_engine }
    }

    fn calc_transform(&mut self) -> (f32, f32, f32) {
        let (map_w, map_h) = (self.game_engine.map.width, self.game_engine.map.height);
        let (screen_w, screen_h) = screen_size();
        let x_scaling = screen_w / map_w;
        let y_scaling = screen_h / map_h;
        let x_offset;
        let y_offset;
        let scaling;

        // Choose scaling and offsets so that the map perfectly fits 1 dimension
        // and is centered on the second dimension
        if x_scaling < y_scaling {
            scaling = x_scaling;
            x_offset = 0.;
            y_offset = f32::abs(screen_h - map_h * scaling) / 2.;
        } else {
            scaling = y_scaling;
            x_offset = f32::abs(screen_w - map_w * scaling) / 2.;
            y_offset = 0.;
        }

        (scaling, x_offset, y_offset)
    }
}

impl View for Game {
    fn draw(&mut self, ctx: &AppContext) {
        clear_background(LIGHTGRAY);

        let (scaling, x_offset, y_offset) = self.calc_transform();
        let transform_x = |x: f32| x * scaling + x_offset;
        let transform_y = |y: f32| y * scaling + y_offset;
        let scale = |dim: f32| dim * scaling;

        // Draw map space
        draw_rectangle(
            transform_x(0.),
            transform_y(0.),
            scale(self.game_engine.map.width),
            scale(self.game_engine.map.height),
            GRAY,
        );

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

        for player in &self.game_engine.players {
            draw_circle(
                transform_x(player.position.x),
                transform_y(player.position.y),
                scale(player.radius),
                if player.team == Team::Blue { BLUE } else { RED },
            );

            if ctx.server.as_ref().is_some_and(|s| s.get_id() == player.id) {
                // Outline our player
                draw_circle_lines(
                    transform_x(player.position.x),
                    transform_y(player.position.y),
                    scale(player.radius),
                    scale(5.),
                    if player.team == Team::Blue { RED } else { BLUE },
                );
            }

            let aim_dir = Vec2::new(player.rotation.cos(), player.rotation.sin());
            draw_line(
                transform_x(player.position.x),
                transform_y(player.position.y),
                transform_x(player.position.x + aim_dir.x * 30.0),
                transform_y(player.position.y + aim_dir.y * 30.0),
                scale(3.0),
                RED,
            );

            // Display health bar
            let (hb_w, hb_h) = (50., 10.);

            draw_rectangle(
                transform_x(player.position.x - hb_w / 2.),
                transform_y(player.position.y - player.radius - hb_h - 10.),
                scale(hb_w),
                scale(hb_h),
                DARKGRAY,
            );

            // Hardcoded max health
            let health_percentage = player.health / 100.;
            draw_rectangle(
                transform_x(player.position.x - hb_w / 2.),
                transform_y(player.position.y - player.radius - hb_h - 10.),
                scale(hb_w * health_percentage),
                scale(hb_h),
                GREEN,
            );

            // Draw nick
            Text::new_simple(TEXT_SMALL, scaling).draw(
                &player.nickname,
                transform_x(player.position.x),
                transform_y(player.position.y - player.radius - hb_h - 30.),
            );
        }

        for projectile in &self.game_engine.projectiles {
            draw_circle(
                transform_x(projectile.position.x),
                transform_y(projectile.position.y),
                scale(projectile.radius),
                YELLOW,
            )
        }

        Text::new_scaled(TEXT_SMALL).draw(&get_fps().to_string(), 10., 10.);
    }

    fn update(&mut self, ctx: &mut AppContext) -> Transition {
        if ctx.server.is_none() {
            return Transition::ConnectionLost;
        }
        let server = ctx.server.as_mut().unwrap();

        match &server.client_state {
            ClientState::AfterGame { winner } => {
                // TODO: handle winner display
                println!("Winner is: {:?}", winner);
                return Transition::PopUntilAnd(
                    ViewId::RoomMenu,
                    Box::new(WinnerScreen::new(*winner)),
                );
            }
            ClientState::Playing { game_engine: _ } => {
                // This is the acceptable current state
            }
            ClientState::Error => {
                return Transition::ConnectionLost;
            }
            _ => {
                panic!("Ended up in an invalid state!");
            }
        }

        if let Some(engine) = server.get_fresh_game() {
            self.game_engine = engine;
        }

        let (scaling, x_offset, y_offset) = self.calc_transform();
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

        let res = server.send_client_message(ClientMessage::GameInput(input));
        if res.is_err() {
            eprintln!("Could not send input!");
            return Transition::Pop;
        }

        if is_key_pressed(KeyCode::Escape) {
            return Transition::Push(Box::new(InGameMenu::new()));
        }

        Transition::None
    }

    fn get_id(&self) -> ViewId {
        ViewId::Game
    }

    fn shadow_update(&mut self, ctx: &mut AppContext) {
        // If the server is present, we update game state so that the game doesn't
        // freeze even if it is overlayed.
        // If the server is not present, that's fine, because the app frame above us
        // should handle that, or we will when we come back to focus.
        if let Some(server) = ctx.server.as_mut()
            && let Some(engine) = server.get_fresh_game()
        {
            self.game_engine = engine;
        }
    }
}

struct InGameMenu {
    resume_clicked: bool,
    quit_clicked: bool,
}

impl InGameMenu {
    fn new() -> Self {
        InGameMenu {
            resume_clicked: false,
            quit_clicked: false,
        }
    }
}

impl View for InGameMenu {
    fn draw(&mut self, _ctx: &AppContext) {
        let x_mid = screen_width() / 2.;
        let default_text_params = TextParams {
            font_size: 30,
            ..Default::default()
        };

        // Menu grays the previous view
        draw_rectangle(
            0.,
            0.,
            screen_width(),
            screen_height(),
            Color::new(0.0, 0.0, 0.0, 0.5),
        );

        Text::new_scaled(TEXT_LARGE).draw("PAUSED", x_mid, 150.);

        self.resume_clicked = Button::new(Field::default(), Some(default_text_params.clone()))
            .draw_centered(x_mid, 250., 250., 50., Some("Resume"))
            .poll();

        self.quit_clicked = Button::new(Field::default(), Some(default_text_params.clone()))
            .draw_centered(x_mid, 320., 250., 50., Some("Exit to Main Menu"))
            .poll();
    }

    fn update(&mut self, ctx: &mut AppContext) -> Transition {
        if ctx.server.is_none() {
            return Transition::ConnectionLost;
        }
        let server = ctx.server.as_mut().unwrap();

        match &server.client_state {
            ClientState::AfterGame { winner } => {
                // TODO: handle winner display
                println!("Winner is: {:?}", winner);
                return Transition::PopUntilAnd(
                    ViewId::RoomMenu,
                    Box::new(WinnerScreen::new(*winner)),
                );
            }
            ClientState::Playing { game_engine: _ } => {
                // This is the acceptable current state
            }
            ClientState::Error => {
                return Transition::ConnectionLost;
            }
            _ => {
                panic!("Ended up in an invalid state!");
            }
        }

        if self.resume_clicked {
            return Transition::Pop;
        }

        if self.quit_clicked {
            return Transition::PopUntil(ViewId::RoomMenu);
        }

        if is_key_pressed(KeyCode::Escape) {
            return Transition::Pop;
        }

        Transition::None
    }

    fn get_id(&self) -> ViewId {
        ViewId::InGameMenu
    }

    fn is_overlay(&self) -> bool {
        true
    }
}
