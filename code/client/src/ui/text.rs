use macroquad::miniquad::window::screen_size;
use macroquad::prelude::*;

use crate::ui::theme::TEXT_HUGE;
use crate::ui::{
    CANONICAL_SCREEN_HEIGHT, CANONICAL_SCREEN_WIDTH, default_text_params, get_ui_scaling_factor,
    get_ui_transform,
};

/// When drawing text, defines what the y position refers to
#[derive(Clone)]
pub(crate) enum TextVerticalPositioning {
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
        // We don't scale using the bult-in scaling. We use font sizes to get nicer
        // results.
        Text {
            params: TextParams {
                font_size: (font_size as f32 * font_scale).round() as u16,
                ..default_text_params()
            },
            ..Default::default()
        }
    }

    /// Create default text with given font size and scaled like an UI element
    pub fn new_scaled(font_size: u16) -> Self {
        Text {
            params: TextParams {
                font_size,
                ..default_text_params()
            },
            ..Default::default()
        }
    }

    /// Create title text
    pub fn new_title() -> Self {
        Text {
            params: TextParams {
                font_size: TEXT_HUGE,
                ..default_text_params()
            },
            ..Default::default()
        }
    }

    /// User provides x, y coordinates that are already transformed
    pub fn draw_no_scaling(&self, text: &str, x: f32, y: f32) {
        let (o_x, o_y) = self.calculate_font_offset(text, &self.params);

        draw_text_ex(text, x + o_x, y + o_y, self.params.clone());
    }

    /// For drawing at a static position but scaled to window size
    pub fn draw_scaled_no_offset(&self, text: &str, x: f32, y: f32) {
        let scale = get_ui_scaling_factor();

        // Scale font
        let params = TextParams {
            font_size: (self.params.font_size as f32 * scale).round() as u16,
            ..self.params
        };

        // Scale position
        let (screen_w, screen_h) = screen_size();
        let x_scaling = screen_w / CANONICAL_SCREEN_WIDTH;
        let y_scaling = screen_h / CANONICAL_SCREEN_HEIGHT;
        let x = x * x_scaling;
        let y = y * y_scaling;

        let (o_x, o_y) = self.calculate_font_offset(text, &params);

        draw_text_ex(text, x + o_x, y + o_y, params);
    }

    /// Will be scaled as an UI element
    pub fn draw(&self, text: &str, x: f32, y: f32) {
        let (scale, transform_x, transform_y) = get_ui_transform();

        // Scale font
        let params = TextParams {
            font_size: (self.params.font_size as f32 * scale).round() as u16,
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
        };

        (x, y)
    }
}

impl Default for Text {
    fn default() -> Self {
        Text {
            params: default_text_params(),
            vertical_positioning: TextVerticalPositioning::CenterExact,
            horizontal_positioning: TextHorizontalPositioning::Center,
        }
    }
}
