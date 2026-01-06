use crate::app::popup::Popup;
use crate::app::room_lobby::RoomLobby;
use crate::app::{AppContext, Transition, View, ViewId};
use crate::server::ClientState;
use crate::ui::{
    BUTTON_H, BUTTON_W, Button, CANONICAL_SCREEN_MID_X, Layout, TEXT_MID, Text,
    TextVerticalPositioning, default_text_params,
};
use common::game::map::MapName;
use common::protocol::ClientMessage;
use macroquad::prelude::*;

const ROUND_NUMBER_CHOICES: [u16; 5] = [1, 5, 10, 15, 20];

#[derive(Copy, Clone)]
enum RoomCreationButtons {
    MapScrollLeft,
    MapScrollRight,
    RoundScrollLeft,
    RoundScrollRight,
    Create,
    Back,
}

pub(crate) struct RoomCreation {
    button_pressed: Option<RoomCreationButtons>,
    round_index: usize,
    current_map: MapName,
}

impl RoomCreation {
    pub fn new() -> Self {
        Self {
            button_pressed: None,
            round_index: 1,
            current_map: MapName::Basic,
        }
    }
}

impl View for RoomCreation {
    fn draw(&mut self, _ctx: &AppContext) {
        // For scrollers
        let consitent_text = Text {
            params: TextParams {
                font_size: TEXT_MID,
                ..default_text_params()
            },
            vertical_positioning: TextVerticalPositioning::CenterConsistent,
            ..Default::default()
        };

        let x_mid = CANONICAL_SCREEN_MID_X;
        let el_w = BUTTON_W;
        let el_h = BUTTON_H;

        let mut layout = Layout::new(100., 30.);
        self.button_pressed = None;

        Text::new_title().draw("Create Room", x_mid, layout.next());
        layout.add(70.);

        Text::new_scaled(TEXT_MID).draw("Choose number of rounds:", x_mid, layout.next());
        layout.add(20.);

        let num_rounds = ROUND_NUMBER_CHOICES[self.round_index];

        consitent_text.draw(&num_rounds.to_string(), x_mid, layout.next());
        if Button::default()
            .draw_centered(x_mid - 100., layout.next(), 50., 50., Some("<"))
            .poll()
        {
            self.button_pressed = Some(RoomCreationButtons::RoundScrollLeft);
        }
        if Button::default()
            .draw_centered(x_mid + 100., layout.next(), 50., 50., Some(">"))
            .poll()
        {
            self.button_pressed = Some(RoomCreationButtons::RoundScrollRight);
        }
        layout.add(el_h);

        Text::new_scaled(TEXT_MID).draw("Choose map:", x_mid, layout.next());
        layout.add(20.);

        let map_name = format!("{:?}", self.current_map);
        consitent_text.draw(&map_name, x_mid, layout.next());
        if Button::default()
            .draw_centered(x_mid - 100., layout.next(), 50., 50., Some("<"))
            .poll()
        {
            self.button_pressed = Some(RoomCreationButtons::MapScrollLeft);
        }
        if Button::default()
            .draw_centered(x_mid + 100., layout.next(), 50., 50., Some(">"))
            .poll()
        {
            self.button_pressed = Some(RoomCreationButtons::MapScrollRight);
        }
        layout.add(el_h);

        if Button::default()
            .draw_centered(x_mid, layout.next(), el_w, el_h, Some("Create"))
            .poll()
        {
            self.button_pressed = Some(RoomCreationButtons::Create);
        }
        layout.add(el_h);

        if Button::default()
            .draw_centered(x_mid, layout.next(), el_w, el_h, Some("Back"))
            .poll()
        {
            self.button_pressed = Some(RoomCreationButtons::Back);
        }
    }

    fn update(&mut self, ctx: &mut AppContext) -> Transition {
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
                RoomCreationButtons::Create => {
                    let res = server.send_client_message(ClientMessage::CreateGame);
                    if res.is_err() {
                        // This is more of a "could not send the request", but this is simplified for now
                        return Transition::Push(Box::new(Popup::new(
                            "Could not create the room!".into(),
                        )));
                    }
                    Transition::None
                }
                RoomCreationButtons::Back => Transition::Pop,
                RoomCreationButtons::MapScrollLeft => {
                    self.current_map = self.current_map.prev();
                    Transition::None
                }
                RoomCreationButtons::MapScrollRight => {
                    self.current_map = self.current_map.next();
                    Transition::None
                }
                RoomCreationButtons::RoundScrollLeft => {
                    let len = ROUND_NUMBER_CHOICES.len();
                    self.round_index = (len + self.round_index - 1) % len;
                    Transition::None
                }
                RoomCreationButtons::RoundScrollRight => {
                    let len = ROUND_NUMBER_CHOICES.len();
                    self.round_index = (self.round_index + 1) % len;
                    Transition::None
                }
            },
            None => Transition::None,
        }
    }

    fn get_id(&self) -> ViewId {
        ViewId::RoomCreation
    }
}
