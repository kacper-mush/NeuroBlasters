use crate::app::{AppContext, Transition, View, ViewId};
use crate::ui::{Button, Field, Text};
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
        let x_mid = screen_width() / 2.;

        Text::new_simple(30).draw("Options here...", x_mid, 200.);
        self.back_clicked = Button::new(Field::default(), Some(TextParams::default()))
            .draw_centered(x_mid, 250., 250., 50., Some("Back"))
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
