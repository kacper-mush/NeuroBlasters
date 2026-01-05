use crate::app::{AppContext, Transition, View, ViewId};
use crate::server::ClientState;
use crate::ui::{Button, CANONICAL_SCREEN_MID_X, CANONICAL_SCREEN_MID_Y, Layout, TEXT_LARGE, Text};
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
        let x_mid = CANONICAL_SCREEN_MID_X;
        let y_mid = CANONICAL_SCREEN_MID_Y;
        let mut layout = Layout::new(y_mid, 30.);

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
            layout.next(),
        );
        layout.add(30.);

        self.back_clicked = Button::default()
            .draw_centered(x_mid, layout.next(), 250., 50., Some("Back"))
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
