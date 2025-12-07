use crate::app::room_lobby::RoomLobby;
use crate::app::{AppContext, Transition, View, ViewId};
use crate::ui::{Button, Field, Text, TextField};
use macroquad::prelude::*;

#[derive(Clone, Copy)]
enum RoomMenuButtons {
    Create,
    Join,
    Back,
}

pub(crate) struct RoomMenu {
    button_pressed: Option<RoomMenuButtons>,
    room_code_field: TextField,
    message: Option<String>,
}

impl RoomMenu {
    pub fn new() -> Self {
        RoomMenu {
            button_pressed: None,
            room_code_field: TextField::new(Field::default(), TextParams::default(), 10),
            message: None,
        }
    }
}

impl View for RoomMenu {
    fn draw(&mut self, _ctx: &AppContext) {
        let x_mid = screen_width() / 2.;
        let mut button = Button::new(Field::default(), Some(TextParams::default()));
        let w = 300.;
        let h = 50.;
        let y_start = 270.;
        let sep = 80.;

        self.button_pressed = None;

        Text::new_simple(30).draw("Rooms", x_mid, 200.);
        if button
            .draw_centered(x_mid, y_start, w, h, Some("Create"))
            .poll()
        {
            self.button_pressed = Some(RoomMenuButtons::Create);
        }

        Text::new_simple(30).draw("Room code:", x_mid, y_start + sep);

        self.room_code_field
            .draw_centered(x_mid, y_start + 2. * sep, w, h);

        if button
            .draw_centered(x_mid, y_start + 3. * sep, w, h, Some("Join"))
            .poll()
        {
            self.button_pressed = Some(RoomMenuButtons::Join);
        }

        if button
            .draw_centered(x_mid, y_start + 4. * sep, w, h, Some("Back"))
            .poll()
        {
            self.button_pressed = Some(RoomMenuButtons::Back);
        }

        if let Some(message) = self.message.as_ref() {
            Text::new_simple(30).draw(message, x_mid, y_start + 5. * sep);
        }
    }

    fn update(&mut self, ctx: &mut AppContext) -> Transition {
        self.room_code_field.update();

        if ctx.server.is_none() {
            return Transition::ConnectionLost;
        }

        let server = ctx.server.as_ref().unwrap();

        match self.button_pressed {
            Some(button) => match button {
                RoomMenuButtons::Create => {
                    if server.create_room() {
                        Transition::Push(Box::new(RoomLobby::new()))
                    } else {
                        self.message = Some("Could not create the room!".into());
                        Transition::None
                    }
                }
                RoomMenuButtons::Join => {
                    let room_code = self.room_code_field.text().parse::<u32>();
                    if room_code.is_err() {
                        self.message = Some("Invalid room code!".into());
                        return Transition::None;
                    }

                    if server.join_room(room_code.unwrap()) {
                        return Transition::Push(Box::new(RoomLobby::new()));
                    }

                    self.message = Some("Could not join the room!".into());
                    Transition::None
                }
                RoomMenuButtons::Back => Transition::Pop,
            },
            None => Transition::None,
        }
    }

    fn on_resume(&mut self, _ctx: &mut AppContext, from_overlay: bool) {
        if !from_overlay {
            self.message = None;
            self.room_code_field.reset();
        }
    }

    fn get_id(&self) -> ViewId {
        ViewId::RoomMenu
    }
}
