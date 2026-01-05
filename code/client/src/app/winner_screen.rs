use crate::app::{AppContext, Transition, View, ViewId};
use crate::server::ClientState;
use crate::ui::{Button, Field, TEXT_LARGE, Text};
use common::protocol::Team;
use macroquad::prelude::*;

pub(crate) struct WinnerScreen {
    winner: Team,
    back_clicked: bool,
}

impl WinnerScreen {
    pub fn new(winner: Team) -> Self {
        WinnerScreen {
            winner,
            back_clicked: false,
        }
    }
}

impl View for WinnerScreen {
    fn draw(&mut self, _ctx: &AppContext) {
        let x_mid = screen_width() / 2.;
        let y_mid = screen_height() / 2.;

        // overlay the previous view
        draw_rectangle(
            0.,
            0.,
            screen_width(),
            screen_height(),
            Color::new(0.0, 0.0, 0.0, 0.5),
        );

        Text::new_scaled(TEXT_LARGE).draw(
            &format!("Winner is: {:?}!", self.winner).to_string(),
            x_mid,
            y_mid,
        );
        self.back_clicked = Button::new(Field::default(), Some(TextParams::default()))
            .draw_centered(x_mid, y_mid + 50., 250., 50., Some("Back"))
            .poll();
    }

    fn update(&mut self, ctx: &mut AppContext) -> Transition {
        if ctx.server.is_none() {
            return Transition::ConnectionLost;
        }
        let server = ctx.server.as_mut().unwrap();

        match &server.client_state {
            ClientState::AfterGame { winner: _ } => {
                // Only valid state here
            }
            ClientState::Error => {
                return Transition::ConnectionLost;
            }
            _ => {
                panic!("Ended up in an invalid state!");
            }
        }

        if self.back_clicked {
            server.back_to_lobby();
            Transition::Pop
        } else {
            Transition::None
        }
    }

    fn is_overlay(&self) -> bool {
        true
    }

    fn get_id(&self) -> ViewId {
        ViewId::WinnerScreen
    }
}
