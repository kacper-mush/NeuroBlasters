use common::protocol::GameCode;

#[derive(Clone, Debug)]
pub enum ClientState {
    Handshaking,
    Lobby { nickname: String },
    InGame { game_code: GameCode },
}

impl Default for ClientState {
    fn default() -> Self {
        Self::Handshaking
    }
}
