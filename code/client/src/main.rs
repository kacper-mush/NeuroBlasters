use app::App;
use macroquad::miniquad::conf::Icon;
use macroquad::prelude::*;

mod app;
mod server;
mod ui;

fn window_conf() -> Conf {
    Conf {
        window_title: "NeuroBlasters".into(),
        window_width: 1080,
        window_height: 720,
        icon: Some(load_icon()),
        ..Default::default()
    }
}

fn load_icon() -> Icon {
    fn load_png<const N: usize>(path: &str) -> [u8; N] {
        let img = image::open(path).expect("Failed to open PNG").to_rgba8();
        let raw = img.into_raw();
        assert!(raw.len() == N, "Image has wrong size");

        let mut arr = [0u8; N];
        arr.copy_from_slice(&raw);
        arr
    }

    Icon {
        small: load_png::<1024>("assets/logo16.png"),  // 16*16*4
        medium: load_png::<4096>("assets/logo32.png"), // 32*32*4
        big: load_png::<16384>("assets/logo64.png"),   // 64*64*4
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut app = App::new().await;
    app.run().await;
}
