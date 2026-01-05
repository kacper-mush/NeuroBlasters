use crate::app::game::Game;
use crate::app::{AppContext, Transition, View, ViewId};
use crate::server::ClientState;
use crate::ui::{Button, CANONICAL_SCREEN_MID_X, Layout, TEXT_HUGE, TEXT_LARGE, TEXT_MID, Text};
use common::protocol::{ClientMessage, GameCode};

// This file will be deleted in the future

#[derive(Clone, Copy)]
enum RoomLobbyButtons {
    Start,
    Leave,
}

pub(crate) struct RoomLobby {
    button_pressed: Option<RoomLobbyButtons>,
    game_code: GameCode,
    player_names: Vec<String>,
}

impl RoomLobby {
    pub fn new(game_code: GameCode, player_names: Vec<String>) -> Self {
        Self {
            button_pressed: None,
            game_code,
            player_names,
        }
    }
}

impl View for RoomLobby {
    fn draw(&mut self, _ctx: &AppContext) {
        let x_mid = CANONICAL_SCREEN_MID_X;
        let button_w = 300.;
        let button_h = 50.;
        let mut layout = Layout::new(100., 30.);

        let title = format!("Room code: {}", self.game_code.0);
        Text::new_scaled(TEXT_HUGE).draw(&title, x_mid, layout.next());
        layout.add(40.);

        Text::new_scaled(TEXT_LARGE).draw("Players:", x_mid, layout.next());
        layout.add(30.);

        for name in &self.player_names {
            Text::new_scaled(TEXT_MID).draw(name, x_mid, layout.next());
            layout.add(30.);
        }

        self.button_pressed = None;

        if Button::default()
            .draw_centered(x_mid, layout.next(), button_w, button_h, Some("Start"))
            .poll()
        {
            self.button_pressed = Some(RoomLobbyButtons::Start);
        }
        layout.add(button_h);

        if Button::default()
            .draw_centered(x_mid, layout.next(), button_w, button_h, Some("Back"))
            .poll()
        {
            self.button_pressed = Some(RoomLobbyButtons::Leave);
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
                // Room lost
                return Transition::Pop;
            }
            ClientState::InRoom {
                game_code,
                player_names,
            } => {
                // Default state, update the display values
                self.game_code = game_code.clone();
                self.player_names = player_names.clone();
            }
            ClientState::Playing { game_engine } => {
                // We were sent to a game
                return Transition::Push(Box::new(Game::new(game_engine.clone())));
            }
            _ => {
                panic!("Ended up in an invalid state!");
            }
        }

        match self.button_pressed {
            Some(button) => match button {
                RoomLobbyButtons::Leave => {
                    // We don't care if server approves
                    let _ = server.send_client_message(ClientMessage::LeaveGame);
                    Transition::Pop
                }
                RoomLobbyButtons::Start => {
                    let res = server.send_client_message(ClientMessage::StartGame);
                    if res.is_err() {
                        print!("Could not start game!"); // for now
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
