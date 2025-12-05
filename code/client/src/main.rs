use macroquad::prelude::*;

#[derive(PartialEq)]
enum AppState {
    MainMenu,
    Playing,
    InGameMenu,
}

mod ui;

fn draw_the_game_world() {
    // Just a blue rectangle in the center of the screen
    let rect_size = 100.0;
    draw_rectangle(
        screen_width() / 2.0 - rect_size / 2.0,
        screen_height() / 2.0 - rect_size / 2.0,
        rect_size,
        rect_size,
        BLUE,
    );

    draw_text(
        "Game is running. Press ESC for menu.",
        20.0,
        30.0,
        20.0,
        DARKGRAY,
    );
}

#[macroquad::main("Neuroblasters")]
async fn main() {
    let mut current_state = AppState::MainMenu;
    let default_text_params = TextParams {
        font_size: 30,
        ..Default::default()
    };
    let mut input = ui::TextField::new_centered(
        screen_width() / 2.,
        400.,
        250.,
        50.,
        Default::default(),
        default_text_params.clone(),
        16,
    );

    loop {
        clear_background(DARKBLUE);

        match current_state {
            AppState::MainMenu => {
                let x_mid = screen_width() / 2.;

                let params = ui::TextParamsExtended {
                    base: TextParams {
                        font_size: 40,
                        color: GRAY,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                ui::extended_draw_text("MAIN MENU", x_mid, 100., params);

                let play_button = ui::Button::new_centered(
                    x_mid,
                    200.0,
                    200.0,
                    50.0,
                    Default::default(),
                    Some(default_text_params.clone()),
                    Some("Play game".into()),
                );
                play_button.draw();

                let quit_button = ui::Button::new_centered(
                    x_mid,
                    270.0,
                    200.0,
                    50.0,
                    Default::default(),
                    Some(default_text_params.clone()),
                    Some("Quit".into()),
                );
                quit_button.draw();

                input.update();
                input.draw();

                if play_button.lm_clicked() {
                    current_state = AppState::Playing;
                }

                if quit_button.lm_clicked() {
                    break;
                }
            }

            AppState::Playing => {
                draw_the_game_world();

                if is_key_pressed(KeyCode::Escape) {
                    current_state = AppState::InGameMenu;
                }
            }

            AppState::InGameMenu => {
                draw_the_game_world();

                // Menu grays the game
                draw_rectangle(
                    0.,
                    0.,
                    screen_width(),
                    screen_height(),
                    Color::new(0.0, 0.0, 0.0, 0.5),
                );

                let x_mid = screen_width() / 2.;

                ui::draw_text_simple_center("PAUSED", x_mid, 150., 40);

                let resume_button = ui::Button::new_centered(
                    x_mid,
                    250.0,
                    250.0,
                    50.0,
                    Default::default(),
                    Some(default_text_params.clone()),
                    Some("Resume".into()),
                );
                resume_button.draw();

                let quit_button = ui::Button::new_centered(
                    x_mid,
                    320.0,
                    250.0,
                    50.0,
                    Default::default(),
                    Some(default_text_params.clone()),
                    Some("Exit to Main Menu".into()),
                );
                quit_button.draw();

                if resume_button.lm_clicked() {
                    current_state = AppState::Playing;
                }

                if quit_button.lm_clicked() {
                    current_state = AppState::MainMenu;
                }

                if is_key_pressed(KeyCode::Escape) {
                    current_state = AppState::Playing;
                }
            }
        }

        next_frame().await
    }
}
