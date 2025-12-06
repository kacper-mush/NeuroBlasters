use crate::ui::{self, TextParamsExtended, extended_draw_text};
use macroquad::prelude::*;

trait AppState {
    fn update(&mut self) -> StateAction;
    fn draw(&mut self);
    /// Decides whether this is a base view, or if it is an overlay.
    fn draw_previous(&self) -> bool {
        false
    }
    /// The state can decide what it does when it gets resumed
    fn on_resume(&mut self) {}
}

enum StateAction {
    None,
    /// Add a new state on top (another menu, etc.)
    Push(Box<dyn AppState>),
    /// Remove a number of states from top (like collapse the whole menu)
    Pop(u16),
}

pub(crate) struct App {
    stack: Vec<Box<dyn AppState>>,
}

impl App {
    pub fn new() -> Self {
        App { stack: vec![Box::new(MainMenu::new())] }
    }

    pub async fn run(&mut self) {
        while let Some(state) = self.stack.last_mut() {
            // We only run update for the State on top of the stack
            let action = state.update();

            clear_background(DARKBLUE);

            // Find the first state that is not letting states beneath it be drawn.
            // Start from the top of the stack
            let mut start_index = 0;
            for i in (0..self.stack.len()).rev() {
                if !self.stack[i].draw_previous() {
                    start_index = i;
                    break;
                }
            }

            // B. Draw from the floor up to the top
            for i in start_index..self.stack.len() {
                self.stack[i].draw();
            }

            match action {
                StateAction::Push(new_state) => self.stack.push(new_state),
                StateAction::Pop(n) => {
                    let to_keep = self.stack.len() - n as usize;
                    self.stack.truncate(to_keep);

                    // Tell the state that it came back into focus
                    if let Some(state) = self.stack.last_mut() {
                        state.on_resume();
                    }
                }
                StateAction::None => {}
            }

            next_frame().await;
        }
    }
}

struct MainMenu {
    input_field: ui::TextField,
    play_pressed: bool,
    quit_pressed: bool,
}

impl MainMenu {
    fn new() -> Self {
        let text_params = TextParams {
            font_size: 30,
            ..Default::default()
        };
        Self {
            input_field: ui::TextField::new_centered(
                screen_width() / 2.,
                400.,
                250.,
                50.,
                Default::default(),
                text_params,
                16,
            ),
            play_pressed: false,
            quit_pressed: false,
        }
    }
}

impl AppState for MainMenu {
    fn draw(&mut self) {
        let x_mid = screen_width() / 2.;
        let default_text_params = TextParams {
            font_size: 30,
            ..Default::default()
        };

        let params = TextParamsExtended {
            base: TextParams {
                font_size: 40,
                color: GRAY,
                ..Default::default()
            },
            vertical_positioning: ui::TextVerticalPositioning::CenterExact,
            horizontal_positioning: ui::TextHorizontalPositioning::Center,
        };
        extended_draw_text("MAIN MENU", x_mid, 100., params);

        let play_button = ui::Button::new_centered(
            x_mid,
            200.0,
            200.0,
            50.0,
            Default::default(),
            Some(default_text_params.clone()),
            Some("Play game".into()),
        );
        play_button.draw();
        self.play_pressed = play_button.lm_clicked();

        let quit_button = ui::Button::new_centered(
            x_mid,
            270.0,
            200.0,
            50.0,
            Default::default(),
            Some(default_text_params.clone()),
            Some("Quit".into()),
        );
        quit_button.draw();
        self.quit_pressed = quit_button.lm_clicked();

        self.input_field.draw();
    }

    fn update(&mut self) -> StateAction {
        self.input_field.update();

        if self.play_pressed {
            return StateAction::Push(Box::new(Game));
        }

        if self.quit_pressed {
            return StateAction::Pop(1);
        }

        StateAction::None
    }

    fn on_resume(&mut self) {
        *self = Self::new()
    }
}

struct Game;

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

        ui::draw_text_simple_center("PAUSED", x_mid, 150., 40);

        let resume_button = ui::Button::new_centered(
            x_mid,
            250.0,
            250.0,
            50.0,
            Default::default(),
            Some(default_text_params.clone()),
            Some("Resume".into()),
        );
        resume_button.draw();
        self.resume_clicked = resume_button.lm_clicked();

        let quit_button = ui::Button::new_centered(
            x_mid,
            320.0,
            250.0,
            50.0,
            Default::default(),
            Some(default_text_params.clone()),
            Some("Exit to Main Menu".into()),
        );
        quit_button.draw();
        self.quit_clicked = quit_button.lm_clicked();
    }

    fn update(&mut self) -> StateAction {
        if self.resume_clicked {
            return StateAction::Pop(1);
        }

        if self.quit_clicked {
            return StateAction::Pop(2);
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
