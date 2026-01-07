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
    fn draw(&mut self, ctx: &AppContext) {
        if ctx.game.is_none() {
            return;
        }
        let game = &ctx.game.as_ref().unwrap();
        game.draw();
    }

    fn update(&mut self, ctx: &mut AppContext) -> Transition {
        match &mut ctx.server.client_state {
            ClientState::Playing => {
                // Default state
            }
            ClientState::Error(err) => {
                return Transition::ConnectionLost(err.clone());
            }
            ClientState::Disconnected => {
                return Transition::ConnectionLost("Disconnected from server.".into());
            }
            ClientState::Connected => {
                return Transition::ConnectionLost("Connection reset.".into());
            }

        }

        let game = ctx.game.as_mut().expect("Game context must be present.");

        if let Some(update) = ctx.server.game_update() {
            game.update(update, &mut ctx.server);
        }

        if is_key_pressed(KeyCode::Escape) {
            return Transition::Push(Box::new(InGameMenu::new()));
        }

        Transition::None
    }

    fn get_id(&self) -> ViewId {
        ViewId::GameView
    }

    fn shadow_update(&mut self, ctx: &mut AppContext) {
        // If the server is present, we update game state so that the game doesn't
        // freeze even if it is overlayed.
        // If the server is not present, that's fine, because the app frame above us
        // should handle that, or we will when we come back to focus.
        if let ClientState::Playing = &mut ctx.server.client_state
            && let Some(update) = ctx.server.game_update()
        {
            let game = ctx.game.as_mut().expect("Game context must be present.");
            game.update(update, &mut ctx.server);
        }
    }
}
