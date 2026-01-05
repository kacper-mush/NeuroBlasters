use macroquad::miniquad::window::screen_size;
use macroquad::prelude::*;

pub const TEXT_SMALL: u16 = 16;
pub const TEXT_MID: u16 = 20;
pub const TEXT_LARGE: u16 = 30;
pub const TEXT_HUGE: u16 = 40;
const GLOBAL_SCALING: f32 = 1.5;
pub const CANONICAL_SCREEN_WIDTH: f32 = 1920. / GLOBAL_SCALING;
pub const CANONICAL_SCREEN_HEIGHT: f32 = 1080. / GLOBAL_SCALING;
pub const CANONICAL_SCREEN_MID_X: f32 = CANONICAL_SCREEN_WIDTH / 2.;
pub const CANONICAL_SCREEN_MID_Y: f32 = CANONICAL_SCREEN_HEIGHT / 2.;

const BACKSPACE_DELAY_SECONDS: f32 = 0.1;

fn get_ui_scaling_factor() -> f32 {
    calc_transform(CANONICAL_SCREEN_WIDTH, CANONICAL_SCREEN_HEIGHT).0
}

fn get_ui_transform() -> (f32, f32, f32) {
    calc_transform(CANONICAL_SCREEN_WIDTH, CANONICAL_SCREEN_HEIGHT)
}

fn scale_dims(x: f32, y: f32, w: f32, h: f32) -> (f32, f32, f32, f32) {
    let (scale, transform_x, transform_y) = get_ui_transform();
    let x = x * scale + transform_x;
    let y = y * scale + transform_y;
    let w = w * scale;
    let h = h * scale;
    (x, y, w, h)
}

pub fn calc_transform(canonical_w: f32, canonical_h: f32) -> (f32, f32, f32) {
    let (screen_w, screen_h) = screen_size();
    let x_scaling = screen_w / canonical_w;
    let y_scaling = screen_h / canonical_h;
    let x_offset;
    let y_offset;
    let scaling;

    // Choose scaling and offsets so that the map perfectly fits 1 dimension
    // and is centered on the second dimension
    if x_scaling < y_scaling {
        scaling = x_scaling;
        x_offset = 0.;
        y_offset = f32::abs(screen_h - canonical_h * scaling) / 2.;
    } else {
        scaling = y_scaling;
        x_offset = f32::abs(screen_w - canonical_w * scaling) / 2.;
        y_offset = 0.;
    }

    (scaling, x_offset, y_offset)
}

/// When drawing text, defines what the y position refers to
#[derive(Clone)]
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
#[derive(Clone)]
pub(crate) enum TextHorizontalPositioning {
    /// The x value is the left edge
    Left,
    /// The x value is the right edge
    Right,
    /// The x value is the center
    Center,
}

/// Adds additional positioning params to the already existing TextParams
#[derive(Clone)]
pub(crate) struct Text {
    pub(crate) params: TextParams<'static>, // Enforce static font storage
    pub(crate) vertical_positioning: TextVerticalPositioning,
    pub(crate) horizontal_positioning: TextHorizontalPositioning,
}

impl Text {
    pub fn new(
        params: TextParams<'static>,
        vertical_positioning: TextVerticalPositioning,
        horizontal_positioning: TextHorizontalPositioning,
    ) -> Self {
        Text {
            params,
            vertical_positioning,
            horizontal_positioning,
        }
    }

