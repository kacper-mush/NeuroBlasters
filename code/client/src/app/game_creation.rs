use crate::app::request_view::RequestView;
use crate::app::server_lobby::ServerLobby;
use crate::app::{AppContext, Transition, View, ViewId};
use crate::server::ClientState;
use crate::ui::{
    BUTTON_H, BUTTON_W, Button, CANONICAL_SCREEN_MID_X, Layout, TEXT_MID, Text,
    TextVerticalPositioning, default_text_params,
};
use common::game::map::MapName;
use common::protocol::ClientMessage;
use macroquad::prelude::*;

const ROUND_NUMBER_CHOICES: [u8; 5] = [1, 5, 10, 15, 20];

#[derive(Copy, Clone)]
enum GameCreationButtons {
    MapScrollLeft,
    MapScrollRight,
    RoundScrollLeft,
    RoundScrollRight,
    Create,
    Back,
}

pub(crate) struct GameCreation {
    button_pressed: Option<GameCreationButtons>,
    round_index: usize,
    current_map: MapName,
}

impl GameCreation {
    pub fn new() -> Self {
        Self {
            button_pressed: None,
            round_index: 1,
            current_map: MapName::Basic,
        }
    }
}

impl View for GameCreation {
    fn draw(&mut self, _ctx: &AppContext, has_input: bool) {
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

        Text::new_title().draw("Create Game", x_mid, layout.next());
        layout.add(70.);

        Text::new_scaled(TEXT_MID).draw("Choose number of rounds:", x_mid, layout.next());
        layout.add(20.);

        let num_rounds = ROUND_NUMBER_CHOICES[self.round_index];

        consitent_text.draw(&num_rounds.to_string(), x_mid, layout.next());
        if Button::default()
            .draw_centered(x_mid - 100., layout.next(), 50., 50., Some("<"), has_input)
            .poll()
        {
            self.button_pressed = Some(GameCreationButtons::RoundScrollLeft);
        }
        if Button::default()
            .draw_centered(x_mid + 100., layout.next(), 50., 50., Some(">"), has_input)
            .poll()
        {
            self.button_pressed = Some(GameCreationButtons::RoundScrollRight);
        }
        layout.add(el_h);

        Text::new_scaled(TEXT_MID).draw("Choose map:", x_mid, layout.next());
        layout.add(20.);

        let map_name = format!("{:?}", self.current_map);
        consitent_text.draw(&map_name, x_mid, layout.next());
        if Button::default()
            .draw_centered(x_mid - 100., layout.next(), 50., 50., Some("<"), has_input)
            .poll()
        {
            self.button_pressed = Some(GameCreationButtons::MapScrollLeft);
        }
        if Button::default()
            .draw_centered(x_mid + 100., layout.next(), 50., 50., Some(">"), has_input)
            .poll()
        {
            self.button_pressed = Some(GameCreationButtons::MapScrollRight);
        }
        layout.add(el_h);

        if Button::default()
            .draw_centered(x_mid, layout.next(), el_w, el_h, Some("Create"), has_input)
            .poll()
        {
            self.button_pressed = Some(GameCreationButtons::Create);
        }
        layout.add(el_h);

        if Button::default()
            .draw_centered(x_mid, layout.next(), el_w, el_h, Some("Back"), has_input)
            .poll()
        {
            self.button_pressed = Some(GameCreationButtons::Back);
        }
    }

    fn update(&mut self, ctx: &mut AppContext) -> Transition {
        ctx.server.assert_state(ClientState::Connected);

        match self.button_pressed {
            Some(button) => match button {
                GameCreationButtons::Create => {
                    ctx.server.send_client_message(ClientMessage::CreateGame {
                        map: self.current_map,
                        rounds: ROUND_NUMBER_CHOICES[self.round_index],
                    });
                    Transition::Push(Box::new(RequestView::new_action(
                        "Creating game...".into(),
                        ServerLobby::get_game_completion_action(),
                    )))
                }
                GameCreationButtons::Back => Transition::Pop,
                GameCreationButtons::MapScrollLeft => {
                    self.current_map = self.current_map.prev();
                    Transition::None
                }
                GameCreationButtons::MapScrollRight => {
                    self.current_map = self.current_map.next();
                    Transition::None
                }
                GameCreationButtons::RoundScrollLeft => {
                    let len = ROUND_NUMBER_CHOICES.len();
                    self.round_index = (len + self.round_index - 1) % len;
                    Transition::None
                }
                GameCreationButtons::RoundScrollRight => {
                    let len = ROUND_NUMBER_CHOICES.len();
                    self.round_index = (self.round_index + 1) % len;
                    Transition::None
                }
            },
            None => Transition::None,
        }
    }

    fn get_id(&self) -> ViewId {
        ViewId::GameCreation
    }
}
