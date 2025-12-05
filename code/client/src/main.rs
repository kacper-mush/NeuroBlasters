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
    let mut input = ui::TextField::new_centered(screen_width() / 2., 400., 250., 50., Default::default(),
                    ui::UnpositionedText::new("s", 30));

    loop {
        clear_background(DARKBLUE);

        match current_state {
            AppState::MainMenu => {
                let x_mid = screen_width() / 2.;

                let params = TextParams {
                    font_size: 40,
                    color: GRAY,
                    ..Default::default()
                };
                ui::UnpositionedText::new_ex("MAIN MENU", params)
                    .position_center(x_mid, 100.0)
                    .draw();

                let play_button = ui::Button::new_centered(x_mid, 200.0, 200.0, 50.0, Default::default(),
                     Some(ui::UnpositionedText::new("Play game", 30)));
                play_button.draw();

                let quit_button = ui::Button::new_centered(x_mid, 270.0, 200.0, 50.0, Default::default(),
                    Some(ui::UnpositionedText::new("Quit", 30)));
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

                ui::UnpositionedText::new("PAUSED", 40)
                    .position_center(x_mid, 150.)
                    .draw();


                let resume_button = ui::Button::new_centered(x_mid, 250.0, 250.0, 50.0, Default::default(),
                    Some(ui::UnpositionedText::new("Resume", 30)));
                resume_button.draw();

                let quit_button = ui::Button::new_centered(x_mid, 320.0, 250.0, 50.0, Default::default(), 
                    Some(ui::UnpositionedText::new("Exit to Main Menu", 30)));
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
