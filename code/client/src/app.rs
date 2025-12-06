use crate::game::Game;
use crate::server::Server;
use crate::ui::{self, Button, Field, Text, TextField};
use macroquad::prelude::*;

pub(crate) trait AppState {
    fn update(&mut self) -> StateAction;
    fn draw(&mut self);
    /// Decides whether this is a base view, or if it is an overlay.
    fn draw_previous(&self) -> bool {
        false
    }
    /// The state can decide what it does when it gets resumed
    fn on_resume(&mut self) {}
}

pub(crate) enum StateAction {
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
        Self {
            button_pressed: None,
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
            .poll()
        {
            self.button_pressed = Some(MainMenuButtons::Training);
        }

        if button
            .draw_centered(
                x_mid,
                start_y + sep,
                button_w,
                button_h,
                Some("Multiplayer"),
            )
            .poll()
        {
            self.button_pressed = Some(MainMenuButtons::Multiplayer);
        }

        if button
            .draw_centered(
                x_mid,
                start_y + 2. * sep,
                button_w,
                button_h,
                Some("Options"),
            )
            .poll()
        {
            self.button_pressed = Some(MainMenuButtons::Options);
        }

        if button
            .draw_centered(x_mid, start_y + 3. * sep, button_w, button_h, Some("Quit"))
            .poll()
        {
            self.button_pressed = Some(MainMenuButtons::Quit);
        }
    }

    fn update(&mut self) -> StateAction {
        match self.button_pressed {
            Some(button) => match button {
                MainMenuButtons::Training => StateAction::Push(Box::new(TrainingMenu::new())),
                MainMenuButtons::Multiplayer => {
                    StateAction::Push(Box::new(ServerConnectMenu::new()))
                }
                MainMenuButtons::Options => StateAction::Push(Box::new(OptionsMenu::new())),
                MainMenuButtons::Quit => StateAction::Pop(1),
            },
            None => StateAction::None,
        }
    }
}

struct TrainingMenu {
    back_clicked: bool,
}

