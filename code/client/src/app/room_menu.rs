use crate::app::room_lobby::RoomLobby;
use crate::app::{AppContext, Transition, View, ViewId};
use crate::server::ClientState;
use crate::ui::{Button, Field, TEXT_LARGE, Text, TextField};
use common::protocol::{ClientMessage, GameCode};
use macroquad::prelude::*;

#[derive(Clone, Copy)]
enum RoomMenuButtons {
    Create,
    Join,
    Back,
}

pub(crate) struct RoomMenu {
    button_pressed: Option<RoomMenuButtons>,
    room_code_field: TextField,
    message: Option<String>,
}

impl RoomMenu {
    pub fn new() -> Self {
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

        Text::new_scaled(TEXT_LARGE).draw("Rooms", x_mid, 200.);
        if button
            .draw_centered(x_mid, y_start, w, h, Some("Create"))
            .poll()
        {
            self.button_pressed = Some(RoomMenuButtons::Create);
        }

        Text::new_scaled(TEXT_LARGE).draw("Room code:", x_mid, y_start + sep);

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
            Text::new_scaled(TEXT_LARGE).draw(message, x_mid, y_start + 5. * sep);
        }
    }

    fn update(&mut self, ctx: &mut AppContext) -> Transition {
        self.room_code_field.update();

        if ctx.server.is_none() {
            return Transition::ConnectionLost;
        }

        let server = ctx.server.as_mut().unwrap();

        match &server.client_state {
            ClientState::Error => {
                return Transition::ConnectionLost;
            }
            ClientState::Connected => {
                // That's the default state for this view
            }
            ClientState::WaitingForRoom => {
                // We are waiting for a response
            }
            ClientState::InRoom {
                game_code,
                player_names,
            } => {
                return Transition::Push(Box::new(RoomLobby::new(
                    game_code.clone(),
                    player_names.clone(),
                )));
            }
            _ => {
                panic!("Ended up in an invalid state!");
            }
        }

        match self.button_pressed {
            Some(button) => match button {
                RoomMenuButtons::Create => {
                    let res = server.send_client_message(ClientMessage::CreateGame);
                    if res.is_err() {
                        // This is more of a "could not send the request", but this is simplified for now
                        self.message = Some("Could not create the room!".into());
                    }
                    Transition::None
                }
                RoomMenuButtons::Join => {
                    let res = server.send_client_message(ClientMessage::JoinGame {
                        game_code: GameCode(self.room_code_field.text()),
                    });

                    if res.is_err() {
                        self.message = Some("Could not join the room!".into());
                    }

                    Transition::None
                }
                RoomMenuButtons::Back => {
                    ctx.server.take(); // Close server connection
                    Transition::Pop
                }
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
