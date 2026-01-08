use crate::app::game::Game;
use crate::app::main_menu::MainMenu;
use crate::app::popup::Popup;
use crate::server::Server;
use crate::ui::BACKGROUND_COLOR;

use macroquad::prelude::*;

pub mod game;
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
    /// combination of PopUntil and PopAnd
    PopUntilAnd(ViewId, Box<dyn View>),
    /// A state that was reliant on a server connection lost it, exit with a reason
    ConnectionLost(String),
}

pub(crate) trait View {
    fn get_id(&self) -> ViewId;

    /// Function for the main update doing all the work
    fn update(&mut self, ctx: &mut AppContext) -> Transition;

    /// Update that is called for View not on top of the app stack,
    /// no input handling is allowed
    fn shadow_update(&mut self, _ctx: &mut AppContext) {}

    fn draw(&mut self, ctx: &AppContext);

    // -- Lifecycle Hooks --

    /// Called when this state becomes the top of the stack
    fn on_start(&mut self, _ctx: &mut AppContext) {}

    /// Called when a new state is pushed on TOP of this one.
    fn on_pause(&mut self, _ctx: &mut AppContext) {}

    /// Called when some amount of states above this one were popped.
    /// `from_overlay` tells us if the thing that just closed was a popup or a full view.
    fn on_resume(&mut self, _ctx: &mut AppContext, _from_overlay: bool) {
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
}

impl App {
    pub async fn new() -> Self {
        App {
            stack: vec![Box::new(MainMenu::new())],
            context: AppContext {
                game: None,
                server: Server::new(),
            },
        }
    }

    pub async fn run(&mut self) {
        while !self.stack.is_empty() {
            self.context.server.tick();

            // We only run update for the state on top of the stack
            let transition = self.stack.last_mut().unwrap().update(&mut self.context);

            // Shadow update all the remaining states
            for i in 0..self.stack.len() - 1 {
                self.stack[i].shadow_update(&mut self.context);
            }

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
                self.stack[i].draw(&self.context);
            }

            match transition {
                Transition::Push(new_state) => {
                    // Pause the current state, push the new one, and start it
                    self.stack.last_mut().unwrap().on_pause(&mut self.context);
                    self.stack.push(new_state);
                    self.stack.last_mut().unwrap().on_start(&mut self.context);
                }
                Transition::PopUntil(target_id) => {
                    let only_overlay = self.pop_until(target_id);

                    // After the loop, there is at least 1 element on the stack, and the top
                    // is our target.
                    self.stack
                        .last_mut()
                        .unwrap()
                        .on_resume(&mut self.context, only_overlay);
                }
                Transition::PopUntilAnd(target_id, new_view) => {
                    // TODO: perhaps we should call on_resume here sometimes
                    let _ = self.pop_until(target_id);
                    self.stack.push(new_view);
                    self.stack.last_mut().unwrap().on_start(&mut self.context);
                }
                Transition::Pop => {
                    let from_overlay = self.stack.last_mut().unwrap().is_overlay();
                    self.stack.pop();

                    if let Some(new_top) = self.stack.last_mut() {
                        // If there is something left on the stack, we should resume it.
                        new_top.as_mut().on_resume(&mut self.context, from_overlay);
                    }
                }
                Transition::PopAnd(new_view) => {
                    self.stack.pop();
                    self.stack.push(new_view);
                    self.stack.last_mut().unwrap().on_start(&mut self.context);
                }
                // For a connection lost transition, we want to return to the ServerConnectMenu
                Transition::ConnectionLost(err) => {
                    self.context.server.close();

                    let _ = self.pop_until(ViewId::ServerConnectMenu);
                    self.stack.push(Box::new(Popup::new(err)));

                    self.stack.last_mut().unwrap().on_start(&mut self.context);
                }
                Transition::None => {}
            }

            next_frame().await;
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
