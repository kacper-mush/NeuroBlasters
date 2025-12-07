#![allow(dead_code)] // For development
use macroquad::miniquad::window::{set_window_position, set_window_size};

mod app;
mod server;
mod ui;

#[macroquad::main("Neuroblasters")]
async fn main() {
    set_window_size(1920, 1080);
    set_window_position(100, 100);
    let mut app = app::App::new();

    app.run().await;
}
