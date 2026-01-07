use std::collections::HashMap;

use common::protocol::{
    API_VERSION, ApiVersion, ClientMessage, CreateGameResponse, HandshakeResponse,
    JoinGameResponse, ServerMessage,
};
use renet::ClientId;
use tracing::debug;

use crate::client::{Client, ClientState};
use crate::game_manager::GameManager;

pub const MAX_CLIENTS: usize = 64;

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
            let response = self.handle_handshake(client_id, api_version, nickname)?;
            return Ok(Some(ServerMessage::HandshakeResponse(response)));
        }

        // Handle other messages
        let client = self.clients.get_mut(&client_id).ok_or("Unknown sender")?;

        let (response, new_state) = match (&client.state, message) {
            (ClientState::Lobby, ClientMessage::CreateGame { map, rounds }) => {
                let response = self.game_manager.create_game(
                    client_id,
                    client.nickname.clone(),
                    map,
                    rounds,
                )?;

                let new_state = match &response {
                    CreateGameResponse::Ok(info) => Some(ClientState::InGame {
                        game_code: info.game_code.clone(),
                        player_id: info.player_id,
                    }),
                    _ => None,
                };

                (Some(ServerMessage::CreateGameReponse(response)), new_state)
            }

            (ClientState::Lobby, ClientMessage::JoinGame { game_code }) => {
                let response =
                    self.game_manager
                        .join_game(&game_code, client_id, client.nickname.clone());

                let new_state = match &response {
                    JoinGameResponse::Ok(info) => Some(ClientState::InGame {
                        game_code: info.game_code.clone(),
                        player_id: info.player_id,
                    }),
                    _ => None,
                };

                (Some(ServerMessage::JoinGameResponse(response)), new_state)
            }

            (ClientState::InGame { game_code, .. }, msg) => match msg {
                ClientMessage::LeaveGame => {
                    self.game_manager.leave_game(game_code, client_id)?;
                    (Some(ServerMessage::LeaveGameAck), Some(ClientState::Lobby))
                }
                ClientMessage::StartCountdown => {
                    let response = self.game_manager.start_countdown(game_code, client_id)?;
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
    ) -> Result<HandshakeResponse, String> {
        if api_version != API_VERSION {
            debug!(%client_id, ?api_version, "Handshake failed: API version mismatch");
            return Ok(HandshakeResponse::ApiMismatch);
        }

        if self.clients.contains_key(&client_id) {
            return Err("Client already connected".to_string());
        }

        if self.clients.len() >= MAX_CLIENTS {
            debug!(%client_id, "Handshake failed: server full");
            return Ok(HandshakeResponse::ServerFull);
        }

        self.clients.insert(
            client_id,
            Client {
                nickname,
                state: ClientState::Lobby,
            },
        );

        Ok(HandshakeResponse::Ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::protocol::{
        ClientMessage, CreateGameResponse, GameCode, HandshakeResponse, JoinGameResponse, MapName,
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
            ServerMessage::CreateGameReponse(CreateGameResponse::Ok(info)) => {
                (info.game_code, info.player_id)
            }
            _ => unreachable!("CreateGame must return CreateGameResponse::Ok in this test setup"),
        }
    }

    fn join_game(logic: &mut ServerLogic, client_id: ClientId, game_code: GameCode) -> PlayerId {
        let resp = logic
            .handle_message(client_id, ClientMessage::JoinGame { game_code })
            .unwrap()
            .expect("join_game should return a response");

        match resp {
            ServerMessage::JoinGameResponse(JoinGameResponse::Ok(info)) => info.player_id,
            _ => unreachable!("JoinGame must return JoinGameResponse::Ok(_) in this test setup"),
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
            ServerMessage::HandshakeResponse(HandshakeResponse::ApiMismatch)
        ));
        assert!(logic.client_state(client_id).is_none());
    }

    #[test]
    fn handshake_duplicate_client_rejected() {
        let mut logic = ServerLogic::new();
        let client_id: ClientId = 1;

        let _ = handshake(&mut logic, client_id, "marcin");
        let err = logic
            .handle_message(
                client_id,
                ClientMessage::Handshake {
                    api_version: API_VERSION,
                    nickname: "marcin2".to_string(),
                },
            )
            .unwrap_err();
        assert!(err.contains("Client already connected"));
    }

    #[test]
    fn create_game_from_lobby_sets_state_in_game() {
        let mut logic = ServerLogic::new();
        let client_id: ClientId = 1;

        let _ = handshake(&mut logic, client_id, "host");
        let (game_code, _player_id) = create_game(&mut logic, client_id);

        assert!(
            matches!(logic.client_state(client_id), Some(ClientState::InGame { game_code: gc, .. }) if gc == game_code)
        );
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
        let err = logic
            .handle_message(joiner_id, ClientMessage::StartCountdown)
            .unwrap_err();
        assert!(err.contains("Only the game master"));

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
            ServerMessage::JoinGameResponse(JoinGameResponse::GameStarted)
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
        assert_eq!(logic.game_manager_mut().games.len(), 1);

        logic.on_disconnect(joiner_id);
        assert!(logic.client_state(joiner_id).is_none());
        assert_eq!(logic.game_manager_mut().games.len(), 0);
    }

    #[test]
    fn accessors_return_internal_state() {
        let mut logic = ServerLogic::new();
        let _ = handshake(&mut logic, 1, "p1");

        assert!(!logic.clients().is_empty());
        // Just ensure it's callable and consistent.
        assert_eq!(
            logic.game_manager().games.len(),
            logic.game_manager_mut().games.len()
        );
    }

    #[test]
    fn non_handshake_from_unknown_sender_is_error() {
        let mut logic = ServerLogic::new();
        let err = logic
            .handle_message(
                123,
                ClientMessage::CreateGame {
                    map: MapName::Basic,
                    rounds: 3,
                },
            )
            .unwrap_err();
        assert!(err.contains("Unknown sender"));
    }

    #[test]
    fn disconnect_logs_when_remove_player_fails() {
        let mut logic = ServerLogic::new();
        let client_id: ClientId = 1;

        let _ = handshake(&mut logic, client_id, "p1");

        // Corrupt client state: says "InGame", but the referenced game doesn't exist.
        logic.clients.get_mut(&client_id).unwrap().state = ClientState::InGame {
            game_code: GameCode("0000".to_string()),
            player_id: 0,
        };

        // Should hit the Err path in on_disconnect (and still remove the client).
        logic.on_disconnect(client_id);
        assert!(logic.client_state(client_id).is_none());
    }

    #[test]
    fn leave_game_success_moves_client_back_to_lobby() {
        let mut logic = ServerLogic::new();
        let host_id: ClientId = 1;

        let _ = handshake(&mut logic, host_id, "host");
        let _ = create_game(&mut logic, host_id);

        let resp = logic
            .handle_message(host_id, ClientMessage::LeaveGame)
            .unwrap()
            .expect("LeaveGame returns a response");

        assert!(matches!(resp, ServerMessage::LeaveGameAck));
        assert!(matches!(
            logic.client_state(host_id),
            Some(ClientState::Lobby)
        ));
    }

    #[test]
    fn leave_game_error_does_not_change_state() {
        let mut logic = ServerLogic::new();
        let host_id: ClientId = 1;
        let other_id: ClientId = 2;

        let _ = handshake(&mut logic, host_id, "host");
        let _ = handshake(&mut logic, other_id, "other");
        let (game_code, _pid) = create_game(&mut logic, host_id);

        // Corrupt other client into InGame without actually being in the game's player list.
        logic.clients.get_mut(&other_id).unwrap().state = ClientState::InGame {
            game_code,
            player_id: 0,
        };

        let err = logic
            .handle_message(other_id, ClientMessage::LeaveGame)
            .unwrap_err();
        assert!(err.contains("Player not found"));
        assert!(matches!(
            logic.client_state(other_id),
            Some(ClientState::InGame { .. })
        ));
    }

    #[test]
    fn invalid_message_in_game_is_error() {
        let mut logic = ServerLogic::new();
        let host_id: ClientId = 1;

        let _ = handshake(&mut logic, host_id, "host");
        let (_game_code, _pid) = create_game(&mut logic, host_id);

        let err = logic
            .handle_message(
                host_id,
                ClientMessage::JoinGame {
                    game_code: GameCode("1234".to_string()),
                },
            )
            .unwrap_err();
        assert!(err.contains("Invalid message"));
    }
}
