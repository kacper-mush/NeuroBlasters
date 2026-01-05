use crate::app::room_menu::RoomMenu;
use crate::app::{AppContext, Transition, View, ViewId};
use crate::server::{ClientState, Server};
use crate::ui::{Button, CANONICAL_SCREEN_MID_X, Field, TEXT_LARGE, TEXT_MID, Text, TextField};
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
        let x_mid = CANONICAL_SCREEN_MID_X;
        let mut button = Button::new(Field::default(), Some(TextParams::default()));
        let w = 300.;
        let h = 50.;
        let y_start = 270.;
        let sep = 80.;

        Text::new_scaled(TEXT_LARGE).draw("Connect to server", x_mid, 200.);

        let default_message = "Enter server name:";

        let message = self.message.as_deref().unwrap_or(default_message);
        Text::new_scaled(TEXT_MID).draw(message, x_mid, 230.);

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

        if let Some(server) = &ctx.server {
            match server.client_state {
                ClientState::Error => {
                    // Handshaking failed
                    self.message = Some("Could not connect to the server!".into());
                    ctx.server.take(); // Drop the server
                    return Transition::None;
                }
                ClientState::Handshaking => {
                    // Waiting for server response, do nothing
                }
                ClientState::Connected => {
                    println!("Connected!");
                    return Transition::Push(Box::new(RoomMenu::new()));
                }
                _ => {
                    panic!("Ended up in an invalid state!");
                }
            }
        }

        match self.button_pressed {
            Some(button) => match button {
                ServerConnectButtons::Connect => {
                    if ctx.server.is_some() {
                        self.message = Some("Connecting... Wait...".into());
                        return Transition::None;
                    }

                    let server = Server::new(self.servername_field.text(), "Bulbulator".into());
                    match server {
                        Err(err) => {
                            self.message =
                                Some(format!("Could not connect to the server!: {}", err));
                        }
                        Ok(server) => {
                            ctx.server = Some(server);
                        }
                    }
                    Transition::None
                }
                ServerConnectButtons::Back => {
                    // Drop the connection if we are going back to main menu
                    ctx.server.take();
                    Transition::Pop
                }
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
