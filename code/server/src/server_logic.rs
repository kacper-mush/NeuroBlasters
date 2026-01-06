use std::collections::HashMap;

use common::protocol::{
    API_VERSION, ApiVersion, ClientMessage, CrateGameReponse, HandshakeResponse, JoinGameResponse,
    LeaveGameResponse, ServerMessage,
};
use renet::ClientId;
use tracing::debug;

use crate::client::{Client, ClientState};
use crate::game_manager::GameManager;

pub struct ServerLogic {
    clients: HashMap<ClientId, Client>,
    game_manager: GameManager,
}

impl ServerLogic {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            game_manager: GameManager::new(),
        }
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub fn clients(&self) -> &HashMap<ClientId, Client> {
        &self.clients
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub fn game_manager(&self) -> &GameManager {
        &self.game_manager
    }

    pub fn game_manager_mut(&mut self) -> &mut GameManager {
        &mut self.game_manager
    }

    pub fn remove_client(&mut self, client_id: ClientId) {
        self.clients.remove(&client_id);
    }

    pub fn client_state(&self, client_id: ClientId) -> Option<ClientState> {
        self.clients.get(&client_id).map(|c| c.state.clone())
    }

    pub fn on_disconnect(&mut self, client_id: ClientId) {
        // If the client was in a game, remove them from the game.
        if let Some(ClientState::InGame { game_code, .. }) = self.client_state(client_id)
            && let Err(e) = self.game_manager.remove_player(&game_code, client_id)
        {
            debug!(%client_id, %e, "Failed to remove player from game");
        }
        self.remove_client(client_id);
    }

    pub fn handle_message(
        &mut self,
        client_id: ClientId,
        message: ClientMessage,
    ) -> Result<Option<ServerMessage>, String> {
        // Handle handshake
        if let ClientMessage::Handshake {
            api_version,
            nickname,
        } = message
        {
            return Ok(Some(ServerMessage::HandshakeResponse(
                self.handle_handshake(client_id, api_version, nickname),
            )));
        }

        // Handle other messages
        let client = self.clients.get_mut(&client_id).ok_or("Unknown sender")?;

        let (response, new_state) = match (&client.state, message) {
            (ClientState::Lobby, ClientMessage::CreateGame { map, rounds }) => {
                let response =
                    self.game_manager
                        .create_game(client_id, client.nickname.clone(), map, rounds);

                let new_state = match &response {
                    CrateGameReponse::Ok {
                        game_code,
                        player_id,
                    } => Some(ClientState::InGame {
                        game_code: game_code.clone(),
                        player_id: *player_id,
                    }),
                    CrateGameReponse::Error(_) => None,
                };

                (Some(ServerMessage::CrateGameReponse(response)), new_state)
            }

            (ClientState::Lobby, ClientMessage::JoinGame { game_code }) => {
                let response =
                    self.game_manager
                        .join_game(&game_code, client_id, client.nickname.clone());

                let new_state = match response {
                    JoinGameResponse::Ok { player_id } => Some(ClientState::InGame {
                        game_code,
                        player_id,
                    }),
                    JoinGameResponse::Error(_) => None,
                };

                (Some(ServerMessage::JoinGameResponse(response)), new_state)
            }

            (ClientState::InGame { game_code, .. }, msg) => match msg {
                ClientMessage::LeaveGame => {
                    let response = self.game_manager.leave_game(game_code, client_id);
                    let new_state = match response {
                        LeaveGameResponse::Ok => Some(ClientState::Lobby),
                        LeaveGameResponse::Error(_) => None,
                    };

                    (Some(ServerMessage::LeaveGameResponse(response)), new_state)
                }
                ClientMessage::StartCountdown => {
                    let response = self.game_manager.start_countdown(game_code, client_id);
                    (Some(ServerMessage::StartCountdownResponse(response)), None)
                }
                ClientMessage::GameInput(input) => {
                    self.game_manager
                        .submit_input(game_code, client_id, input)?;
                    (None, None)
                }
                _ => return Err("Invalid message in current state".to_string()),
            },
            (_, _) => return Err("Invalid message in current state".to_string()),
        };

        if let Some(s) = new_state {
            client.state = s;
        }

        Ok(response)
    }

    fn handle_handshake(
        &mut self,
        client_id: ClientId,
        api_version: ApiVersion,
        nickname: String,
    ) -> HandshakeResponse {
        if api_version != API_VERSION {
            return HandshakeResponse::Error("Api version mismatch".to_string());
        }

        if self.clients.contains_key(&client_id) {
            return HandshakeResponse::Error("Client already connected".to_string());
        }

        self.clients.insert(
            client_id,
            Client {
                nickname,
                state: ClientState::Lobby,
            },
        );

        HandshakeResponse::Ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::protocol::{
        ClientMessage, CrateGameReponse, GameCode, HandshakeResponse, JoinGameResponse, MapName,
        PlayerId, ServerMessage, StartCountdownResponse,
    };
    use glam::Vec2;

    fn handshake(logic: &mut ServerLogic, client_id: ClientId, nickname: &str) -> ServerMessage {
        logic
            .handle_message(
                client_id,
                ClientMessage::Handshake {
                    api_version: API_VERSION,
                    nickname: nickname.to_string(),
                },
            )
            .unwrap()
            .expect("handshake should always return a response")
    }

    fn create_game(logic: &mut ServerLogic, client_id: ClientId) -> (GameCode, PlayerId) {
        let resp = logic
            .handle_message(
                client_id,
                ClientMessage::CreateGame {
                    map: MapName::Basic,
                    rounds: 3,
                },
            )
            .unwrap()
            .expect("create_game should return a response");

        match resp {
            ServerMessage::CrateGameReponse(CrateGameReponse::Ok {
                game_code,
                player_id,
            }) => (game_code, player_id),
            other => panic!("unexpected create_game response: {other:?}"),
        }
    }

    fn join_game(logic: &mut ServerLogic, client_id: ClientId, game_code: GameCode) -> PlayerId {
        let resp = logic
            .handle_message(client_id, ClientMessage::JoinGame { game_code })
            .unwrap()
            .expect("join_game should return a response");

        match resp {
            ServerMessage::JoinGameResponse(JoinGameResponse::Ok { player_id }) => player_id,
            other => panic!("unexpected join_game response: {other:?}"),
        }
    }

    #[test]
    fn handshake_ok_creates_client_in_lobby() {
        let mut logic = ServerLogic::new();
        let client_id: ClientId = 1;

        let resp = handshake(&mut logic, client_id, "marcin");
        assert!(matches!(
            resp,
            ServerMessage::HandshakeResponse(HandshakeResponse::Ok)
        ));
        assert!(matches!(
            logic.client_state(client_id),
            Some(ClientState::Lobby)
        ));
    }

    #[test]
    fn handshake_wrong_api_version_rejected() {
        let mut logic = ServerLogic::new();
        let client_id: ClientId = 1;

        let resp = logic
            .handle_message(
                client_id,
                ClientMessage::Handshake {
                    api_version: API_VERSION + 1,
                    nickname: "marcin".to_string(),
                },
            )
            .unwrap()
            .expect("handshake should return a response");

        assert!(matches!(
            resp,
            ServerMessage::HandshakeResponse(HandshakeResponse::Error(_))
        ));
        assert!(logic.client_state(client_id).is_none());
    }

    #[test]
    fn handshake_duplicate_client_rejected() {
        let mut logic = ServerLogic::new();
        let client_id: ClientId = 1;

        let _ = handshake(&mut logic, client_id, "marcin");
        let resp = handshake(&mut logic, client_id, "marcin2");

        assert!(matches!(
            resp,
            ServerMessage::HandshakeResponse(HandshakeResponse::Error(_))
        ));
    }

    #[test]
    fn create_game_from_lobby_sets_state_in_game() {
        let mut logic = ServerLogic::new();
        let client_id: ClientId = 1;

        let _ = handshake(&mut logic, client_id, "host");
        let (game_code, _player_id) = create_game(&mut logic, client_id);

        match logic.client_state(client_id) {
            Some(ClientState::InGame { game_code: gc, .. }) => assert_eq!(gc, game_code),
            other => panic!("expected InGame state, got: {other:?}"),
        }
    }

    #[test]
    fn join_game_from_lobby_sets_state_in_game() {
        let mut logic = ServerLogic::new();
        let host_id: ClientId = 1;
        let joiner_id: ClientId = 2;

        let _ = handshake(&mut logic, host_id, "host");
        let _ = handshake(&mut logic, joiner_id, "joiner");

        let (game_code, _host_player_id) = create_game(&mut logic, host_id);
        let _joiner_player_id = join_game(&mut logic, joiner_id, game_code.clone());

        assert!(matches!(
            logic.client_state(joiner_id),
            Some(ClientState::InGame { game_code: gc, .. }) if gc == game_code
        ));
    }

    #[test]
    fn invalid_message_in_lobby_is_error() {
        let mut logic = ServerLogic::new();
        let client_id: ClientId = 1;

        let _ = handshake(&mut logic, client_id, "p1");
        let err = logic
            .handle_message(client_id, ClientMessage::StartCountdown)
            .unwrap_err();
        assert!(err.contains("Invalid message"));
    }

    #[test]
    fn start_countdown_requires_master_and_two_players() {
        let mut logic = ServerLogic::new();
        let host_id: ClientId = 1;
        let joiner_id: ClientId = 2;

        let _ = handshake(&mut logic, host_id, "host");
        let _ = handshake(&mut logic, joiner_id, "joiner");

        let (game_code, _host_player_id) = create_game(&mut logic, host_id);
        let _ = join_game(&mut logic, joiner_id, game_code.clone());

        // Non-master should be rejected.
        let resp = logic
            .handle_message(joiner_id, ClientMessage::StartCountdown)
            .unwrap()
            .expect("start_countdown returns a response");
        assert!(matches!(
            resp,
            ServerMessage::StartCountdownResponse(StartCountdownResponse::Error(_))
        ));

        // Master should be accepted.
        let resp = logic
            .handle_message(host_id, ClientMessage::StartCountdown)
            .unwrap()
            .expect("start_countdown returns a response");
        assert!(matches!(
            resp,
            ServerMessage::StartCountdownResponse(StartCountdownResponse::Ok)
        ));
    }

    #[test]
    fn join_game_rejected_when_game_not_in_lobby_state() {
        let mut logic = ServerLogic::new();
        let host_id: ClientId = 1;
        let joiner_id: ClientId = 2;
        let late_id: ClientId = 3;

        let _ = handshake(&mut logic, host_id, "host");
        let _ = handshake(&mut logic, joiner_id, "joiner");
        let _ = handshake(&mut logic, late_id, "late");

        let (game_code, _host_player_id) = create_game(&mut logic, host_id);
        let _ = join_game(&mut logic, joiner_id, game_code.clone());

        // Start countdown (transition out of lobby).
        let _ = logic
            .handle_message(host_id, ClientMessage::StartCountdown)
            .unwrap();

        // Late join should be rejected.
        let resp = logic
            .handle_message(late_id, ClientMessage::JoinGame { game_code })
            .unwrap()
            .expect("join_game returns a response");

        assert!(matches!(
            resp,
            ServerMessage::JoinGameResponse(JoinGameResponse::Error(_))
        ));
    }

    #[test]
    fn game_input_in_game_returns_no_response() {
        let mut logic = ServerLogic::new();
        let host_id: ClientId = 1;

        let _ = handshake(&mut logic, host_id, "host");
        let (_game_code, _host_player_id) = create_game(&mut logic, host_id);

        let input = common::protocol::InputPayload {
            move_axis: Vec2::ZERO,
            aim_pos: Vec2::ZERO,
            shoot: true,
        };

        let resp = logic
            .handle_message(host_id, ClientMessage::GameInput(input))
            .unwrap();
        assert!(resp.is_none());
    }

    #[test]
    fn disconnect_removes_client_and_removes_game_when_last_player_leaves() {
        let mut logic = ServerLogic::new();
        let host_id: ClientId = 1;
        let joiner_id: ClientId = 2;

        let _ = handshake(&mut logic, host_id, "host");
        let _ = handshake(&mut logic, joiner_id, "joiner");

        let (game_code, _host_player_id) = create_game(&mut logic, host_id);
        let _ = join_game(&mut logic, joiner_id, game_code.clone());

        assert_eq!(logic.game_manager_mut().games.len(), 1);

        logic.on_disconnect(host_id);
        assert!(logic.client_state(host_id).is_none());
        assert_eq!(
            logic.game_manager_mut().games.len(),
            1,
            "game should remain with 1 player"
        );

        logic.on_disconnect(joiner_id);
        assert!(logic.client_state(joiner_id).is_none());
        assert_eq!(
            logic.game_manager_mut().games.len(),
            0,
            "game should be removed when empty"
        );
    }
}
