use crate::app::fps_display::FPSDisplay;
use crate::app::game::Game;
use crate::app::main_menu::MainMenu;
use crate::app::popup::Popup;
use crate::server::Server;
use crate::ui::BACKGROUND_COLOR;

use macroquad::prelude::*;

mod feeds;
mod fps_display;
mod game;
mod game_creation;
mod game_view;
mod in_game_menu;
mod main_menu;
mod options_menu;
mod popup;
mod request_view;
mod server_connect_menu;
mod server_lobby;
mod training_menu;

// Global data that persists across views
pub(crate) struct AppContext {
    pub game: Option<Game>,
    pub server: Server,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub(crate) enum ViewId {
    MainMenu,
    TrainingMenu,
    ServerConnectMenu,
    ServerLobby,
    GameView,
    InGameMenu,
    OptionsMenu,
    Popup,
    GameCreation,
    RequestView,
}

pub(crate) enum Transition {
    None,
    /// Push a new state onto the stack
    Push(Box<dyn View>),
    /// Pop the top state
    Pop,
    /// Like Pop but also pushes a new screen on top of screen below (i.e. an overlay popup)
    PopAnd(Box<dyn View>),
    /// Pop states until we find the specific ID (e.g., "Back to Main Menu")
    PopUntil(ViewId),
    /// Combination of PopUntil and PopAnd
    PopUntilAnd(ViewId, Box<dyn View>),
    /// Perform if app should go to the serverless view
    ToServerlessView(String),
}

pub(crate) trait View {
    fn get_id(&self) -> ViewId;

    /// Function for the main update doing all the work
    fn update(&mut self, ctx: &mut AppContext) -> Transition;

    /// Only the top view has input
    fn draw(&mut self, ctx: &AppContext, has_input: bool);

    fn visible_again(&mut self, _ctx: &mut AppContext) {
        // Default behavior: Do nothing (preserve text fields).
        // If we want to reset, we override this method.
    }

    /// Helper to determine if we should draw the state below this one.
    fn is_overlay(&self) -> bool {
        false
    }
}

pub(crate) struct App {
    stack: Vec<Box<dyn View>>,
    context: AppContext,
    fps_display: FPSDisplay,
}

impl App {
    pub async fn new() -> Self {
        App {
            stack: vec![Box::new(MainMenu::new())],
            context: AppContext {
                game: None,
                server: Server::new(),
            },
            fps_display: FPSDisplay::new(30),
        }
    }

    pub async fn run(&mut self) {
        while !self.stack.is_empty() {
            if let Err(reason) = self.context.server.tick() {
                self.perform_transition(Transition::ToServerlessView(reason));
            }

            if let Some(game) = &mut self.context.game
                && let Some(update) = self.context.server.game_update()
            {
                game.update(update, &mut self.context.server);
            }

            // We only run update for the state on top of the stack
            let transition = self.stack.last_mut().unwrap().update(&mut self.context);

            clear_background(BACKGROUND_COLOR);

            // Find the first state that is not letting states beneath it be drawn.
            // Start from the top of the stack
            let mut start_index = 0;
            for i in (0..self.stack.len()).rev() {
                if !self.stack[i].is_overlay() {
                    start_index = i;
                    break;
                }
            }

            // Draw from the floor up to the top
            for i in start_index..self.stack.len() {
                let has_input = i == self.stack.len() - 1;
                self.stack[i].draw(&self.context, has_input);
            }

            self.perform_transition(transition);

            self.fps_display.update();
            self.fps_display.draw();

            next_frame().await;
        }
    }

    fn perform_transition(&mut self, transition: Transition) {
        match transition {
            Transition::Push(new_state) => {
                self.stack.push(new_state);
            }
            Transition::PopUntil(target_id) => {
                let only_overlay = self.pop_until(target_id);

                // After the loop, there is at least 1 element on the stack, and the top
                // is our target.
                if !only_overlay {
                    self.stack
                        .last_mut()
                        .unwrap()
                        .visible_again(&mut self.context);
                }
            }
            Transition::PopUntilAnd(target_id, new_view) => {
                let only_overlay = self.pop_until(target_id);
                if !only_overlay && new_view.is_overlay() {
                    self.stack
                        .last_mut()
                        .unwrap()
                        .visible_again(&mut self.context);
                }

                self.stack.push(new_view);
            }
            Transition::Pop => {
                let from_overlay = self.stack.last_mut().unwrap().is_overlay();
                self.stack.pop();

                if let Some(new_top) = self.stack.last_mut()
                    && !from_overlay
                {
                    new_top.as_mut().visible_again(&mut self.context);
                }
            }
            Transition::PopAnd(new_view) => {
                let from_overlay = self.stack.last_mut().unwrap().is_overlay();
                self.stack.pop();
                if !from_overlay && new_view.is_overlay() {
                    self.stack
                        .last_mut()
                        .unwrap()
                        .visible_again(&mut self.context);
                }

                self.stack.push(new_view);
            }
            Transition::ToServerlessView(reason) => {
                self.context.server.close();
                self.perform_transition(Transition::PopUntilAnd(
                    ViewId::ServerConnectMenu,
                    Box::new(Popup::new(reason)),
                ));
            }
            Transition::None => {}
        }
    }

    fn pop_until(&mut self, target_id: ViewId) -> bool {
        let mut only_overlay = true;
        // We try to find the state with provided target_id. We panic if we don't
        // find it, as it is clearly a bug
        loop {
            let curr_top = self
                .stack
                .last()
                .expect("Provided target id did not exist in the app stack!");
            if curr_top.get_id() == target_id {
                break;
            }

            if !curr_top.is_overlay() {
                // There was something on top of the target that completely covered it
                only_overlay = false;
            }
            self.stack.pop();
        }

        only_overlay
    }
}
