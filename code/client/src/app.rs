use crate::ui::{self, Field, Text, Button};
use macroquad::prelude::{state_machine::State, *};

trait AppState {
    fn update(&mut self) -> StateAction;
    fn draw(&mut self);
    /// Decides whether this is a base view, or if it is an overlay.
    fn draw_previous(&self) -> bool {
        false
    }
    /// The state can decide what it does when it gets resumed
    fn on_resume(&mut self) { }
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
        App {
            stack: vec![Box::new(MainMenu::new())],
        }
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

#[derive(Clone, Copy)]
enum MainMenuButtons {
    Training,
    Multiplayer,
    Options,
    Quit,
}

struct MainMenu {
    button_pressed: Option<MainMenuButtons>,
}

impl MainMenu {
    fn new() -> Self {
        Self { button_pressed: None }
    }
}

impl AppState for MainMenu {
    fn draw(&mut self) {
        let x_mid = screen_width() / 2.;
        let default_text_params = TextParams {
            font_size: 30,
            ..Default::default()
        };

        Text {
            params: TextParams {
                font_size: 40,
                color: GRAY,
                ..Default::default()
            },
            ..Default::default()
        }
        .draw("NeuroBlasters", x_mid, 100.);

        let start_y = 200.;
        let button_w = 200.;
        let button_h = 50.;
        let sep = 80.;
        let mut button = Button::new(Field::default(), Some(default_text_params.clone()));

        self.button_pressed = None;

        if button
            .draw_centered(x_mid, start_y, button_w, button_h, Some("Train Models"))
            .poll() {
                self.button_pressed = Some(MainMenuButtons::Training);
            }

        if button
            .draw_centered(x_mid, start_y + sep, button_w, button_h, Some("Multiplayer"))
            .poll() {
                self.button_pressed = Some(MainMenuButtons::Multiplayer);
            }

        if button
            .draw_centered(x_mid, start_y + 2. * sep, button_w, button_h, Some("Options"))
            .poll() {
                self.button_pressed = Some(MainMenuButtons::Options);
            }


        if button
            .draw_centered(x_mid, start_y + 3. * sep, button_w, button_h, Some("Quit"))
            .poll() {
                self.button_pressed = Some(MainMenuButtons::Quit);
            }
    }

    fn update(&mut self) -> StateAction {
        match self.button_pressed {
            Some(button) => match button {
                MainMenuButtons::Training => StateAction::Push(Box::new(TrainingMenu::new())),
                MainMenuButtons::Multiplayer => StateAction::Push(Box::new(ServerConnectMenu::new())),
                MainMenuButtons::Options => StateAction::Push(Box::new(OptionsMenu::new())),
                MainMenuButtons::Quit => StateAction::Pop(1),
            }
            None => StateAction::None
        }
    }
}


struct TrainingMenu {
    back_clicked: bool,
}

impl TrainingMenu {
    fn new() -> Self {
        TrainingMenu { back_clicked: false }
    }
}

impl AppState for TrainingMenu {
    fn draw(&mut self) {
        let x_mid = screen_width() / 2.;

        Text::new_simple(30).draw("Training coming soon!", x_mid, 200.);
        self.back_clicked = Button::new(Field::default(), Some(TextParams::default()))
            .draw_centered(x_mid, 250., 250., 50., Some("Back"))
            .poll();
    }

    fn update(&mut self) -> StateAction {
        if self.back_clicked {
            StateAction::Pop(1)
        } else {
            StateAction::None
        }
    }
}

struct OptionsMenu {
    back_clicked: bool,
}

impl OptionsMenu {
    fn new() -> Self {
        OptionsMenu { back_clicked: false }
    }
}

impl AppState for OptionsMenu {
    fn draw(&mut self) {
        let x_mid = screen_width() / 2.;

        Text::new_simple(30).draw("Options here...", x_mid, 200.);
        self.back_clicked = Button::new(Field::default(), Some(TextParams::default()))
            .draw_centered(x_mid, 250., 250., 50., Some("Back"))
            .poll();
    }

    fn update(&mut self) -> StateAction {
        if self.back_clicked {
            StateAction::Pop(1)
        } else {
            StateAction::None
        }
    }
}


struct ServerConnectMenu {
    back_clicked: bool,
}

impl ServerConnectMenu {
    fn new() -> Self {
        ServerConnectMenu { back_clicked: false }
    }
}

impl AppState for ServerConnectMenu {
    fn draw(&mut self) {
        let x_mid = screen_width() / 2.;

        Text::new_simple(30).draw("Connection...", x_mid, 200.);
        self.back_clicked = Button::new(Field::default(), Some(TextParams::default()))
            .draw_centered(x_mid, 250., 250., 50., Some("Back"))
            .poll();
    }

    fn update(&mut self) -> StateAction {
        if self.back_clicked {
            StateAction::Pop(1)
        } else {
            StateAction::None
        }
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
