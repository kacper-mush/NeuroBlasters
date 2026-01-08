use macroquad::prelude::*;

use crate::ui::{
    scale_dims,
    theme::{FIELD_COLOR, FIELD_HOVER_COLOR},
};

pub(crate) struct Field {
    pub(crate) color: Color,
    pub(crate) hover_color: Color,
    pub(crate) outline_color: Color,
    pub(crate) outline_thickness: f32,
    draw_cache: Option<(f32, f32, f32, f32)>,
}

impl Field {
    pub fn draw(&mut self, x: f32, y: f32, w: f32, h: f32, has_input: bool) -> &mut Self {
        let (x, y, w, h) = scale_dims(x, y, w, h);

        let is_hovered = self.is_hovered(x, y, w, h);
        let bg_color = if is_hovered && has_input {
            self.hover_color
        } else {
            self.color
        };
        draw_rectangle(x, y, w, h, bg_color);

        draw_rectangle_lines(x, y, w, h, self.outline_thickness, self.outline_color);

        self.draw_cache = Some((x, y, w, h));

        self
    }

    pub fn poll(&self) -> bool {
        if let Some((x, y, w, h)) = self.draw_cache {
            return self.is_hovered(x, y, w, h) && is_mouse_button_released(MouseButton::Left);
        }
        false
    }

    fn is_hovered(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        let mouse_pos = mouse_position();
        mouse_pos.0 >= x && mouse_pos.0 <= x + w && mouse_pos.1 >= y && mouse_pos.1 <= y + h
    }
}

impl Default for Field {
    fn default() -> Self {
        Self {
            color: FIELD_COLOR,
            hover_color: FIELD_HOVER_COLOR,
            outline_color: BLACK,
            outline_thickness: 4.,
            draw_cache: None,
        }
    }
}
