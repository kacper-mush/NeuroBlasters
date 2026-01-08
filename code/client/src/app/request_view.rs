use crate::app::popup::Popup;
use crate::app::{AppContext, Transition, View, ViewId};
use crate::ui::{
    BUTTON_H, BUTTON_W, Button, CANONICAL_SCREEN_MID_X, CANONICAL_SCREEN_MID_Y, Layout, TEXT_LARGE,
    Text,
};
use macroquad::prelude::*;

const TIME_TO_SHOW_ABORT: f64 = 5.;
const TIME_TO_SHOW_BEFORE_ACTION: f64 = 0.7;
pub(crate) type RequestAction = Box<dyn FnOnce(&mut AppContext) -> Transition>;

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
    success_action: Option<RequestAction>,
    abort_clicked: bool,
    abort_show_timer: Timer,
    time_to_show_timer: Timer,
}

impl RequestView {
    pub fn new_open_view(text: String, success_view: Box<dyn View>) -> Self {
        RequestView {
            text,
            success_action: Some(Box::new(|_| Transition::PopAnd(success_view))),
            abort_clicked: false,
            abort_show_timer: Timer::new(TIME_TO_SHOW_ABORT),
            time_to_show_timer: Timer::new(TIME_TO_SHOW_BEFORE_ACTION),
        }
    }

    pub fn new_transition(text: String, success_transition: Transition) -> Self {
        RequestView {
            text,
            success_action: Some(Box::new(|_| success_transition)),
            abort_clicked: false,
            abort_show_timer: Timer::new(TIME_TO_SHOW_ABORT),
            time_to_show_timer: Timer::new(TIME_TO_SHOW_BEFORE_ACTION),
        }
    }

    pub fn new_action(text: String, success_action: RequestAction) -> Self {
        RequestView {
            text,
            success_action: Some(success_action),
            abort_clicked: false,
            abort_show_timer: Timer::new(TIME_TO_SHOW_ABORT),
            time_to_show_timer: Timer::new(TIME_TO_SHOW_BEFORE_ACTION),
        }
    }
}

impl View for RequestView {
    fn draw(&mut self, _ctx: &AppContext, has_input: bool) {
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
                .draw_centered(
                    x_mid,
                    layout.next(),
                    BUTTON_W,
                    BUTTON_H,
                    Some("Abort"),
                    has_input,
                )
                .poll();
        }
    }

    fn update(&mut self, ctx: &mut AppContext) -> Transition {
        // Do not handle anything for a short amount of time; prevents flashing this screen for a milisecond
        if !self.time_to_show_timer.done() {
            return Transition::None;
        }

        // Check for server request response
        if let Some(resp) = ctx.server.take_request_response() {
            return match resp {
                // Request was successful
                Ok(_) => self.success_action.take().unwrap()(ctx),

                // Request failed: show the reason why
                Err(reason) => Transition::PopAnd(Box::new(Popup::new(reason))),
            };
        }

        if self.abort_clicked {
            Transition::ToServerlessView("User aborted connection.".into())
        } else {
            Transition::None
        }
    }

    fn is_overlay(&self) -> bool {
        true
    }

    fn get_id(&self) -> ViewId {
        ViewId::RequestView
    }
}
