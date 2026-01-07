use crate::app::request_view::RequestView;
use crate::app::{AppContext, Transition, View, ViewId};
use crate::server::ClientState;
use crate::ui::{
    BUTTON_H, BUTTON_W, Button, CANONICAL_SCREEN_MID_X, Layout, TEXT_LARGE, TEXT_MID, Text,
};

use common::protocol::{ClientMessage, GameState};
use macroquad::prelude::*;

enum MenuButton {
    Resume,
    Quit,
    StartGame,
}

pub(crate) struct InGameMenu {
    button_clicked: Option<MenuButton>,
}

impl InGameMenu {
    pub fn new() -> Self {
        InGameMenu {
            button_clicked: None,
        }
    }
}

impl View for InGameMenu {
    fn draw(&mut self, ctx: &AppContext) {
        let x_mid = CANONICAL_SCREEN_MID_X;
        let button_w = BUTTON_W;
        let button_h = BUTTON_H;
        let mut layout = Layout::new(150., 30.);

        // Menu grays the previous view
        draw_rectangle(
            0.,
            0.,
            screen_width(),
            screen_height(),
            Color::new(0.0, 0.0, 0.0, 0.5),
        );

        Text::new_scaled(TEXT_LARGE).draw("Game Menu", x_mid, layout.next());
        layout.add(50.);

        let game_code = ctx
            .game_context
            .as_ref()
            .unwrap()
            .initial_game_info
            .game_code
            .clone();

        Text::new_scaled(TEXT_MID).draw(
            &format!("Game code: {}", game_code.0),
            x_mid,
            layout.next(),
        );
        layout.add(30.);

        self.button_clicked = None;

        if Button::default()
            .draw_centered(x_mid, layout.next(), button_w, button_h, Some("Resume"))
            .poll()
        {
            self.button_clicked = Some(MenuButton::Resume);
        }
        layout.add(button_h);

        let is_host = ctx.game_context.as_ref().is_some_and(|game_context| {
            game_context.is_host && matches!(game_context.game_state, GameState::Waiting)
        });

        if is_host {
            if Button::default()
                .draw_centered(x_mid, layout.next(), button_w, button_h, Some("Start Game"))
                .poll()
            {
                self.button_clicked = Some(MenuButton::StartGame);
            }
            layout.add(button_h);
        }

        if Button::default()
            .draw_centered(
                x_mid,
                layout.next(),
                button_w,
                button_h,
                Some("Exit to Main Menu"),
            )
            .poll()
        {
            self.button_clicked = Some(MenuButton::Quit);
        }
    }

    fn update(&mut self, ctx: &mut AppContext) -> Transition {
        match &ctx.server.client_state {
            ClientState::Playing => {
                // This is the acceptable current state
            }
            ClientState::Error(err) => {
                return Transition::ConnectionLost(err.clone());
            }
            _ => {
                panic!("Ended up in an invalid state!");
            }
        }

        if is_key_pressed(KeyCode::Escape) {
            return Transition::Pop;
        }

        if let Some(button) = &self.button_clicked {
            match button {
                MenuButton::Resume => return Transition::Pop,
                MenuButton::Quit => {
                    ctx.server.send_client_message(ClientMessage::LeaveGame);
                    let success_transition = Transition::PopUntil(ViewId::ServerLobby);
                    return Transition::Push(Box::new(RequestView::new_with_transition(
                        "Exiting game...".into(),
                        success_transition,
                    )));
                }
                MenuButton::StartGame => {
                    ctx.server
                        .send_client_message(ClientMessage::StartCountdown);
                    let success_transition = Transition::PopUntil(ViewId::Game);
                    return Transition::Push(Box::new(RequestView::new_with_transition(
                        "Starting game...".into(),
                        success_transition,
                    )));
                }
            }
        };

        Transition::None
    }

    fn get_id(&self) -> ViewId {
        ViewId::InGameMenu
    }

    fn is_overlay(&self) -> bool {
        true
    }
}
