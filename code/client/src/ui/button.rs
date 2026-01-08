use macroquad::prelude::*;

use crate::ui::default_text_params;
use crate::ui::field::Field;
use crate::ui::text::{Text, TextHorizontalPositioning, TextVerticalPositioning};
use crate::ui::theme::TEXT_LARGE;

pub(crate) struct Button {
    field: Field,
    text: Option<Text>,
}

impl Button {
    pub fn new(field: Field, params: Option<TextParams<'static>>) -> Self {
        let text = params.map(|params| Text {
            params,
            horizontal_positioning: TextHorizontalPositioning::Center,
            vertical_positioning: TextVerticalPositioning::CenterConsistent,
        });

        Self { field, text }
    }

    pub fn draw(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        text: Option<&str>,
        has_input: bool,
    ) -> &mut Self {
        self.field.draw(x, y, w, h, has_input);

        if let Some(text_str) = text {
            self.text
                .as_ref()
                .unwrap_or(&Text::default())
                .draw(text_str, x + w / 2., y + h / 2.);
        }

        self
    }

    pub fn draw_centered(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        text: Option<&str>,
        has_input: bool,
    ) -> &mut Self {
        self.draw(x - w / 2., y - h / 2., w, h, text, has_input)
    }

    pub fn poll(&self) -> bool {
        self.field.poll()
    }
}

impl Default for Button {
    fn default() -> Self {
        let params = TextParams {
            font_size: TEXT_LARGE,
            ..default_text_params()
        };

        Button::new(Field::default(), Some(params))
    }
}
