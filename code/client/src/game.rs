use crate::app::{AppState, StateAction};
use crate::server::Server;
use crate::ui::{self, Field};
use common::game::engine::GameEngine;
use common::protocol::{InputPayload, Team};
use macroquad::prelude::*;

pub(crate) struct Game {
    server: Server,
    game_engine: GameEngine,
}

impl Game {
    pub fn new(server: Server) -> Self {
        let map = server.get_map();
        Self {
            server,
            game_engine: GameEngine::new(map.clone()),
        }
    }
}

impl AppState for Game {
    fn draw(&mut self) {
        clear_background(LIGHTGRAY);

        // 4. Draw Map
        for wall in &self.game_engine.map.walls {
            draw_rectangle(
                wall.min.x,
                wall.min.y,
                wall.max.x - wall.min.x,
                wall.max.y - wall.min.y,
                BLACK,
            );
        }

        for player in &self.game_engine.state.players {
            draw_circle(
                player.position.x,
                player.position.y,
                player.radius,
                if player.team == Team::Blue { BLUE } else { RED },
            );

            let aim_dir = Vec2::new(player.rotation.cos(), player.rotation.sin());
            draw_line(
                player.position.x,
                player.position.y,
                player.position.x + aim_dir.x * 30.0,
                player.position.y + aim_dir.y * 30.0,
                2.0,
                RED,
            );
        }

        for projectile in &self.game_engine.state.projectiles {
            draw_circle(
                projectile.position.x,
                projectile.position.y,
                projectile.radius,
                YELLOW,
            )
        }
    }

    fn update(&mut self) -> StateAction {
        self.server.tick();
        self.game_engine.sync_state(self.server.get_tick());

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
            aim_pos: mouse_position().into(),
            shoot: is_mouse_button_down(MouseButton::Left),
        };

        self.server.send_input(input);

        if is_key_pressed(KeyCode::Escape) {
            return StateAction::Push(Box::new(InGameMenu::new()));
        }

        StateAction::None
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

impl AppState for InGameMenu {
    fn draw(&mut self) {
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

        ui::Text::new_simple(40).draw("PAUSED", x_mid, 150.);

        self.resume_clicked = ui::Button::new(Field::default(), Some(default_text_params.clone()))
            .draw_centered(x_mid, 250., 250., 50., Some("Resume"))
            .poll();

        self.quit_clicked = ui::Button::new(Field::default(), Some(default_text_params.clone()))
            .draw_centered(x_mid, 320., 250., 50., Some("Exit to Main Menu"))
            .poll();
    }

    fn update(&mut self) -> StateAction {
        if self.resume_clicked {
            return StateAction::Pop(1);
        }

        if self.quit_clicked {
            return StateAction::Pop(3);
        }

        if is_key_pressed(KeyCode::Escape) {
            return StateAction::Pop(1);
        }

        StateAction::None
    }

    fn draw_previous(&self) -> bool {
        true
    }
}
