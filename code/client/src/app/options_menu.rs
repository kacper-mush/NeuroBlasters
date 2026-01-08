use crate::app::{AppContext, Transition, View, ViewId};
use crate::ui::{
    BUTTON_H, BUTTON_W, Button, CANONICAL_SCREEN_MID_X, CANONICAL_SCREEN_MID_Y, Layout, TEXT_LARGE,
    Text,
};
use macroquad::prelude::*;

pub(crate) struct OptionsMenu {
    back_clicked: bool,
}

impl OptionsMenu {
    pub fn new() -> Self {
        OptionsMenu {
            back_clicked: false,
        }
    }
}

impl View for OptionsMenu {
    fn draw(&mut self, _ctx: &AppContext, has_input: bool) {
        let x_mid = CANONICAL_SCREEN_MID_X;
        let y_mid = CANONICAL_SCREEN_MID_Y;
        let mut layout = Layout::new(y_mid - 50., 30.);

        Text::new_scaled(TEXT_LARGE).draw("Options here...", x_mid, layout.next());
        layout.add(30.);

        self.back_clicked = Button::default()
            .draw_centered(
                x_mid,
                layout.next(),
                BUTTON_W,
                BUTTON_H,
                Some("Back"),
                has_input,
            )
            .poll();
    }

    fn update(&mut self, _ctx: &mut AppContext) -> Transition {
        if self.back_clicked {
            Transition::Pop
        } else {
            Transition::None
        }
    }

    fn get_id(&self) -> ViewId {
        ViewId::OptionsMenu
    }
}
