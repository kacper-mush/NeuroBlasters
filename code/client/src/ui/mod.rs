use macroquad::miniquad::window::screen_size;
use macroquad::prelude::*;

use crate::ui::theme::{MAIN_FONT, TEXT_COLOR};

pub(crate) mod button;
pub(crate) mod field;
pub(crate) mod text;
pub(crate) mod text_field;
pub(crate) mod theme;

pub(crate) use button::*;
pub(crate) use text::*;
pub(crate) use text_field::*;
pub(crate) use theme::*;

const GLOBAL_SCALING: f32 = 1.5;
pub(crate) const CANONICAL_SCREEN_WIDTH: f32 = 1920. / GLOBAL_SCALING;
pub(crate) const CANONICAL_SCREEN_HEIGHT: f32 = 1080. / GLOBAL_SCALING;
pub(crate) const CANONICAL_SCREEN_MID_X: f32 = CANONICAL_SCREEN_WIDTH / 2.;
pub(crate) const CANONICAL_SCREEN_MID_Y: f32 = CANONICAL_SCREEN_HEIGHT / 2.;

fn get_ui_scaling_factor() -> f32 {
    calc_transform(CANONICAL_SCREEN_WIDTH, CANONICAL_SCREEN_HEIGHT).0
}

fn get_ui_transform() -> (f32, f32, f32) {
    calc_transform(CANONICAL_SCREEN_WIDTH, CANONICAL_SCREEN_HEIGHT)
}

pub(crate) fn default_text_params() -> TextParams<'static> {
    TextParams {
        font: Some(&MAIN_FONT),
        font_scale: 1.,
        color: TEXT_COLOR,
        ..Default::default()
    }
}

fn scale_dims(x: f32, y: f32, w: f32, h: f32) -> (f32, f32, f32, f32) {
    let (scale, transform_x, transform_y) = get_ui_transform();
    let x = x * scale + transform_x;
    let y = y * scale + transform_y;
    let w = w * scale;
    let h = h * scale;
    (x, y, w, h)
}

pub(crate) fn calc_transform(canonical_w: f32, canonical_h: f32) -> (f32, f32, f32) {
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

pub(crate) struct Layout {
    current_pos: f32,
    padding: f32,
}

impl Layout {
    pub fn new(start_pos: f32, padding: f32) -> Self {
        Layout {
            current_pos: start_pos,
            padding,
        }
    }

    pub fn next(&self) -> f32 {
        self.current_pos
    }

    pub fn add(&mut self, el_size: f32) {
        self.current_pos += el_size + self.padding
    }
}

pub(crate) fn draw_texture_centered(texture: &Texture2D, x: f32, y: f32, scale: f32) {
    let w = texture.width() * scale;
    let h = texture.height() * scale;
    let (x, y, w, h) = scale_dims(x - w / 2., y - h / 2., w, h);
    draw_texture_ex(
        texture,
        x,
        y,
        WHITE,
        DrawTextureParams {
            dest_size: Some(Vec2::new(w, h)), // width, height in pixels
            ..Default::default()
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_initial_position() {
        let layout = Layout::new(100.0, 10.0);
        assert_eq!(layout.next(), 100.0);
    }

    #[test]
    fn test_layout_advances_correctly() {
        let mut layout = Layout::new(50.0, 5.0);
        assert_eq!(layout.next(), 50.0);

        layout.add(20.0); // element size 20 + padding 5 = 25
        assert_eq!(layout.next(), 75.0);

        layout.add(10.0); // element size 10 + padding 5 = 15
        assert_eq!(layout.next(), 90.0);
    }

    #[test]
    fn test_layout_zero_padding() {
        let mut layout = Layout::new(0.0, 0.0);
        assert_eq!(layout.next(), 0.0);

        layout.add(100.0);
        assert_eq!(layout.next(), 100.0);
    }

    #[test]
    fn test_layout_multiple_elements() {
        let mut layout = Layout::new(0.0, 10.0);

        for i in 0..5 {
            let expected = i as f32 * (30.0 + 10.0); // element + padding
            assert_eq!(layout.next(), expected);
            layout.add(30.0);
        }
    }
}
