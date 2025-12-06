use crate::app::{AppState, StateAction};
use crate::server::Server;
use crate::ui::{self, Button, Field, Text, TextField};
use macroquad::prelude::*;

pub(crate) struct Game {
    server: Server,
}

impl Game {
    pub fn new(server: Server) -> Self {
        Self { server }
    }
}

impl AppState for Game {
    fn draw(&mut self) {
        // Just a blue rectangle in the center of the screen
        let rect_size = 100.0;
        draw_rectangle(
            screen_width() / 2.0 - rect_size / 2.0,
            screen_height() / 2.0 - rect_size / 2.0,
            rect_size,
            rect_size,
            BLUE,
        );

        draw_text(
            "Game is running. Press ESC for menu.",
            20.0,
            30.0,
            20.0,
            DARKGRAY,
        );
    }

    fn update(&mut self) -> StateAction {
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
