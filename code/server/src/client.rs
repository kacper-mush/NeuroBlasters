use common::protocol::GameCode;

#[derive(Clone, Debug)]
pub enum ClientState {
    Handshaking,
    Lobby {
        nickname: String
    },
    InGame {
        nickname: String,
        game_code: GameCode,
    },
}

impl Default for ClientState {
    fn default() -> Self {
        Self::Handshaking
    }
}

// impl Client {
//     pub fn new(nickname: String) -> Self {
//         nickname,
//         state: ClientState::Lobby,
//     }

//     pub fn handle_message(&mut self, message: ClientMessage, game_manager: &mut GameManager) {
//         match (self.state, message) {
//             (ClientState::Lobby, ClientMessage::CreateGame) => {
//                 let game_code = game_manager.create_game(self.id);
//             }
//         }
//     }
// }

// impl ClientState {
//     pub fn apply(self, event: ClientEvent) -> Self {
//         match (self, event) {
//             (ClientState::Handshaking, ClientEvent::HandshakeCompleted { nickname }) => {
//                 ClientState::Lobby { nickname }
//             }
//             (ClientState::Lobby { nickname }, ClientEvent::GameJoined { game_code }) 
//             | (ClientState::Lobby { nickname }, ClientEvent::GameCreated { game_code }) => {
//                 ClientState::InGame { nickname, game_code }
//             }
//             (ClientState::InGame { nickname, .. }, ClientEvent::GameLeft) => {
//                 ClientState::Lobby { nickname }
//             }
//             (state, event) => {
//                 panic!("Invalid transition in Client state machine: {:?} -> {:?}", state, event);
//             }
//         }
//     }
// }

// pub fn handle_message(
//     state: &ClientState,
//     msg: ClientMessage,
//     gm: &mut GameManager,
//     my_id: common::protocol::ClientId,
// ) -> Result<Option<ClientEvent>, String> {
//     match (state, msg) {
//         (ClientState::Handshaking, ClientMessage::Handshake { nickname, .. }) => {
//             Ok(Some(ClientEvent::HandshakeCompleted { nickname }))
//         }

//         (ClientState::Lobby { .. }, ClientMessage::CreateGame) => {
//             let code = gm.create_game(my_id);
//             Ok(Some(ClientEvent::GameCreated { game_code: code }))
//         }

//         (ClientState::Lobby { .. }, ClientMessage::JoinGame { game_code }) => {
//             let code = GameCode(game_code);
//             gm.join_game(&code, my_id)?;
//             Ok(Some(ClientEvent::GameJoined { game_code: code }))
//         }

//         (ClientState::InGame { game_code, .. }, ClientMessage::LeaveGame) => {
//             gm.leave_game(game_code.clone(), my_id)?;
//             Ok(Some(ClientEvent::GameLeft))
//         }

//         (ClientState::Handshaking, _) => Err("You must handshake first.".to_string()),
//         (ClientState::InGame { .. }, ClientMessage::JoinGame { .. }) => Err("Already in a game.".to_string()),
//         (ClientState::Lobby { .. }, ClientMessage::LeaveGame) => Err("Not in a game.".to_string()),
        
//         _ => Err("Command not supported in this state.".to_string()),
//     }
// }