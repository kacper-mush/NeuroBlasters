use crate::game::Game;
use crate::server::Server;
use crate::ui::{Button, Field, Text, TextField};
use macroquad::prelude::*;

// Global data that persists across views
pub(crate) struct AppContext {
    pub server: Option<Server>,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub(crate) enum ViewId {
    MainMenu,
    TrainingMenu,
    ServerConnectMenu,
    RoomMenu,
    RoomLobby,
    Game,
    InGameMenu,
    OptionsMenu,
}

pub(crate) enum Transition {
    None,
    /// Push a new state onto the stack
    Push(Box<dyn View>),
    /// Pop the top state
    Pop,
    /// Pop states until we find the specific ID (e.g., "Back to Main Menu")
    PopUntil(ViewId),
    /// A state that was reliant on a server connection lost it
    ConnectionLost,
}

pub(crate) trait View {
    fn get_id(&self) -> ViewId;

    /// Function for the main update doing all the work
    fn update(&mut self, ctx: &mut AppContext) -> Transition;

    /// Update that is called for View not on top of the app stack,
    /// no input handling is allowed
    fn shadow_update(&mut self, _ctx: &mut AppContext) {}

    fn draw(&mut self, ctx: &AppContext);

    // -- Lifecycle Hooks --

    /// Called when this state becomes the top of the stack
    fn on_start(&mut self, _ctx: &mut AppContext) {}

    /// Called when a new state is pushed on TOP of this one.
    fn on_pause(&mut self, _ctx: &mut AppContext) {}

    /// Called when some amount of states above this one were popped.
    /// `from_overlay` tells us if the thing that just closed was a popup or a full view.
    fn on_resume(&mut self, _ctx: &mut AppContext, _from_overlay: bool) {
        // Default behavior: Do nothing (preserve text fields).
        // If we want to reset, we override this method.
    }

    /// Helper to determine if we should draw the state below this one.
    fn is_overlay(&self) -> bool {
        false
    }
}

pub(crate) struct App {
    stack: Vec<Box<dyn View>>,
    context: AppContext,
}

impl App {
    pub fn new() -> Self {
        App {
            stack: vec![Box::new(MainMenu::new())],
            context: AppContext { server: None },
        }
    }

