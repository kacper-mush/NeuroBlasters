use common::{
    game::{InputPayload, MapDefinition, Team, engine::GameEngine},
    protocol::{ClientMessage, GameEvent, GameState, GameUpdate, InitialGameInfo},
};

use crate::{
    server::Server,
    ui::theme::{DARK_BG, GRID_COLOR, NEON_CYAN, NEON_PINK, WALL_COLOR, WALL_OUTLINE},
    ui::{
        CANONICAL_SCREEN_MID_X, Layout, TEXT_LARGE, TEXT_MID, TEXT_SMALL, Text,
        TextHorizontalPositioning, TextVerticalPositioning, calc_transform, default_text_params,
    },
};
use macroquad::prelude::*;

struct MainFeed {
    text: String,
}

impl MainFeed {
    fn new() -> Self {
        Self {
            text: String::new(),
        }
    }

    fn set(&mut self, text: String) {
        self.text = text;
    }

    fn draw(&self) {
        Text::new_scaled(TEXT_MID).draw(&self.text, CANONICAL_SCREEN_MID_X, 50.);
    }
}

struct FeedElement {
    text: String,
    active_time: f64,
    time_start: Option<f64>,
    is_active: bool,
}

impl FeedElement {
    fn new(text: String, expire_time: f64) -> Self {
        Self {
            text,
            time_start: None,
            active_time: expire_time,
            is_active: true,
        }
    }

    fn update(&mut self, time: f64) {
        match self.time_start {
            None => {
                self.time_start = Some(time);
            }
            Some(time_start) => {
                if time - time_start >= self.active_time {
                    self.is_active = false;
                }
            }
        }
    }

    fn is_active(&self) -> bool {
        self.is_active
    }
}

struct SideFeed {
    events: Vec<FeedElement>,
    display_time: f64,
    max_display: u8,
}

impl SideFeed {
    fn new(display_time: f64, max_display: u8) -> Self {
        Self {
            events: Vec::new(),
            display_time,
            max_display,
        }
    }

    fn add(&mut self, text: String) {
        self.events.push(FeedElement::new(text, self.display_time));
    }

    fn draw(&self) {
        let x = 20.;
        let mut layout = Layout::new(30., 5.);

        let text = Text::new(
            TextParams {
                font_size: TEXT_SMALL,
                ..default_text_params()
            },
            TextVerticalPositioning::CenterConsistent,
            TextHorizontalPositioning::Left,
        );

        for el in self.events.iter().take(self.max_display as usize) {
            text.draw(&el.text, x, layout.next());
            layout.add(10.);
        }
    }

    fn update(&mut self) {
        // Only time the ones that are displayed
        for el in self.events.iter_mut().take(self.max_display as usize) {
            el.update(macroquad::time::get_time());
        }

        self.events.retain(|el| el.is_active());
    }
}

