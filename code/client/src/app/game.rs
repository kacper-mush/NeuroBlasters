use crate::app::request_view::RequestView;
use crate::app::{AppContext, Transition, View, ViewId};
use crate::server::ClientState;
use crate::ui::{
    BUTTON_H, BUTTON_W, Button, CANONICAL_SCREEN_MID_X, Layout, TEXT_LARGE, TEXT_SMALL, Text,
    calc_transform,
};
use common::game::MapDefinition;
use common::game::engine::GameEngine;
use common::protocol::{ClientMessage, InitialGameInfo, InputPayload, Team};
use macroquad::prelude::*;

pub(crate) struct Game {
    initial_game_info: Option<InitialGameInfo>,
    game_engine: Option<GameEngine>,
}

impl Game {
    pub fn new() -> Self {
        Self {
            initial_game_info: None,
            game_engine: None,
        }
    }
}

impl View for Game {
    fn on_start(&mut self, ctx: &mut AppContext) {
        match &ctx.server.client_state {
            ClientState::Playing {
                initial_game_info,
                update: _,
            } => {
                self.game_engine = Some(GameEngine::new(MapDefinition::load_name(
                    initial_game_info.map_name,
                )));
                self.initial_game_info = Some(initial_game_info.clone());
            }
            _ => {
                // We cannot handle other states here, it will happen eventually.
            }
        }
    }

    fn draw(&mut self, _ctx: &AppContext) {
        if self.game_engine.is_none() || self.initial_game_info.is_none() {
            return;
        }
        let game_engine = self.game_engine.as_mut().unwrap();
        let game_info = self.initial_game_info.as_ref().unwrap();

        clear_background(LIGHTGRAY);

        let (scaling, x_offset, y_offset) =
            calc_transform(game_engine.map.width, game_engine.map.height);
        let transform_x = |x: f32| x * scaling + x_offset;
        let transform_y = |y: f32| y * scaling + y_offset;
        let scale = |dim: f32| dim * scaling;

        // Draw map space
        draw_rectangle(
            transform_x(0.),
            transform_y(0.),
            scale(game_engine.map.width),
            scale(game_engine.map.height),
            GRAY,
        );

        // Draw Map
        for wall in &game_engine.map.walls {
            draw_rectangle(
                transform_x(wall.min.x),
                transform_y(wall.min.y),
                scale(wall.max.x - wall.min.x),
                scale(wall.max.y - wall.min.y),
                BLACK,
            );
        }

        for player in &game_engine.players {
            draw_circle(
                transform_x(player.position.x),
                transform_y(player.position.y),
                scale(player.radius),
                if player.team == Team::Blue { BLUE } else { RED },
            );

            if player.id == game_info.player_id {
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
            Text::new_simple(TEXT_SMALL, scaling).draw_no_scaling(
                &player.nickname,
                transform_x(player.position.x),
                transform_y(player.position.y - player.radius - hb_h - 30.),
            );
        }

        for projectile in &game_engine.projectiles {
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
        match &mut ctx.server.client_state {
            ClientState::Playing {
                initial_game_info,
                update,
            } => {
                if self.game_engine.is_none() {
                    self.game_engine = Some(GameEngine::new(MapDefinition::load_name(
                        initial_game_info.map_name,
                    )));
                    self.initial_game_info = Some(initial_game_info.clone());
                }

                if let Some(update) = update.take() {
                    let game_engine = self.game_engine.as_mut().unwrap();
                    game_engine.players = update.snapshot.players;
                    game_engine.projectiles = update.snapshot.projectiles;
                }
            }
            ClientState::Error(err) => {
                return Transition::ConnectionLost(err.clone());
            }
            _ => {
                panic!("Ended up in an invalid state!");
            }
        }

        let game_engine = self.game_engine.as_ref().unwrap();

        let (scaling, x_offset, y_offset) =
            calc_transform(game_engine.map.width, game_engine.map.height);
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

        ctx.server
            .send_client_message(ClientMessage::GameInput(input));

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
        if let ClientState::Playing {
            initial_game_info: _,
            update,
        } = &mut ctx.server.client_state
            && let Some(update) = update.take()
        {
            let game_engine = self.game_engine.as_mut().unwrap();
            game_engine.players = update.snapshot.players;
            game_engine.projectiles = update.snapshot.projectiles;
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
        let x_mid = CANONICAL_SCREEN_MID_X;
        let button_w = BUTTON_W;
        let button_h = BUTTON_H;
        let mut layout = Layout::new(150., 30.);

        // Menu grays the previous view
        draw_rectangle(
            0.,
            0.,
            screen_width(),
            screen_height(),
            Color::new(0.0, 0.0, 0.0, 0.5),
        );

        Text::new_scaled(TEXT_LARGE).draw("PAUSED", x_mid, layout.next());
        layout.add(50.);

        self.resume_clicked = Button::default()
            .draw_centered(x_mid, layout.next(), button_w, button_h, Some("Resume"))
            .poll();
        layout.add(button_h);

        self.quit_clicked = Button::default()
            .draw_centered(
                x_mid,
                layout.next(),
                button_w,
                button_h,
                Some("Exit to Main Menu"),
            )
            .poll();
    }

    fn update(&mut self, ctx: &mut AppContext) -> Transition {
        match &ctx.server.client_state {
            ClientState::Playing {
                initial_game_info: _,
                update: _,
            } => {
                // This is the acceptable current state
            }
            ClientState::Error(err) => {
                return Transition::ConnectionLost(err.clone());
            }
            _ => {
                panic!("Ended up in an invalid state!");
            }
        }

        if self.resume_clicked {
            return Transition::Pop;
        }

        if self.quit_clicked {
            ctx.server.send_client_message(ClientMessage::LeaveGame);
            let success_transition = Transition::PopUntil(ViewId::ServerLobby);
            return Transition::Push(Box::new(RequestView::new_with_transition(
                "Exiting game...".into(),
                success_transition,
            )));
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