    pub async fn run(&mut self) {
        while !self.stack.is_empty() {
            // We only run update for the state on top of the stack
            let mut transition = self.stack.last_mut().unwrap().update(&mut self.context);

            // Shadow update all the remaining states
            for i in 0..self.stack.len() - 1 {
                self.stack[i].shadow_update(&mut self.context);
            }

            clear_background(DARKBLUE);

            // Find the first state that is not letting states beneath it be drawn.
            // Start from the top of the stack
            let mut start_index = 0;
            for i in (0..self.stack.len()).rev() {
                if !self.stack[i].is_overlay() {
                    start_index = i;
                    break;
                }
            }

            // Draw from the floor up to the top
            for i in start_index..self.stack.len() {
                self.stack[i].draw(&self.context);
            }

            // For a connection lost transition, we want to return to the ServerConnectMenu
            if matches!(transition, Transition::ConnectionLost) {
                self.context.server = None;
                transition = Transition::PopUntil(ViewId::ServerConnectMenu)
                // A place for additional handling...
            }

            match transition {
                Transition::Push(new_state) => {
                    // Pause the current state, push the new one, and start it
                    self.stack.last_mut().unwrap().on_pause(&mut self.context);
                    self.stack.push(new_state);
                    self.stack.last_mut().unwrap().on_start(&mut self.context);
                }
                Transition::PopUntil(target_id) => {
                    // We try to find the state with provided target_id. We panic if we don't
                    // find it, as it is clearly a bug
                    let mut only_overlay = true;
                    loop {
                        let curr_top = self
                            .stack
                            .last()
                            .expect("Provided target id did not exist in the app stack!");
                        if curr_top.get_id() == target_id {
                            break;
                        }

                        if !curr_top.is_overlay() {
                            // There was something on top of the target that completely covered it
                            only_overlay = false;
                        }
                        self.stack.pop();
                    }

                    // After the loop, there is at least 1 element on the stack, and the top
                    // is our target.
                    self.stack
                        .last_mut()
                        .unwrap()
                        .on_resume(&mut self.context, only_overlay);
                }
                Transition::Pop => {
                    let from_overlay = self.stack.last_mut().unwrap().is_overlay();
                    self.stack.pop();

                    if let Some(new_top) = self.stack.last_mut() {
                        // If there is something left on the stack, we should resume it.
                        new_top.as_mut().on_resume(&mut self.context, from_overlay);
                    }
                }
                Transition::ConnectionLost => {
                    // Will not happen
                }
                Transition::None => {}
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

impl View for MainMenu {
    fn draw(&mut self, _ctx: &AppContext) {
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

    fn update(&mut self, _ctx: &mut AppContext) -> Transition {
        match self.button_pressed {
            Some(button) => match button {
                MainMenuButtons::Training => Transition::Push(Box::new(TrainingMenu::new())),
                MainMenuButtons::Multiplayer => {
                    Transition::Push(Box::new(ServerConnectMenu::new()))
                }
                MainMenuButtons::Options => Transition::Push(Box::new(OptionsMenu::new())),
                MainMenuButtons::Quit => Transition::Pop,
            },
            None => Transition::None,
        }
    }

    fn on_resume(&mut self, _ctx: &mut AppContext, _from_overlay: bool) {
        self.button_pressed = None;
    }

    fn get_id(&self) -> ViewId {
        ViewId::MainMenu
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

impl View for TrainingMenu {
    fn draw(&mut self, _ctx: &AppContext) {
        let x_mid = screen_width() / 2.;

        Text::new_simple(30).draw("Training coming soon!", x_mid, 200.);
        self.back_clicked = Button::new(Field::default(), Some(TextParams::default()))
            .draw_centered(x_mid, 250., 250., 50., Some("Back"))
            .poll();
    }

    fn update(&mut self, _ctx: &mut AppContext) -> Transition {
        if self.back_clicked {
            Transition::Pop
        } else {
            Transition::None
        }
    }

    fn get_id(&self) -> ViewId {
        ViewId::TrainingMenu
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

impl View for OptionsMenu {
    fn draw(&mut self, _ctx: &AppContext) {
        let x_mid = screen_width() / 2.;

        Text::new_simple(30).draw("Options here...", x_mid, 200.);
        self.back_clicked = Button::new(Field::default(), Some(TextParams::default()))
            .draw_centered(x_mid, 250., 250., 50., Some("Back"))
            .poll();
    }

    fn update(&mut self, _ctx: &mut AppContext) -> Transition {
        if self.back_clicked {
            Transition::Pop
        } else {
            Transition::None
        }
    }

    fn get_id(&self) -> ViewId {
        ViewId::OptionsMenu
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

impl View for ServerConnectMenu {
    fn draw(&mut self, _ctx: &AppContext) {
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

    fn update(&mut self, ctx: &mut AppContext) -> Transition {
        self.servername_field.update();

        match self.button_pressed {
            Some(button) => match button {
                ServerConnectButtons::Connect => {
                    let server = Server::new();
                    if server.connect(self.servername_field.text()) {
                        ctx.server = Some(server);
                        return Transition::Push(Box::new(RoomMenu::new()));
                    }

                    self.message = Some("Could not connect to the server!".into());
                    Transition::None
                }
                ServerConnectButtons::Back => Transition::Pop,
            },
            None => Transition::None,
        }
    }

    fn on_resume(&mut self, _ctx: &mut AppContext, from_overlay: bool) {
        // For overlays, we don't want the input to disappear
        if !from_overlay {
            self.message = None;
            self.servername_field.reset();
        }
    }

    fn get_id(&self) -> ViewId {
        ViewId::ServerConnectMenu
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
    message: Option<String>,
}

impl RoomMenu {
    fn new() -> Self {
        RoomMenu {
            button_pressed: None,
            room_code_field: TextField::new(Field::default(), TextParams::default(), 10),
            message: None,
        }
    }
}

impl View for RoomMenu {
    fn draw(&mut self, _ctx: &AppContext) {
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

        if let Some(message) = self.message.as_ref() {
            Text::new_simple(30).draw(message, x_mid, y_start + 5. * sep);
        }
    }

    fn update(&mut self, ctx: &mut AppContext) -> Transition {
        self.room_code_field.update();

        if ctx.server.is_none() {
            return Transition::ConnectionLost;
        }

        let server = ctx.server.as_ref().unwrap();

        match self.button_pressed {
            Some(button) => match button {
                RoomMenuButtons::Create => {
                    if server.create_room() {
                        Transition::Push(Box::new(RoomView::new()))
                    } else {
                        self.message = Some("Could not create the room!".into());
                        Transition::None
                    }
                }
                RoomMenuButtons::Join => {
                    let room_code = self.room_code_field.text().parse::<u32>();
                    if room_code.is_err() {
                        self.message = Some("Invalid room code!".into());
                        return Transition::None;
                    }

                    if server.join_room(room_code.unwrap()) {
                        return Transition::Push(Box::new(RoomView::new()));
                    }

                    self.message = Some("Could not join the room!".into());
                    Transition::None
                }
                RoomMenuButtons::Back => Transition::Pop,
            },
            None => Transition::None,
        }
    }

    fn on_resume(&mut self, _ctx: &mut AppContext, from_overlay: bool) {
        if !from_overlay {
            self.message = None;
            self.room_code_field.reset();
        }
    }

    fn get_id(&self) -> ViewId {
        ViewId::RoomMenu
    }
}

#[derive(Clone, Copy)]
enum RoomViewButtons {
    Start,
    Leave,
}

struct RoomView {
    button_pressed: Option<RoomViewButtons>,
    room_code: u32,
    player_names: Vec<String>,
}

impl RoomView {
    fn new() -> Self {
        Self {
            button_pressed: None,
            room_code: 0,
            player_names: Vec::new(),
        }
    }
}

impl View for RoomView {
    fn draw(&mut self, _ctx: &AppContext) {
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

    fn update(&mut self, ctx: &mut AppContext) -> Transition {
        if ctx.server.is_none() {
            return Transition::ConnectionLost;
        }

        let server = ctx.server.as_mut().unwrap();

        if server.game_started() {
            let game = Game::try_new(ctx);
            match game {
                Some(game) => return Transition::Push(Box::new(game)),
                None => return Transition::ConnectionLost,
            }
        }

        self.room_code = match server.get_room_code() {
            Ok(code) => code,
            Err(_) => return Transition::Pop,
        };

        self.player_names = match server.get_player_list() {
            Ok(player_list) => player_list,
            Err(_) => return Transition::Pop,
        };

        match self.button_pressed {
            Some(button) => match button {
                RoomViewButtons::Leave => {
                    server.leave();
                    Transition::Pop
                }
                RoomViewButtons::Start => {
                    if server.start_game() {
                        let game = Game::try_new(ctx);
                        match game {
                            Some(game) => return Transition::Push(Box::new(game)),
                            None => return Transition::ConnectionLost,
                        }
                    }

                    Transition::None
                }
            },
            None => Transition::None,
        }
    }

    fn get_id(&self) -> ViewId {
        ViewId::RoomLobby
    }
}
