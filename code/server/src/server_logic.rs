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


