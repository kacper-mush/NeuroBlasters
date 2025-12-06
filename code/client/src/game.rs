use crate::app::{AppState, StateAction};
use crate::server::Server;
use crate::ui::{self, Field};
use common::game_logic::apply_player_physics;
use common::protocol::{InputPayload, MapDefinition, PlayerId, PlayerState, RectWall, Team};
use macroquad::prelude::*;

pub(crate) struct Game {
    server: Server,
    player: PlayerState,
    map: MapDefinition,
}

impl Game {
    pub fn new(server: Server) -> Self {
        let player = PlayerState {
            id: PlayerId(1),
            position: (100.0, 100.0).into(),
            velocity: (0.0, 0.0).into(),
            rotation: 0.0,
            radius: 15.0,
            speed: 200.0,
            health: 100.0,
            weapon_cooldown: 0.0,
            team: Team::Blue,
        };

        let map = MapDefinition {
            width: screen_width(),
            height: screen_height(),
            walls: vec![
                RectWall {
                    min: (200.0, 200.0).into(),
                    max: (300.0, 400.0).into(),
                }, // Test Obstacle
                RectWall {
                    min: (500.0, 100.0).into(),
                    max: (700.0, 150.0).into(),
                }, // Test Obstacle
            ],
        };
        Self {
            server,
            player,
            map,
        }
    }
}

impl AppState for Game {
    fn draw(&mut self) {
        clear_background(LIGHTGRAY);

        // 4. Draw Map
        for wall in &self.map.walls {
            draw_rectangle(
                wall.min.x,
                wall.min.y,
                wall.max.x - wall.min.x,
                wall.max.y - wall.min.y,
                BLACK,
            );
        }

        // 5. Draw Player
        draw_circle(
            self.player.position.x,
            self.player.position.y,
            self.player.radius,
            BLUE,
        );

        // Draw Direction Line
        let aim_dir = Vec2::new(self.player.rotation.cos(), self.player.rotation.sin());
        draw_line(
            self.player.position.x,
            self.player.position.y,
            self.player.position.x + aim_dir.x * 30.0,
            self.player.position.y + aim_dir.y * 30.0,
            2.0,
            RED,
        );
    }

    fn update(&mut self) -> StateAction {
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

        // 3. Run Your Physics Engine
        apply_player_physics(&mut self.player, &input, &self.map, get_frame_time());

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