impl TrainingMenu {
    fn new() -> Self {
        TrainingMenu {
            back_clicked: false,
        }
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
        OptionsMenu {
            back_clicked: false,
        }
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

#[derive(Copy, Clone)]
enum ServerConnectButtons {
    Connect,
    Back,
}

struct ServerConnectMenu {
    button_pressed: Option<ServerConnectButtons>,
    message: Option<String>,
    servername_field: TextField,
}

impl ServerConnectMenu {
    fn new() -> Self {
        ServerConnectMenu {
            button_pressed: None,
            message: None,
            servername_field: TextField::new(Field::default(), TextParams::default(), 30),
        }
    }
}

impl AppState for ServerConnectMenu {
    fn draw(&mut self) {
        let x_mid = screen_width() / 2.;
        let mut button = Button::new(Field::default(), Some(TextParams::default()));
        let w = 300.;
        let h = 50.;
        let y_start = 270.;
        let sep = 80.;

        Text::new_simple(30).draw("Connect to server", x_mid, 200.);

        let default_message = "Enter server name:";

        let message = self.message.as_deref().unwrap_or(default_message);
        Text::new_simple(20).draw(message, x_mid, 230.);

        self.servername_field.draw_centered(x_mid, y_start, w, h);

        self.button_pressed = None;

        if button
            .draw_centered(x_mid, y_start + sep, w, h, Some("Connect"))
            .poll()
        {
            self.button_pressed = Some(ServerConnectButtons::Connect);
        }

        if button
            .draw_centered(x_mid, y_start + 2. * sep, w, h, Some("Back"))
            .poll()
        {
            self.button_pressed = Some(ServerConnectButtons::Back);
        }
    }

    fn update(&mut self) -> StateAction {
        self.servername_field.update();

        match self.button_pressed {
            Some(button) => match button {
                ServerConnectButtons::Connect => {
                    let server = Server::new();
                    if server.connect(self.servername_field.text()) {
                        return StateAction::Push(Box::new(RoomMenu::new(server)));
                    }

                    self.message = Some("Could not connect to the server!".into());
                    StateAction::None
                }
                ServerConnectButtons::Back => StateAction::Pop(1),
            },
            None => StateAction::None,
        }
    }

    fn on_resume(&mut self) {
        *self = Self::new()
    }
}

#[derive(Clone, Copy)]
enum RoomMenuButtons {
    Create,
    Join,
    Back,
}

struct RoomMenu {
    button_pressed: Option<RoomMenuButtons>,
    room_code_field: TextField,
    server: Server,
    message: Option<String>,
}

impl RoomMenu {
    fn new(server: Server) -> Self {
        RoomMenu {
            button_pressed: None,
            room_code_field: TextField::new(Field::default(), TextParams::default(), 10),
            server,
            message: None,
        }
    }
}

impl AppState for RoomMenu {
    fn draw(&mut self) {
        let x_mid = screen_width() / 2.;
        let mut button = Button::new(Field::default(), Some(TextParams::default()));
        let w = 300.;
        let h = 50.;
        let y_start = 270.;
        let sep = 80.;

        self.button_pressed = None;

        Text::new_simple(30).draw("Rooms", x_mid, 200.);
        if button
            .draw_centered(x_mid, y_start, w, h, Some("Create"))
            .poll()
        {
            self.button_pressed = Some(RoomMenuButtons::Create);
        }

        Text::new_simple(30).draw("Room code:", x_mid, y_start + sep);

        self.room_code_field
            .draw_centered(x_mid, y_start + 2. * sep, w, h);

        if button
            .draw_centered(x_mid, y_start + 3. * sep, w, h, Some("Join"))
            .poll()
        {
            self.button_pressed = Some(RoomMenuButtons::Join);
        }

        if button
            .draw_centered(x_mid, y_start + 4. * sep, w, h, Some("Back"))
            .poll()
        {
            self.button_pressed = Some(RoomMenuButtons::Back);
        }

        if let Some(message) = self.message.clone() {
            Text::new_simple(30).draw(&message, x_mid, y_start + 5. * sep);
        }
    }

    fn update(&mut self) -> StateAction {
        self.room_code_field.update();

        match self.button_pressed {
            Some(button) => match button {
                RoomMenuButtons::Create => {
                    if self.server.create_room() {
                        StateAction::Push(Box::new(RoomView::new(self.server.clone())))
                    } else {
                        self.message = Some("Could not create the room!".into());
                        StateAction::None
                    }
                }
                RoomMenuButtons::Join => {
                    let room_code = self.room_code_field.text().parse::<u32>();
                    if room_code.is_err() {
                        self.message = Some("Invalid room code!".into());
                        return StateAction::None;
                    }

                    if self.server.join_room(room_code.unwrap()) {
                        return StateAction::Push(Box::new(RoomView::new(self.server.clone())));
                    }

                    self.message = Some("Could not join the room!".into());
                    StateAction::None
                }
                RoomMenuButtons::Back => StateAction::Pop(1),
            },
            None => StateAction::None,
        }
    }

    fn on_resume(&mut self) {
        *self = Self::new(self.server.clone())
    }
}

#[derive(Clone, Copy)]
enum RoomViewButtons {
    Start,
    Leave,
}

struct RoomView {
    button_pressed: Option<RoomViewButtons>,
    server: Server,
    room_code: u32,
    player_names: Vec<String>,
}

impl RoomView {
    fn new(server: Server) -> Self {
        Self {
            button_pressed: None,
            server,
            room_code: 0,
            player_names: Vec::new(),
        }
    }
}

impl AppState for RoomView {
    fn draw(&mut self) {
        let x_mid = screen_width() / 2.;
        let mut button = Button::new(Field::default(), Some(TextParams::default()));
        let w = 300.;
        let h = 50.;
        let y_start = 200.;
        let sep = 40.;
        let mut offset = 0.;

        let title = format!("Room: {}", self.room_code);
        Text::new_simple(40).draw(&title, x_mid, y_start + offset);
        offset += sep;
        Text::new_simple(30).draw("Players:", x_mid, y_start + offset);
        offset += sep;
        for name in &self.player_names {
            Text::new_simple(25).draw(name, x_mid, y_start + offset);
            offset += sep;
        }

        self.button_pressed = None;

        if button
            .draw_centered(x_mid, y_start + offset, w, h, Some("Start"))
            .poll()
        {
            self.button_pressed = Some(RoomViewButtons::Start);
        }
        offset += 100.;
        if button
            .draw_centered(x_mid, y_start + offset, w, h, Some("Back"))
            .poll()
        {
            self.button_pressed = Some(RoomViewButtons::Leave);
        }
    }

    fn update(&mut self) -> StateAction {
        self.room_code = match self.server.get_room_code() {
            Ok(code) => code,
            Err(e) => return StateAction::Pop(1),
        };

        self.player_names = match self.server.get_player_list() {
            Ok(player_list) => player_list,
            Err(e) => return StateAction::Pop(1),
        };

        match self.button_pressed {
            Some(button) => match button {
                RoomViewButtons::Leave => {
                    self.server.leave();
                    StateAction::Pop(1)
                }
                RoomViewButtons::Start => {
                    if self.server.start_game() {
                        return StateAction::Push(Box::new(Game::new(self.server.clone())));
                    }

                    StateAction::None
                }
            },
            None => StateAction::None,
        }
    }
}
