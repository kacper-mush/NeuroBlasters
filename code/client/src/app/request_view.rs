use crate::app::{AppContext, Transition, View, ViewId};
use crate::server::ClientState;
use crate::ui::{
    BUTTON_H, BUTTON_W, Button, CANONICAL_SCREEN_MID_X, CANONICAL_SCREEN_MID_Y, Layout, TEXT_LARGE,
    Text,
};
use macroquad::prelude::*;

const TIME_TO_SHOW_ABORT: f64 = 5.;

// TODO: move it to somewhere nice :)
struct Timer {
    start: f64,
    duration: f64,
}

impl Timer {
    fn new(seconds: f64) -> Self {
        Self {
            start: macroquad::time::get_time(),
            duration: seconds,
        }
    }

    fn done(&self) -> bool {
        macroquad::time::get_time() - self.start >= self.duration
    }
}

pub(crate) struct RequestView {
    text: String,
    success_transition: Transition,
    abort_clicked: bool,
    abort_show_timer: Timer,
}

impl RequestView {
    pub fn new(text: String, success_transition: Transition) -> Self {
        RequestView {
            text,
            success_transition,
            abort_clicked: false,
            abort_show_timer: Timer::new(TIME_TO_SHOW_ABORT),
        }
    }
}

impl View for RequestView {
    fn draw(&mut self, _ctx: &AppContext) {
        let x_mid = CANONICAL_SCREEN_MID_X;
        let y_mid = CANONICAL_SCREEN_MID_Y;
        let mut layout = Layout::new(y_mid - 50., 30.);

        // overlay the previous view
        draw_rectangle(
            0.,
            0.,
            screen_width(),
            screen_height(),
            Color::new(0.0, 0.0, 0.0, 0.5),
        );

        Text::new_scaled(TEXT_LARGE).draw(&self.text, x_mid, layout.next());
        layout.add(30.);

        if self.abort_show_timer.done() {
            self.abort_clicked = Button::default()
                .draw_centered(x_mid, layout.next(), BUTTON_W, BUTTON_H, Some("Abort"))
                .poll();
        }
    }

    fn update(&mut self, ctx: &mut AppContext) -> Transition {
        if ctx.server.is_none() {
            return Transition::ConnectionLost("Server connection lost.".into());
        }

        let server = ctx.server.as_mut().unwrap();

        match &server.client_state {
            ClientState::Error(err) => {
                return Transition::ConnectionLost(err.clone());
            }
            _ => {
                // We listen and we don't judge
            }
        }

        // check server request response
        // if present either success transition or display error

        if self.abort_clicked {
            Transition::Pop
        } else {
            Transition::None
        }
    }

    fn is_overlay(&self) -> bool {
        true
    }

    fn get_id(&self) -> ViewId {
        ViewId::Popup
    }
}
