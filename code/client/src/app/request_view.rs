use crate::app::popup::Popup;
use crate::app::{AppContext, Transition, View, ViewId};
use crate::server::ClientState;
use crate::ui::{
    BUTTON_H, BUTTON_W, Button, CANONICAL_SCREEN_MID_X, CANONICAL_SCREEN_MID_Y, Layout, TEXT_LARGE,
    Text,
};
use macroquad::prelude::*;

const TIME_TO_SHOW_ABORT: f64 = 5.;
const TIME_TO_SHOW_BEFORE_ACTION: f64 = 1.;

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
    success_view: Option<Box<dyn View>>,
    success_transition: Option<Transition>,
    abort_clicked: bool,
    abort_show_timer: Timer,
    time_to_show_timer: Timer,
}

impl RequestView {
    pub fn new(text: String, success_view: Option<Box<dyn View>>) -> Self {
        RequestView {
            text,
            success_view,
            success_transition: None,
            abort_clicked: false,
            abort_show_timer: Timer::new(TIME_TO_SHOW_ABORT),
            time_to_show_timer: Timer::new(TIME_TO_SHOW_BEFORE_ACTION),
        }
    }

    pub fn new_with_transition(text: String, success_transition: Transition) -> Self {
        RequestView {
            text,
            success_view: None,
            success_transition: Some(success_transition),
            abort_clicked: false,
            abort_show_timer: Timer::new(TIME_TO_SHOW_ABORT),
            time_to_show_timer: Timer::new(TIME_TO_SHOW_BEFORE_ACTION),
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
        // Do not handle anything for a short amount of time; prevents flashing this screen for a milisecond
        if !self.time_to_show_timer.done() {
            return Transition::None;
        }

        if let ClientState::Error(err) = &ctx.server.client_state {
            return Transition::ConnectionLost(err.clone());
        }

        // check server request response
        // if present either success transition or display error
        match ctx.server.take_request_response() {
            None => {} // We are still waiting for a response
            Some(resp) => {
                return match resp {
                    // Request was successful
                    Ok(_) => match self.success_view.take() {
                        // Our parent requested to go to another view on success
                        Some(view) => Transition::PopAnd(view),
                        // No custom view to go, maybe a custom transition?
                        None => match self.success_transition.take() {
                            Some(transition) => transition,
                            // No view or transition, so we just pop ourselves
                            None => Transition::Pop,
                        },
                    },
                    // Request failed: show the reason why
                    Err(reason) => Transition::PopAnd(Box::new(Popup::new(reason))),
                };
            }
        }

        if self.abort_clicked {
            Transition::ConnectionLost("User aborted connection.".into())
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
