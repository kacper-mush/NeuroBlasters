use crate::app::in_game_menu::InGameMenu;

use crate::app::{AppContext, Transition, View, ViewId};
use crate::server::ClientState;
use macroquad::prelude::*;

pub(crate) struct GameView;

impl GameView {
    pub fn new() -> Self {
        Self {}
    }
}

impl View for GameView {
    fn draw(&mut self, ctx: &AppContext, _has_input: bool) {
        if ctx.game.is_none() {
            return;
        }
        let game = &ctx.game.as_ref().unwrap();
        game.draw();
    }

    fn update(&mut self, ctx: &mut AppContext) -> Transition {
        ctx.server.assert_state(ClientState::Playing);

        if is_key_pressed(KeyCode::Escape) {
            return Transition::Push(Box::new(InGameMenu::new()));
        }

        Transition::None
    }

    fn get_id(&self) -> ViewId {
        ViewId::GameView
    }
}
