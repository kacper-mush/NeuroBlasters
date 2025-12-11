use crate::app::options_menu::OptionsMenu;
use crate::app::server_connect_menu::ServerConnectMenu;
use crate::app::training_menu::TrainingMenu;
use crate::app::{AppContext, Transition, View, ViewId};
use crate::ui::{Button, Field, Text};
use macroquad::prelude::*;

#[derive(Clone, Copy)]
enum MainMenuButtons {
    Training,
    Multiplayer,
    Options,
    Quit,
}

pub(crate) struct MainMenu {
    button_pressed: Option<MainMenuButtons>,
}

impl MainMenu {
    pub fn new() -> Self {
        Self {
            button_pressed: None,
        }
    }
}

impl View for MainMenu {
    fn draw(&mut self, _ctx: &AppContext) {
        let x_mid = screen_width() / 2.;
        let default_text_params = TextParams {
            font_size: 30,
            ..Default::default()
        };

        Text {
            params: TextParams {
                font_size: 40,
                color: GRAY,
                ..Default::default()
            },
            ..Default::default()
        }
        .draw("NeuroBlasters", x_mid, 100.);

        let start_y = 200.;
        let button_w = 200.;
        let button_h = 50.;
        let sep = 80.;
        let mut button = Button::new(Field::default(), Some(default_text_params.clone()));

        self.button_pressed = None;

        if button
            .draw_centered(x_mid, start_y, button_w, button_h, Some("Train Models"))
            .poll()
        {
            self.button_pressed = Some(MainMenuButtons::Training);
        }

        if button
            .draw_centered(
                x_mid,
                start_y + sep,
                button_w,
                button_h,
                Some("Multiplayer"),
            )
            .poll()
        {
            self.button_pressed = Some(MainMenuButtons::Multiplayer);
        }

        if button
            .draw_centered(
                x_mid,
                start_y + 2. * sep,
                button_w,
                button_h,
                Some("Options"),
            )
            .poll()
        {
            self.button_pressed = Some(MainMenuButtons::Options);
        }

        if button
            .draw_centered(x_mid, start_y + 3. * sep, button_w, button_h, Some("Quit"))
            .poll()
        {
            self.button_pressed = Some(MainMenuButtons::Quit);
        }
    }

    fn update(&mut self, _ctx: &mut AppContext) -> Transition {
        match self.button_pressed {
            Some(button) => match button {
                MainMenuButtons::Training => Transition::Push(Box::new(TrainingMenu::new())),
                MainMenuButtons::Multiplayer => {
                    Transition::Push(Box::new(ServerConnectMenu::new()))
                }
                MainMenuButtons::Options => Transition::Push(Box::new(OptionsMenu::new())),
                MainMenuButtons::Quit => Transition::Pop,
            },
            None => Transition::None,
        }
    }

    fn get_id(&self) -> ViewId {
        ViewId::MainMenu
    }
}
