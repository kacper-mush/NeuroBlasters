use crate::app::room_menu::RoomMenu;
use crate::app::{AppContext, Transition, View, ViewId};
use crate::server::Server;
use crate::ui::{Button, Field, Text, TextField};
use macroquad::prelude::*;

#[derive(Copy, Clone)]
enum ServerConnectButtons {
    Connect,
    Back,
}

pub(crate) struct ServerConnectMenu {
    button_pressed: Option<ServerConnectButtons>,
    message: Option<String>,
    servername_field: TextField,
}

impl ServerConnectMenu {
    pub fn new() -> Self {
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
