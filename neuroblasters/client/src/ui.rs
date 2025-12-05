use macroquad::prelude::*;

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
}

impl Text {
    pub fn draw(&self) {
        draw_text_ex(&self.text, self.pos.x, self.pos.y, self.params.clone());
    }
}

pub(crate) struct ButtonParams {
    pub(crate) color: Color,
    pub(crate) hover_color: Color,
    pub(crate) outline_color: Color,
    pub(crate) outline_thickness: f32,
    pub(crate) text: Option<UnpositionedText>,
}

impl Default for ButtonParams {
    fn default() -> ButtonParams {
        ButtonParams {
            color: GRAY,
            hover_color: DARKGRAY,
            outline_color: BLACK,
            outline_thickness: 2.,
            text: None,
        }
    }
}

pub(crate) struct Button {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
    hover_color: Color,
    outline_color: Color,
    outline_thickness: f32,
    text: Option<Text>,
}

impl Button {
    pub fn new(x: f32, y: f32, w: f32, h: f32, params: ButtonParams) -> Self {
        Button {
            x,
            y,
            w,
            h,
            color: params.color,
            hover_color: params.hover_color,
            outline_color: params.outline_color,
            outline_thickness: params.outline_thickness,
            text: params
                .text
                .map(|t| t.position_center(x + w / 2., y + h / 2.)),
        }
    }

    pub fn new_centered(x: f32, y: f32, w: f32, h: f32, params: ButtonParams) -> Self {
        Button {
            x: x - w / 2.,
            y: y - h / 2.,
            w,
            h,
            color: params.color,
            hover_color: params.hover_color,
            outline_color: params.outline_color,
            outline_thickness: params.outline_thickness,
            text: params.text.map(|t| t.position_center(x, y)),
        }
    }

    pub fn draw(&self) {
        let bg_color = if self.is_hovered() {
            self.hover_color
        } else {
            self.color
        };
        draw_rectangle(self.x, self.y, self.w, self.h, bg_color);

        draw_rectangle_lines(
            self.x,
            self.y,
            self.w,
            self.h,
            self.outline_thickness,
            self.outline_color,
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
