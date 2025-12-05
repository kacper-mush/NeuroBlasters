use macroquad::prelude::*;

const BACKSPACE_DELAY_SECONDS: f32 = 0.1;

pub(crate) struct UnpositionedText {
    text: String,
    params: TextParams<'static>, // Enforce static font storage
}

pub(crate) struct Text {
    text: String,
    pos: Vec2,
    params: TextParams<'static>, // Enforce static font storage
}

impl UnpositionedText {
    pub fn new_ex(text: &str, params: TextParams<'static>) -> Self {
        UnpositionedText {
            text: text.to_string(),
            params,
        }
    }

    pub fn new(text: &str, font_size: u16) -> Self {
        let params = TextParams {
            font_size,
            ..Default::default()
        };
        UnpositionedText::new_ex(text, params)
    }

    pub fn position(self, x: f32, y: f32) -> Text {
        Text {
            text: self.text,
            pos: Vec2::new(x, y),
            params: self.params,
        }
    }

    pub fn position_center(self, x: f32, y: f32) -> Text {
        let text_dims = measure_text(
            &self.text,
            self.params.font,
            self.params.font_size,
            self.params.font_scale,
        );

        // The text should start half of its width left from center
        let draw_x = x - text_dims.width / 2.0;

        // Almost the same as for x, but the point draw_text recieves is the text baseline,
        // not its lowest point. If we passed the lowest point, it would get treated as the
        // baseline, and would be drawn too low. We need to offset our lowest point by how
        // much the text "dips" below baseline, which is stored in .offset_y.
        let draw_y = y - (text_dims.height / 2.0) + text_dims.offset_y;

        Text {
            text: self.text,
            pos: Vec2::new(draw_x, draw_y),
            params: self.params,
        }
    }

    pub fn position_center_left(self, x: f32, y: f32) -> Text {
        let text_dims = measure_text(
            &self.text,
            self.params.font,
            self.params.font_size,
            self.params.font_scale,
        );

        // The point draw_text recieves is the text baseline,
        // not its lowest point. If we passed the lowest point, it would get treated as the
        // baseline, and would be drawn too low. We need to offset our lowest point by how
        // much the text "dips" below baseline, which is stored in .offset_y.
        let draw_y = y - (text_dims.height / 2.0) + text_dims.offset_y;

        Text {
            text: self.text,
            pos: Vec2::new(x, draw_y),
            params: self.params,
        }
    }
}

impl Text {
    pub fn text<'a>(&'a mut self) -> &'a mut String {
        &mut self.text
    } 

    pub fn draw(&self) {
        draw_text_ex(&self.text, self.pos.x, self.pos.y, self.params.clone());
    }

    pub fn width(&self) -> f32 {
        measure_text(
            &self.text,
            self.params.font,
            self.params.font_size,
            self.params.font_scale,
        ).width
    }
}

pub(crate) struct FieldParams {
    pub(crate) color: Color,
    pub(crate) hover_color: Color,
    pub(crate) outline_color: Color,
    pub(crate) outline_thickness: f32,
}

impl Default for FieldParams {
    fn default() -> Self {
        Self {
            color: GRAY,
            hover_color: DARKGRAY,
            outline_color: BLACK,
            outline_thickness: 2.,
        }
    }
}

pub(crate) struct Button {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    field_params: FieldParams,
    text: Option<Text>,
}

impl Button {
    pub fn new(x: f32, y: f32, w: f32, h: f32, field_params: FieldParams, text: Option<UnpositionedText>) -> Self {
        Self {
            x,
            y,
            w,
            h,
            field_params,
            text: text
                .map(|t| t.position_center(x + w / 2., y + h / 2.)),
        }
    }

    pub fn new_centered(x: f32, y: f32, w: f32, h: f32, field_params: FieldParams, text: Option<UnpositionedText>) -> Self {
        Self {
            x: x - w / 2.,
            y: y - h / 2.,
            w,
            h,
            field_params,
            text: text.map(|t| t.position_center(x, y)),
        }
    }

    pub fn draw(&self) {
        let bg_color = if self.is_hovered() {
            self.field_params.hover_color
        } else {
            self.field_params.color
        };
        draw_rectangle(self.x, self.y, self.w, self.h, bg_color);

        draw_rectangle_lines(
            self.x,
            self.y,
            self.w,
            self.h,
            self.field_params.outline_thickness,
            self.field_params.outline_color,
        );

        if let Some(text) = &self.text {
            text.draw();
        }
    }

    fn is_hovered(&self) -> bool {
        let mouse_pos = mouse_position();
        mouse_pos.0 >= self.x
            && mouse_pos.0 <= self.x + self.w
            && mouse_pos.1 >= self.y
            && mouse_pos.1 <= self.y + self.h
    }

    pub fn lm_clicked(&self) -> bool {
        self.is_hovered() && is_mouse_button_released(MouseButton::Left)
    }
}

pub(crate) struct TextField {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    field_params: FieldParams,
    text: Text,
    focused: bool,
    since_last_remove: f32,
}

impl TextField {
    pub fn new(x: f32, y: f32, w: f32, h: f32, field_params: FieldParams, text: UnpositionedText) -> Self {
        Self {
            x,
            y,
            w,
            h,
            field_params,
            text: text.position_center_left(x, y + h / 2.),
            focused: false,
            since_last_remove: 0., 
        }
    }

    pub fn new_centered(x: f32, y: f32, w: f32, h: f32, field_params: FieldParams, text: UnpositionedText) -> Self {
        Self {
            x: x - w / 2.,
            y: y - h / 2.,
            w,
            h,
            field_params,
            text: text.position_center_left(x - w / 2., y),
            focused: false,
            since_last_remove: 0., 
        }
    }

    pub fn update(&mut self) {
        self.since_last_remove += get_frame_time();

        if is_mouse_button_released(MouseButton::Left) {
            // If the user hovered over and pressed, we are focused
            // else, the user pressed outside, so unfocus
            if self.is_hovered() {
                self.focused = true;
                while let Some(_) = get_char_pressed() {
                    // discard
                }
            } else {
                self.focused = false;
            }
        }

        if self.focused {
            if self.since_last_remove > BACKSPACE_DELAY_SECONDS && is_key_down(KeyCode::Backspace) {
                self.since_last_remove = 0.;
                self.text.text().pop();
            } 

            while let Some(c) = get_char_pressed() {
                if c.is_ascii_graphic() || c == ' ' {
                    self.text.text().push(c);

                    if self.text.width() > self.w {
                        self.text.text().pop();
                    }
                }
            }
        }
    }

    pub fn draw(&self) {
        let bg_color = if self.is_hovered() || self.focused {
            self.field_params.hover_color
        } else {
            self.field_params.color
        };
        draw_rectangle(self.x, self.y, self.w, self.h, bg_color);

        draw_rectangle_lines(
            self.x,
            self.y,
            self.w,
            self.h,
            self.field_params.outline_thickness,
            self.field_params.outline_color,
        );

        self.text.draw();
    }

    fn is_hovered(&self) -> bool {
        let mouse_pos = mouse_position();
        mouse_pos.0 >= self.x
            && mouse_pos.0 <= self.x + self.w
            && mouse_pos.1 >= self.y
            && mouse_pos.1 <= self.y + self.h
    }
}