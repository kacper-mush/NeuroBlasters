use crate::app::game::Game;
use crate::app::{AppContext, Transition, View, ViewId};
use crate::ui::{Button, Field, Text};
use macroquad::prelude::*;

#[derive(Clone, Copy)]
enum RoomLobbyButtons {
    Start,
    Leave,
}

pub(crate) struct RoomLobby {
    button_pressed: Option<RoomLobbyButtons>,
    room_code: u32,
    player_names: Vec<String>,
}

impl RoomLobby {
    pub fn new() -> Self {
        Self {
            button_pressed: None,
            room_code: 0,
            player_names: Vec::new(),
        }
    }
}

impl View for RoomLobby {
    fn draw(&mut self, _ctx: &AppContext) {
        let x_mid = screen_width() / 2.;
        let mut button = Button::new(Field::default(), Some(TextParams::default()));
        let w = 300.;
        let h = 50.;
        let y_start = 200.;
        let sep = 40.;
        let mut offset = 0.;

        let title = format!("Room: {}", self.room_code);
        Text::new_simple(40).draw(&title, x_mid, y_start + offset);
        offset += sep;
        Text::new_simple(30).draw("Players:", x_mid, y_start + offset);
        offset += sep;
        for name in &self.player_names {
            Text::new_simple(25).draw(name, x_mid, y_start + offset);
            offset += sep;
        }

        self.button_pressed = None;

        if button
            .draw_centered(x_mid, y_start + offset, w, h, Some("Start"))
            .poll()
        {
            self.button_pressed = Some(RoomLobbyButtons::Start);
        }
        offset += 100.;
        if button
            .draw_centered(x_mid, y_start + offset, w, h, Some("Back"))
            .poll()
        {
            self.button_pressed = Some(RoomLobbyButtons::Leave);
        }
    }

    fn update(&mut self, ctx: &mut AppContext) -> Transition {
        if ctx.server.is_none() {
            return Transition::ConnectionLost;
        }

        let server = ctx.server.as_mut().unwrap();

        if server.game_started() {
            let game = Game::try_new(ctx);
            match game {
                Some(game) => return Transition::Push(Box::new(game)),
                None => return Transition::ConnectionLost,
            }
        }

        self.room_code = match server.get_room_code() {
            Ok(code) => code,
            Err(_) => return Transition::Pop,
        };

        self.player_names = match server.get_player_list() {
            Ok(player_list) => player_list,
            Err(_) => return Transition::Pop,
        };

        match self.button_pressed {
            Some(button) => match button {
                RoomLobbyButtons::Leave => {
                    server.leave();
                    Transition::Pop
                }
                RoomLobbyButtons::Start => {
                    if server.start_game() {
                        let game = Game::try_new(ctx);
                        match game {
                            Some(game) => return Transition::Push(Box::new(game)),
                            None => return Transition::ConnectionLost,
                        }
                    }

                    Transition::None
                }
            },
            None => Transition::None,
        }
    }

    fn get_id(&self) -> ViewId {
        ViewId::RoomLobby
    }
}
