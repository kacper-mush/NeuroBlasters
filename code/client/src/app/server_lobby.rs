use crate::app::game::Game;
use crate::app::game_creation::GameCreation;
use crate::app::request_view::RequestView;
use crate::app::{AppContext, Transition, View, ViewId};
use crate::server::ClientState;
use crate::ui::{
    BUTTON_H, BUTTON_W, Button, CANONICAL_SCREEN_MID_X, Layout, TEXT_LARGE, TEXT_MID, Text,
    TextField,
};
use common::protocol::{ClientMessage, GameCode};
use macroquad::prelude::*;

#[derive(Clone, Copy)]
enum ServerLobbyButtons {
    Create,
    Join,
    Back,
}

pub(crate) struct ServerLobby {
    button_pressed: Option<ServerLobbyButtons>,
    game_code_field: TextField,
    message: Option<String>,
}

impl ServerLobby {
    pub fn new() -> Self {
        ServerLobby {
            button_pressed: None,
            game_code_field: TextField::new_simple(10),
            message: None,
        }
    }
}

impl View for ServerLobby {
    fn draw(&mut self, _ctx: &AppContext) {
        let x_mid = CANONICAL_SCREEN_MID_X;
        let el_w = BUTTON_W;
        let el_h = BUTTON_H;
        let mut layout = Layout::new(100., 30.);

        self.button_pressed = None;

        Text::new_title().draw("Games", x_mid, layout.next());
        layout.add(70.);

        if Button::default()
            .draw_centered(x_mid, layout.next(), el_w, el_h, Some("Create new"))
            .poll()
        {
            self.button_pressed = Some(ServerLobbyButtons::Create);
        }
        layout.add(el_h);

        Text::new_scaled(TEXT_MID).draw("Game code:", x_mid, layout.next());
        layout.add(20.);

        let left_x = x_mid - el_w / 4.;
        let right_x = x_mid + el_w / 4.;

        self.game_code_field
            .draw_centered(left_x, layout.next(), el_w / 2., el_h);

        if Button::default()
            .draw_centered(right_x, layout.next(), el_w / 2., el_h, Some("Join"))
            .poll()
        {
            self.button_pressed = Some(ServerLobbyButtons::Join);
        }
        layout.add(el_h);

        if Button::default()
            .draw_centered(x_mid, layout.next(), el_w, el_h, Some("Back"))
            .poll()
        {
            self.button_pressed = Some(ServerLobbyButtons::Back);
        }
        layout.add(el_h);

        if let Some(message) = self.message.as_ref() {
            Text::new_scaled(TEXT_LARGE).draw(message, x_mid, layout.next());
        }
    }

    fn update(&mut self, ctx: &mut AppContext) -> Transition {
        self.game_code_field.update();

        match &ctx.server.client_state {
            ClientState::Error(err) => {
                return Transition::ConnectionLost(err.clone());
            }
            ClientState::Connected => {
                // Default state for this view
            }
            _ => {
                panic!("Invalid server state for server lobby.");
            }
        }

        match self.button_pressed {
            Some(button) => match button {
                ServerLobbyButtons::Create => Transition::Push(Box::new(GameCreation::new())),
                ServerLobbyButtons::Join => {
                    ctx.server.send_client_message(ClientMessage::JoinGame {
                        game_code: GameCode(self.game_code_field.text()),
                    });
                    let success_view = Some(Box::new(Game::new()) as Box<dyn View>);
                    Transition::Push(Box::new(RequestView::new(
                        "Joining game...".into(),
                        success_view,
                    )))
                }
                ServerLobbyButtons::Back => {
                    ctx.server.close();
                    Transition::Pop
                }
            },
            None => Transition::None,
        }
    }

    fn on_resume(&mut self, _ctx: &mut AppContext, from_overlay: bool) {
        if !from_overlay {
            self.message = None;
            self.game_code_field.reset();
        }
    }

    fn get_id(&self) -> ViewId {
        ViewId::ServerLobby
    }
}
