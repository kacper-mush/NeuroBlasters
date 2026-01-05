use crate::app::{AppContext, Transition, View, ViewId};
use crate::ui::{Button, CANONICAL_SCREEN_MID_X, Layout, TEXT_LARGE, Text};
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
    fn draw(&mut self, _ctx: &AppContext) {
        let x_mid = CANONICAL_SCREEN_MID_X;
        let mut layout = Layout::new(200., 30.);

        Text::new_scaled(TEXT_LARGE).draw("Options here...", x_mid, layout.next());
        layout.add(30.);

        self.back_clicked = Button::default()
            .draw_centered(x_mid, layout.next(), 250., 50., Some("Back"))
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
