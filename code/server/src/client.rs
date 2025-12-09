use common::protocol::GameCode;

#[derive(Clone, Debug, Default)]
pub enum ClientState {
    #[default]
    Handshaking,
    Lobby {
        nickname: String,
    },
    InGame {
        game_code: GameCode,
    },
}
