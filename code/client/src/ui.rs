use macroquad::prelude::*;

const BACKSPACE_DELAY_SECONDS: f32 = 0.1;

/// When drawing text, defines what the y position refers to
pub(crate) enum TextVerticalPositioning {
    /// The y value is the text baseline
    Default,
    /// The y value is exactly at the center of the text
    CenterExact,
    /// The y value is at the center of text with same params but of maximum height;
    /// not precisely the center, but when the text literal changes, the center will not.
    CenterConsistent,
}

/// When drawing text, defines what the x position refers to
pub(crate) enum TextHorizontalPositioning {
    /// The x value is the left edge
    Left,
    /// The x value is the right edge
    Right,
    /// The x value is the center
    Center,
}

/// Adds additional positioning params to the already existing TextParams
pub(crate) struct TextParamsExtended {
    pub(crate) base: TextParams<'static>, // Enforce static font storage
    pub(crate) vertical_positioning: TextVerticalPositioning,
    pub(crate) horizontal_positioning: TextHorizontalPositioning,
}

impl Default for TextParamsExtended {
    fn default() -> Self {
        TextParamsExtended {
            base: Default::default(),
            vertical_positioning: TextVerticalPositioning::CenterExact,
            horizontal_positioning: TextHorizontalPositioning::Center,
        }
    }
}

/// Simple helper to use for quick prototyping
pub fn draw_text_simple_center(text: &str, x: f32, y: f32, font_size: u16) {
    let params = TextParamsExtended {
        base: TextParams {
            font_size,
            ..Default::default()
        },
        ..Default::default()
    };
    extended_draw_text(text, x, y, params);
}

/// Draw text with additional positioning handling
pub fn extended_draw_text(text: &str, x: f32, y: f32, params: TextParamsExtended) {
    let text_dims = measure_text(
        text,
        params.base.font,
        params.base.font_size,
        params.base.font_scale,
    );

    let x = match params.horizontal_positioning {
        TextHorizontalPositioning::Left => x,
        TextHorizontalPositioning::Right => x - text_dims.width,
        TextHorizontalPositioning::Center => x - text_dims.width / 2.,
    };

    let y = match params.vertical_positioning {
        // We need to add the offset because the draw_text function draws regarding to the text baseline,
        // and not its lowest nor highest point. offset_y fixes that.
        TextVerticalPositioning::CenterExact => y - (text_dims.height / 2.) + text_dims.offset_y,
        // We use approx_dims for y calculation so the result is consistent for any text with these params
        TextVerticalPositioning::CenterConsistent => {
            // "Hg" is a good approximate of highest text, because it has high-ascent and deep-descent glyphs.
            let approx_dims = measure_text(
                "Hg",
                params.base.font,
                params.base.font_size,
                params.base.font_scale,
            );
            y - (approx_dims.height / 2.) + approx_dims.offset_y
        }
        TextVerticalPositioning::Default => y,
    };

    draw_text_ex(text, x, y, params.base);
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
    text_params: Option<TextParams<'static>>,
    text: Option<String>,
}

impl Button {
    pub fn new(
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        field_params: FieldParams,
        text_params: Option<TextParams<'static>>,
        text: Option<String>,
    ) -> Self {
        Self {
            x,
            y,
            w,
            h,
            field_params,
            text_params,
            text,
        }
    }

    pub fn new_centered(
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        field_params: FieldParams,
        text_params: Option<TextParams<'static>>,
        text: Option<String>,
    ) -> Self {
        Self {
            x: x - w / 2.,
            y: y - h / 2.,
            w,
            h,
            field_params,
            text_params,
            text,
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
            let params = TextParamsExtended {
                base: self.text_params.clone().unwrap_or_default(),
                horizontal_positioning: TextHorizontalPositioning::Center,
                vertical_positioning: TextVerticalPositioning::CenterConsistent,
            };

            extended_draw_text(text, self.x + self.w / 2., self.y + self.h / 2., params);
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
    text_params: TextParams<'static>,
    text: String,
    max_len: u32,
    focused: bool,
    since_last_remove: f32,
}

impl TextField {
    pub fn new(
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        field_params: FieldParams,
        text_params: TextParams<'static>,
        max_len: u32,
    ) -> Self {
        Self {
            x,
            y,
            w,
            h,
            field_params,
            text_params,
            text: String::new(),
            max_len,
            focused: false,
            since_last_remove: 0.,
        }
    }

    pub fn new_centered(
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        field_params: FieldParams,
        text_params: TextParams<'static>,
        max_len: u32,
    ) -> Self {
        Self::new(
            x - w / 2.,
            y - h / 2.,
            w,
            h,
            field_params,
            text_params,
            max_len,
        )
    }

    pub fn update(&mut self) {
        self.since_last_remove += get_frame_time();

        if is_mouse_button_released(MouseButton::Left) {
            // If the user hovered over and pressed, we are focused
            // else, the user pressed outside, so unfocus
            if self.is_hovered() {
                self.focused = true;
                while get_char_pressed().is_some() {
                    // discard
                }
            } else {
                self.focused = false;
            }
        }

        if self.focused {
            if self.since_last_remove > BACKSPACE_DELAY_SECONDS && is_key_down(KeyCode::Backspace) {
                self.since_last_remove = 0.;
                self.text.pop();
            }

            while let Some(c) = get_char_pressed() {
                if self.text.len() >= self.max_len as usize {
                    continue; // let the char queue exhaust but do not add more
                }

                if c.is_ascii_graphic() || c == ' ' {
                    self.text.push(c);
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

        let params = TextParamsExtended {
            base: self.text_params.clone(),
            vertical_positioning: TextVerticalPositioning::CenterConsistent,
            horizontal_positioning: TextHorizontalPositioning::Left,
        };
        extended_draw_text(&self.text, self.x, self.y + self.h / 2., params);
    }

    fn is_hovered(&self) -> bool {
        let mouse_pos = mouse_position();
        mouse_pos.0 >= self.x
            && mouse_pos.0 <= self.x + self.w
            && mouse_pos.1 >= self.y
            && mouse_pos.1 <= self.y + self.h
    }
}
