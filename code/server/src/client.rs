use common::protocol::{GameCode, PlayerId};

#[derive(Clone, Debug)]
pub enum ClientState {
    Lobby,
    InGame {
        game_code: GameCode,
        #[allow(dead_code)] // Might be used in the future.
        player_id: PlayerId,
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
