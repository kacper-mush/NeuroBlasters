use common::protocol::GameCode;

#[derive(Clone, Debug)]
pub enum ClientState {
    Lobby,
    InGame {
        game_code: GameCode,
    },
}

pub struct Client {
    pub nickname: String,
    pub state: ClientState,
}

impl Default for Client {
    fn default() -> Self {
        Self {
            nickname: String::new(),
            state: ClientState::Lobby,
        }
    }
}