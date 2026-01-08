use futures::executor::block_on;
use macroquad::prelude::*;
use once_cell::sync::Lazy;

pub const DARK_BG: Color = Color::new(0.05, 0.05, 0.1, 1.0); // Very dark blue/black
pub const GRID_COLOR: Color = Color::new(0.0, 1.0, 1.0, 0.1); // Faint cyan
pub const NEON_CYAN: Color = Color::new(0.0, 1.0, 1.0, 1.0);
pub const NEON_PINK: Color = Color::new(1.0, 0.0, 1.0, 1.0);
pub const WALL_COLOR: Color = Color::new(0.0, 0.2, 0.4, 0.8); // Dark semi-transparent blue
pub const WALL_OUTLINE: Color = Color::new(0.0, 1.0, 1.0, 0.5); // Cyan outline
pub const TEXT_COLOR: Color = Color::new(0.9, 0.9, 0.9, 1.0);
pub const FIELD_COLOR: Color = Color::from_rgba(0, 119, 206, 255);
pub const FIELD_HOVER_COLOR: Color = Color::from_rgba(0, 72, 125, 255);
pub const BACKGROUND_COLOR: Color = Color::from_rgba(2, 25, 89, 255);

pub const TEXT_SMALL: u16 = 20;
pub const TEXT_MID: u16 = 25;
pub const TEXT_LARGE: u16 = 35;
pub const TEXT_HUGE: u16 = 50;

/// Typical button width
pub const BUTTON_W: f32 = 300.;
/// Typical button height
pub const BUTTON_H: f32 = 55.;

pub static MAIN_FONT: Lazy<Font> = Lazy::new(|| {
    let mut font = block_on(load_ttf_font("assets/arcade_riders.ttf")).unwrap();
    font.set_filter(FilterMode::Nearest); // Better results for a pixelated font
    font
});

pub static BANNER_TEXUTRE: Lazy<Texture2D> = Lazy::new(|| {
    let banner = block_on(load_texture("assets/banner.png")).unwrap();
    banner.set_filter(FilterMode::Nearest); // Better for Pixel art
    banner
});