    /// Create default text with given font size and custom scaling
    pub fn new_simple(font_size: u16, font_scale: f32) -> Self {
        Text {
            params: TextParams {
                font_size,
                font_scale,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Create default text with given font size and scaled like an UI element
    pub fn new_scaled(font_size: u16) -> Self {
        Text {
            params: TextParams {
                font_size,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Create title text
    pub fn new_title() -> Self {
        Text {
            params: TextParams {
                font_size: TEXT_HUGE,
                color: GRAY,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// User provides x, y coordinates that are already transformed
    pub fn draw_no_scaling(&self, text: &str, x: f32, y: f32) {
        let (o_x, o_y) = self.calculate_font_offset(text, &self.params);

        draw_text_ex(text, x + o_x, y + o_y, self.params.clone());
    }

    /// Will be scaled as an UI element
    pub fn draw(&self, text: &str, x: f32, y: f32) {
        let (scale, transform_x, transform_y) = get_ui_transform();

        // Scale font
        let params = TextParams {
            font_scale: scale,
            ..self.params
        };

        // Transform position
        let x = x * scale + transform_x;
        let y = y * scale + transform_y;

        let (o_x, o_y) = self.calculate_font_offset(text, &params);

        draw_text_ex(text, x + o_x, y + o_y, params);
    }

    fn calculate_font_offset(&self, text: &str, params: &TextParams<'static>) -> (f32, f32) {
        let text_dims = measure_text(text, params.font, params.font_size, params.font_scale);
        let x = match self.horizontal_positioning {
            TextHorizontalPositioning::Left => 0.,
            TextHorizontalPositioning::Right => -text_dims.width,
            TextHorizontalPositioning::Center => -text_dims.width / 2.,
        };

        let y = match self.vertical_positioning {
            // We need to add the offset because the draw_text function draws regarding to the text baseline,
            // and not its lowest nor highest point. offset_y fixes that.
            TextVerticalPositioning::CenterExact => -(text_dims.height / 2.) + text_dims.offset_y,
            // We use approx_dims for y calculation so the result is consistent for any text with these params
            TextVerticalPositioning::CenterConsistent => {
                // "Hg" is a good approximate of highest text, because it has high-ascent and deep-descent glyphs.
                let approx_dims =
                    measure_text("Hg", params.font, params.font_size, params.font_scale);
                -(approx_dims.height / 2.) + approx_dims.offset_y
            }
            TextVerticalPositioning::Default => 0.,
        };

        (x, y)
    }
}

impl Default for Text {
    fn default() -> Self {
        Text {
            params: Default::default(),
            vertical_positioning: TextVerticalPositioning::CenterExact,
            horizontal_positioning: TextHorizontalPositioning::Center,
        }
    }
}

pub(crate) struct Field {
    pub(crate) color: Color,
    pub(crate) hover_color: Color,
    pub(crate) outline_color: Color,
    pub(crate) outline_thickness: f32,
    draw_cache: Option<(f32, f32, f32, f32)>,
}

impl Field {
    pub fn draw(&mut self, x: f32, y: f32, w: f32, h: f32) -> &mut Self {
        let (x, y, w, h) = scale_dims(x, y, w, h);

        let is_hovered = self.is_hovered(x, y, w, h);
        let bg_color = if is_hovered {
            self.hover_color
        } else {
            self.color
        };
        draw_rectangle(x, y, w, h, bg_color);

        draw_rectangle_lines(x, y, w, h, self.outline_thickness, self.outline_color);

        self.draw_cache = Some((x, y, w, h));

        self
    }

    pub fn draw_centered(&mut self, x: f32, y: f32, w: f32, h: f32) -> &mut Self {
        self.draw(x - w / 2., y - h / 2., w, h)
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
            color: GRAY,
            hover_color: DARKGRAY,
            outline_color: BLACK,
            outline_thickness: 2.,
            draw_cache: None,
        }
    }
}

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

    pub fn draw(&mut self, x: f32, y: f32, w: f32, h: f32, text: Option<&str>) -> &mut Self {
        self.field.draw(x, y, w, h);

        if let Some(text_str) = text {
            self.text
                .clone()
                .unwrap_or_default()
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
    ) -> &mut Self {
        self.draw(x - w / 2., y - h / 2., w, h, text)
    }

    pub fn poll(&self) -> bool {
        self.field.poll()
    }
}

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

    pub fn draw(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.field.draw(x, y, w, h);
        self.text.draw(&self.text_string, x, y + h / 2.);
    }

    pub fn draw_centered(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.draw(x - w / 2., y - h / 2., w, h)
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
