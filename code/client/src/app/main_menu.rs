use crate::app::options_menu::OptionsMenu;
use crate::app::server_connect_menu::ServerConnectMenu;
use crate::app::training_menu::TrainingMenu;
use crate::app::{AppContext, Transition, View, ViewId};
use crate::ui::{Button, CANONICAL_SCREEN_MID_X, Layout, draw_texture_centered};

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
    fn draw(&mut self, ctx: &AppContext) {
        let mut layout = Layout::new(100., 30.);
        let x_mid = CANONICAL_SCREEN_MID_X;
        let button_w = 200.;
        let button_h = 50.;

        draw_texture_centered(&ctx.banner_texture, x_mid, layout.next(), 1.5);
        layout.add(100.);

        self.button_pressed = None;

        if Button::default()
            .draw_centered(
                x_mid,
                layout.next(),
                button_w,
                button_h,
                Some("Train Models"),
            )
            .poll()
        {
            self.button_pressed = Some(MainMenuButtons::Training);
        }
        layout.add(button_h);

        if Button::default()
            .draw_centered(
                x_mid,
                layout.next(),
                button_w,
                button_h,
                Some("Multiplayer"),
            )
            .poll()
        {
            self.button_pressed = Some(MainMenuButtons::Multiplayer);
        }
        layout.add(button_h);

        if Button::default()
            .draw_centered(x_mid, layout.next(), button_w, button_h, Some("Options"))
            .poll()
        {
            self.button_pressed = Some(MainMenuButtons::Options);
        }
        layout.add(button_h);

        if Button::default()
            .draw_centered(x_mid, layout.next(), button_w, button_h, Some("Quit"))
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