pub(crate) struct Game {
    initial_game_info: InitialGameInfo,
    game_engine: GameEngine,
    pub game_state: GameState,
    is_host: bool,
    current_round: u8,
    main_feed: MainFeed,
    side_feed: SideFeed,
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
            current_round: 1,
            main_feed: MainFeed::new(),
            side_feed: SideFeed::new(5., 5),
        }
    }

    pub fn update(&mut self, game_update: GameUpdate, server: &mut Server) {
        let old_round = self.current_round;

        self.game_engine.apply_snapshot(game_update.snapshot.engine);
        self.game_state = game_update.snapshot.state;
        self.is_host = game_update.snapshot.game_master == server.get_client_id();
        self.current_round = game_update.snapshot.round_number;
        self.side_feed.update();

        for event in game_update.events {
            match event {
                GameEvent::RoundEnded(winner) => self.side_feed.add(format!(
                    "Round {} ended. Winner is {:?}!",
                    old_round, winner
                )),

                GameEvent::RoundStarted => self
                    .side_feed
                    .add(format!("Round {} has started.", self.current_round)),

                GameEvent::Kill(kill_event) => {
                    let victim = format!(
                        "{} ({:?})",
                        kill_event.victim_info.nickname, kill_event.victim_info.team
                    );
                    let killer = format!(
                        "{} ({:?})",
                        kill_event.killer_info.nickname, kill_event.killer_info.team
                    );

                    self.side_feed.add(format!("{} killed {}", killer, victim));
                }

                GameEvent::PlayerJoined(player) => {
                    self.side_feed.add(format!("{} joined the game.", player));
                }

                GameEvent::PlayerLeft(player) => {
                    self.side_feed.add(format!("{} left the game.", player));
                }
            }
        }

        let string = match self.game_state {
            GameState::Waiting => String::from("Waiting for game start"),
            GameState::Countdown(count) => {
                format!("Round {} starting in {}...", self.current_round, count)
            }
            GameState::Battle(_) => String::new(),
            GameState::Results {
                winner,
                blue_score,
                red_score,
            } => {
                format!(
                    "Team {:?} won! (Blue: {}, Red: {})",
                    winner, blue_score, red_score
                )
            }
        };
        self.main_feed.set(string);

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
        clear_background(DARK_BG);

        let map = self.game_engine.map();
        let (scaling, x_offset, y_offset) = calc_transform(map.width, map.height);
        let transform_x = |x: f32| x * scaling + x_offset;
        let transform_y = |y: f32| y * scaling + y_offset;
        let scale = |dim: f32| dim * scaling;

        // Draw Grid
        let grid_size = 50.0;
        let mut x = 0.0;
        while x <= map.width {
            let screen_x = transform_x(x);
            draw_line(
                screen_x,
                transform_y(0.0),
                screen_x,
                transform_y(map.height),
                1.0,
                GRID_COLOR,
            );
            x += grid_size;
        }
        let mut y = 0.0;
        while y <= map.height {
            let screen_y = transform_y(y);
            draw_line(
                transform_x(0.0),
                screen_y,
                transform_x(map.width),
                screen_y,
                1.0,
                GRID_COLOR,
            );
            y += grid_size;
        }

        // Draw Map Walls
        for wall in &map.walls {
            let wx = transform_x(wall.min.x);
            let wy = transform_y(wall.min.y);
            let ww = scale(wall.max.x - wall.min.x);
            let wh = scale(wall.max.y - wall.min.y);

            draw_rectangle(wx, wy, ww, wh, WALL_COLOR);
            draw_rectangle_lines(wx, wy, ww, wh, 2.0, WALL_OUTLINE);
        }

        for tank in self.game_engine.tanks() {
            let px = transform_x(tank.position.x);
            let py = transform_y(tank.position.y);
            let pr = scale(tank.radius);

            let (main_color, glow_color) = if tank.player_info.team == Team::Blue {
                (NEON_CYAN, Color::new(0.0, 1.0, 1.0, 0.2))
            } else {
                (NEON_PINK, Color::new(1.0, 0.0, 1.0, 0.2))
            };

            // Glow
            draw_circle(px, py, pr * 1.5, glow_color);
            // Main Body
            draw_circle(px, py, pr, main_color);
            // Inner Core
            draw_circle(px, py, pr * 0.5, BLACK);

            if tank.player_info.id == self.initial_game_info.player_id {
                // Outline our player
                draw_circle_lines(px, py, pr + 3.0, 2.0, WHITE);
            }

            // Direction indicator (Laser sight style)
            let aim_dir = Vec2::new(tank.rotation.cos(), tank.rotation.sin());
            draw_line(
                px,
                py,
                px + aim_dir.x * scale(40.0),
                py + aim_dir.y * scale(40.0),
                2.0,
                main_color,
            );

            // Display health bar
            let (hb_w, hb_h) = (50., 6.);

            let hb_x = transform_x(tank.position.x - hb_w / 2.);
            let hb_y = transform_y(tank.position.y - tank.radius - hb_h - 15.);

            // Health bar background
            draw_rectangle(
                hb_x,
                hb_y,
                scale(hb_w),
                scale(hb_h),
                Color::new(0.1, 0.1, 0.1, 0.8),
            );

            // Health bar fill
            let health_percentage = tank.health / 100.;
            let health_color = if health_percentage > 0.5 {
                GREEN
            } else if health_percentage > 0.25 {
                YELLOW
            } else {
                RED
            };

            draw_rectangle(
                hb_x,
                hb_y,
                scale(hb_w * health_percentage),
                scale(hb_h),
                health_color,
            );

            // Draw nick
            Text::new_simple(TEXT_SMALL, scaling).draw_no_scaling(
                &tank.player_info.nickname,
                transform_x(tank.position.x),
                transform_y(tank.position.y - tank.radius - hb_h - 35.),
            );
        }

        for projectile in self.game_engine.projectiles() {
            let px = transform_x(projectile.position.x);
            let py = transform_y(projectile.position.y);
            let pr = scale(projectile.radius);

            // Projectile Glow
            draw_circle(px, py, pr * 2.0, Color::new(1.0, 1.0, 0.0, 0.3));
            // Projectile Core
            draw_circle(px, py, pr, YELLOW);
        }

        Text::new_scaled(TEXT_SMALL).draw(&get_fps().to_string(), 10., 10.);

        if let GameState::Battle(seconds_left) = self.game_state {
            Text::new_scaled(TEXT_LARGE).draw(
                &format!("Time: {}", seconds_left),
                CANONICAL_SCREEN_MID_X,
                50.0,
            );
        }

        self.main_feed.draw();
        self.side_feed.draw();
    }

    pub fn can_user_start_game(&self) -> bool {
        self.is_host && matches!(self.game_state, GameState::Waiting)
    }

    pub fn get_game_code(&self) -> &str {
        &self.initial_game_info.game_code.0
    }

    pub fn get_current_round(&self) -> u8 {
        self.current_round
    }
}
