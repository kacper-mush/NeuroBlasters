use crate::app::request_view::RequestView;
use crate::app::server_lobby::ServerLobby;
use crate::app::{AppContext, Transition, View, ViewId};
use crate::server::ClientState;
use crate::ui::{
    BUTTON_H, BUTTON_W, Button, CANONICAL_SCREEN_MID_X, Layout, TEXT_MID, Text, TextField,
};

#[derive(Copy, Clone)]
enum ServerConnectButtons {
    Connect,
    Back,
}

pub(crate) struct ServerConnectMenu {
    button_pressed: Option<ServerConnectButtons>,
    servername_field: TextField,
    username_field: TextField,
}

impl ServerConnectMenu {
    pub fn new() -> Self {
        ServerConnectMenu {
            button_pressed: None,
            servername_field: TextField::new_simple(30),
            username_field: TextField::new_simple(20),
        }
    }
}

impl View for ServerConnectMenu {
    fn draw(&mut self, _ctx: &AppContext) {
        let x_mid = CANONICAL_SCREEN_MID_X;
        let el_w = BUTTON_W;
        let el_h = BUTTON_H;
        let mut layout = Layout::new(100., 30.);

        Text::new_title().draw("Connect to server", x_mid, layout.next());
        layout.add(70.);

        Text::new_scaled(TEXT_MID).draw("Enter server name:", x_mid, layout.next());
        layout.add(20.);

        self.servername_field
            .draw_centered(x_mid, layout.next(), el_w, el_h);
        layout.add(el_h - 10.);

        Text::new_scaled(TEXT_MID).draw("Enter username:", x_mid, layout.next());
        layout.add(20.);

        self.username_field
            .draw_centered(x_mid, layout.next(), el_w, el_h);
        layout.add(el_h);

        self.button_pressed = None;

        if Button::default()
            .draw_centered(x_mid, layout.next(), el_w, el_h, Some("Connect"))
            .poll()
        {
            self.button_pressed = Some(ServerConnectButtons::Connect);
        }
        layout.add(el_h);

        if Button::default()
            .draw_centered(x_mid, layout.next(), el_w, el_h, Some("Back"))
            .poll()
        {
            self.button_pressed = Some(ServerConnectButtons::Back);
        }
    }

    fn update(&mut self, ctx: &mut AppContext) -> Transition {
        self.servername_field.update();
        self.username_field.update();

        match &ctx.server.client_state {
            ClientState::Disconnected => {}
            _ => panic!("Invalid server state for the server connect view."),
        }

        match self.button_pressed {
            Some(button) => match button {
                ServerConnectButtons::Connect => {
                    ctx.server
                        .connect(self.servername_field.text(), self.username_field.text());
                    let success_view = Some(Box::new(ServerLobby::new()) as Box<dyn View>);
                    Transition::Push(Box::new(RequestView::new(
                        "Connecting to server...".into(),
                        success_view,
                    )))
                }
                ServerConnectButtons::Back => Transition::Pop,
            },
            None => Transition::None,
        }
    }

    fn get_id(&self) -> ViewId {
        ViewId::ServerConnectMenu
    }
}
