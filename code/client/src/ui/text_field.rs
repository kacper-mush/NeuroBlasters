use crate::ui::{
    default_text_params,
    field::Field,
    text::{Text, TextHorizontalPositioning, TextVerticalPositioning},
};
use macroquad::prelude::*;

const BACKSPACE_DELAY_SECONDS: f32 = 0.1;

pub(crate) struct TextField {
    field: Field,
    text: Text,
    text_string: String,
    max_len: u32,
    focused: bool,
    since_last_remove: f32,
}

impl TextField {
    pub fn new(field: Field, text_params: TextParams<'static>, max_len: u32) -> Self {
        Self {
            field,
            text: Text {
                params: text_params,
                vertical_positioning: TextVerticalPositioning::CenterConsistent,
                horizontal_positioning: TextHorizontalPositioning::Left,
            },
            text_string: String::new(),
            max_len,
            focused: false,
            since_last_remove: 0.,
        }
    }

    pub fn new_simple(max_len: u32) -> Self {
        TextField::new(Field::default(), default_text_params(), max_len)
    }

    pub fn draw(&mut self, x: f32, y: f32, w: f32, h: f32, has_input: bool) {
        self.field.draw(x, y, w, h, has_input);
        let left_pad = 0.05 * w;
        self.text.draw(&self.text_string, x + left_pad, y + h / 2.);
    }

    pub fn draw_centered(&mut self, x: f32, y: f32, w: f32, h: f32, has_input: bool) {
        self.draw(x - w / 2., y - h / 2., w, h, has_input)
    }

    pub fn update(&mut self) {
        self.since_last_remove += get_frame_time();

        if self.field.poll() {
            if !self.focused {
                self.focused = true;
                while get_char_pressed().is_some() {
                    // discard
                }
            }
        } else if is_mouse_button_released(MouseButton::Left) || is_key_released(KeyCode::Escape) {
            // User clicked outside or pressed escape
            self.focused = false;
        }

        if self.focused {
            if self.since_last_remove > BACKSPACE_DELAY_SECONDS && is_key_down(KeyCode::Backspace) {
                self.since_last_remove = 0.;
                self.text_string.pop();
            }

            while let Some(c) = get_char_pressed() {
                if self.text_string.len() >= self.max_len as usize {
                    continue; // let the char queue exhaust but do not add more
                }

                if c.is_ascii_graphic() || c == ' ' {
                    self.text_string.push(c);
                }
            }
        }
    }

    pub fn text(&self) -> String {
        self.text_string.clone()
    }

    pub fn reset(&mut self) {
        self.text_string.clear();
        self.focused = false;
        self.since_last_remove = 0.;
    }
}
