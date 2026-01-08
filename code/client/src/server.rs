use std::net::ToSocketAddrs;

use common::protocol::{
    API_VERSION, CreateGameResponse, GameUpdate, HandshakeResponse, InitialGameInfo,
    JoinGameResponse,
};
use common::{
    codec::{decode_server_message, encode_client_message},
    game::player::is_valid_username,
    protocol::{ClientMessage, ServerMessage},
};
use rand::Rng;
use std::sync::mpsc::Receiver;

use std::net::UdpSocket;
use std::time::{Duration, Instant, SystemTime};

use renet::{ClientId, ConnectionConfig, DisconnectReason, RenetClient};
use renet_netcode::{
    ClientAuthentication, NetcodeClientTransport, NetcodeDisconnectReason, NetcodeError,
    NetcodeTransportError,
};

/// Represents in what state of the communication the client is
#[derive(Debug, PartialEq)]
pub(crate) enum ClientState {
    /// Initial state
    Disconnected,
    /// Connected to the server, can create / join games
    Connected,
    /// In a game
    Playing,
}

struct ConnectionData {
    client: RenetClient,
    client_id: ClientId,
    transport: NetcodeClientTransport,
}

pub(crate) struct Server {
    connection_data: Option<ConnectionData>,
    connect_rx: Option<Receiver<Result<ConnectionData, String>>>,
    last_tick: Instant,
    game_update: Option<GameUpdate>,
    initial_game_info: Option<InitialGameInfo>,
    client_state: ClientState,
    /// If request failed, the client can check why
    request_response: Option<Result<(), String>>,
    request_pending: bool,
}

const PROTOCOL_ID: u64 = 0;
const RELIABLE_CHANNEL_ID: u8 = 0;

impl Server {
    pub fn new() -> Self {
        Self {
            connection_data: None,
            connect_rx: None,
            last_tick: Instant::now(),
            game_update: None,
            initial_game_info: None,
            client_state: ClientState::Disconnected,
            request_response: None,
            request_pending: false,
        }
    }

    pub fn connect(&mut self, servername: String, username: String) {
        if self.request_pending || !matches!(self.client_state, ClientState::Disconnected) {
            panic!("Unexpected call to connect.");
        }

        let (tx, rx) = std::sync::mpsc::channel();
        self.connect_rx = Some(rx);

        // The request will be pending for as long as we receive handshake response (or an error occurs earlier)
        self.request_pending = true;

        std::thread::spawn(move || {
            let result = connect_blocking(servername, username);
            let _ = tx.send(result);
        });
    }

    pub fn get_client_id(&self) -> ClientId {
        self.connection_data
            .as_ref()
            .expect("Should not be called when there is no connection.")
            .client_id
    }

    pub fn tick(&mut self) -> Result<(), String> {
        if let Some(rx) = &self.connect_rx {
            // Connecting to server has finished
            if let Ok(result) = rx.try_recv() {
                self.connect_rx = None;

                match result {
                    Ok(connection_data) => {
                        // Succesfully connected, but we are still waiting for handshake response.
                        self.connection_data = Some(connection_data);
                    }
                    Err(reason) => {
                        // Could not connect.
                        self.request_pending = false;
                        self.request_response = Some(Err(reason));
                    }
                }
            }
        }

        let now = Instant::now();
        let dt = now.duration_since(self.last_tick);
        self.last_tick = now;

        self.handle_network(dt)?;
        Ok(())
    }

    fn handle_network(&mut self, dt: Duration) -> Result<(), String> {
        if let Some(mut connection_data) = self.connection_data.take() {
            let result = connection_data
                .transport
                .update(dt, &mut connection_data.client);
            self.handle_net_result(result)?;

            if connection_data.client.is_connected() {
                self.process_server_messages(&mut connection_data)?;

                let result = connection_data
                    .transport
                    .send_packets(&mut connection_data.client);
                self.handle_net_result(result)?;
            }
            self.connection_data = Some(connection_data);
        }
        Ok(())
    }

    fn handle_net_result(
        &mut self,
        result: Result<(), NetcodeTransportError>,
    ) -> Result<(), String> {
        if let Err(transport_error) = result {
            eprintln!("Server errror: {}", transport_error);
            let default = Err("Network connection failed.".into());
            match transport_error {
                NetcodeTransportError::Renet(DisconnectReason::DisconnectedByServer) => {
                    Err("Server closed connection.".into())
                }
                NetcodeTransportError::Netcode(NetcodeError::Disconnected(
                    NetcodeDisconnectReason::DisconnectedByServer,
                )) => Err("Server closed connection.".into()),
                _ => default,
            }
        } else {
            Ok(())
        }
    }

    fn process_server_messages(
        &mut self,
        connection_data: &mut ConnectionData,
    ) -> Result<(), String> {
        while let Some(message) = connection_data.client.receive_message(RELIABLE_CHANNEL_ID) {
            let server_msg = decode_server_message(&message).or(Err("Invalid server message."))?;
            self.client_state = self.process_message(server_msg)?;
        }
        Ok(())
    }

    fn process_message(&mut self, server_msg: ServerMessage) -> Result<ClientState, String> {
        match &self.client_state {
            ClientState::Disconnected => self.handle_disconnected_state(server_msg),
            ClientState::Connected => self.handle_connected_state(server_msg),
            ClientState::Playing => self.handle_playing_state(server_msg),
        }
    }

    fn handle_disconnected_state(
        &mut self,
        server_msg: ServerMessage,
    ) -> Result<ClientState, String> {
        match server_msg {
            ServerMessage::HandshakeResponse(resp) => match resp {
                HandshakeResponse::Ok => self.complete_request(Ok(()), ClientState::Connected),
                HandshakeResponse::ApiMismatch => Err("Server error: API mismatch.".into()),
                HandshakeResponse::ServerFull => Err("Server error: server is full.".into()),
            },

            ServerMessage::Error(error) => {
                Err(format!("Server error while handshaking: {}", error))
            }

            _ => Err("Got invalid server message while handshaking.".into()),
        }
    }

    fn handle_connected_state(&mut self, server_msg: ServerMessage) -> Result<ClientState, String> {
        match server_msg {
            ServerMessage::CreateGameReponse(resp) => match resp {
                CreateGameResponse::Ok(initial_game_info) => {
                    self.complete_request_fn(Ok(()), |server: &mut Server| {
                        server.initial_game_info = Some(initial_game_info);
                        Ok(ClientState::Playing)
                    })
                }
                CreateGameResponse::TooManyGames => self.complete_request(
                    Err("Server game limit exhausted.".into()),
                    ClientState::Connected,
                ),
            },

            ServerMessage::JoinGameResponse(resp) => match resp {
                JoinGameResponse::Ok(initial_game_info) => {
                    self.complete_request_fn(Ok(()), |server: &mut Server| {
                        server.initial_game_info = Some(initial_game_info);
                        Ok(ClientState::Playing)
                    })
                }

                JoinGameResponse::GameFull => {
                    self.complete_request(Err("Game is full.".into()), ClientState::Connected)
                }
                JoinGameResponse::InvalidCode => self.complete_request(
                    Err("Game does not exist (invalid code).".into()),
                    ClientState::Connected,
                ),
                JoinGameResponse::GameStarted => self.complete_request(
                    Err("Game has already started.".into()),
                    ClientState::Connected,
                ),
            },

            ServerMessage::Error(error) => Err(format!("Server errror: {}", error)),

            _ => Err("Got invalid server message.".into()),
        }
    }

    fn handle_playing_state(&mut self, server_msg: ServerMessage) -> Result<ClientState, String> {
        match server_msg {
            ServerMessage::GameUpdate(new_update) => {
                self.game_update = Some(new_update);
                Ok(ClientState::Playing)
            }

            ServerMessage::StartCountdownAck => self.complete_request(Ok(()), ClientState::Playing),

            ServerMessage::LeaveGameAck => self.complete_request(Ok(()), ClientState::Connected),

            ServerMessage::Error(e) => Err(format!(
                "Got error response from server while in game: {}",
                e
            )),

            _ => Err("Got invalid server message while in game.".into()),
        }
    }

    pub fn send_client_message(&mut self, msg: ClientMessage) {
        // Checking if the message we are sending aligns with the state we are in
        match (&self.client_state, &msg) {
            // Trying to create / join a game
            (
                ClientState::Connected,
                ClientMessage::CreateGame { map: _, rounds: _ }
                | ClientMessage::JoinGame { game_code: _ },
            ) => {}

            // Available options in game
            (
                ClientState::Playing,
                ClientMessage::LeaveGame
                | ClientMessage::StartCountdown
                | ClientMessage::GameInput(_),
            ) => {}

            _ => {
                panic!("Invalid message for current state!");
            }
        }

        // All messages are requests besides the GameInput one
        match &msg {
            ClientMessage::GameInput(_) => {}

            _ => {
                if self.request_pending {
                    panic!(
                        "Trying to send another request when the previous one is still pending! Only one request at a time!"
                    )
                }
                self.request_pending = true;
            }
        }

        let payload =
            encode_client_message(&msg).expect("Serializing Client Message should never fail.");
        self.connection_data
            .as_mut()
            .expect("Send should never be called when connection was not yet established")
            .client
            .send_message(RELIABLE_CHANNEL_ID, payload);
    }

    /// Tries to set the request response (if the response is a valid response to some request we made)
    /// on success, returns the success client state
    /// on failure, returns an error state with an appropriate message
    fn complete_request_fn<F: FnOnce(&mut Server) -> Result<ClientState, String>>(
        &mut self,
        response: Result<(), String>,
        success_action: F,
    ) -> Result<ClientState, String> {
        // In both cases there is some unwanted response; the first case is simple, but in the second
        // we have a guarantee from the send_client_message function that we do not make 2 consecutive requests,
        // so this new response must also be at server's fault
        if !self.request_pending || self.request_response.is_some() {
            return Err("Server sent response but no request was made.".into());
        }

        self.request_response = Some(response);
        self.request_pending = false;
        success_action(self)
    }

    fn complete_request(
        &mut self,
        response: Result<(), String>,
        success: ClientState,
    ) -> Result<ClientState, String> {
        self.complete_request_fn(response, |_| Ok(success))
    }

    #[must_use]
    pub fn take_request_response(&mut self) -> Option<Result<(), String>> {
        self.request_response.take()
    }

    pub fn close(&mut self) {
        // Full reset
        *self = Self::new();
    }

    #[must_use]
    pub fn game_update(&mut self) -> Option<GameUpdate> {
        self.game_update.take()
    }

    #[must_use]
    pub fn initial_game_info(&mut self) -> Option<InitialGameInfo> {
        self.initial_game_info.take()
    }

    #[must_use]
    pub fn client_id(&self) -> Option<ClientId> {
        self.connection_data.as_ref().map(|c| c.client_id)
    }

    pub fn assert_state(&self, state: ClientState) {
        if self.client_state != state {
            panic!("Server is in invalid state.");
        }
    }
}

fn connect_blocking(mut servername: String, username: String) -> Result<ConnectionData, String> {
    is_valid_username(&username)?;

    // If no port suffix present, append the 8080 port which is the default for our server
    if !servername.contains(':') {
        servername.push_str(":8080");
    }

    let addrs: Vec<std::net::SocketAddr> = servername
        .to_socket_addrs()
        .map_err(|_| "Server not found.".to_string())?
        .collect();
    let mut addrs = addrs;
    // Prefer IPv4 when both families are available (common for "localhost" resolving to ::1 first on Linux).
    addrs.sort_by_key(|a| if a.is_ipv4() { 0 } else { 1 });
    let server_addr = addrs
        .first()
        .copied()
        .ok_or("Server not found.".to_string())?;

    let connection_config = ConnectionConfig::default();

    let mut client = RenetClient::new(connection_config);

    // Listen on all interfaces on any port, with the appropriate protocol
    let socket = if server_addr.is_ipv4() {
        UdpSocket::bind("0.0.0.0:0")
    } else {
        UdpSocket::bind("[::]:0")
    }
    .or(Err("Could not establish a connection.".to_string()))?;

    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    let client_id = rand::rng().random();
    // No authentication for now
    let authentication = ClientAuthentication::Unsecure {
        client_id,
        protocol_id: PROTOCOL_ID,
        server_addr,
        user_data: None,
    };

    let transport = NetcodeClientTransport::new(current_time, authentication, socket)
        .or(Err("Could not establish a connection."))?;

    // Send handshake as the final step. User will wait for server response.
    let payload = encode_client_message(&ClientMessage::Handshake {
        api_version: API_VERSION,
        nickname: username,
    })
    .or(Err("Could not send handshake message."))?;

    client.send_message(RELIABLE_CHANNEL_ID, payload);

    Ok(ConnectionData {
        client,
        client_id,
        transport,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::protocol::{GameCode, GameSnapshot, MapName, PlayerId};

    #[test]
    fn test_server_new_initial_state() {
        let server = Server::new();

        assert!(server.connection_data.is_none());
        assert!(server.connect_rx.is_none());
        assert!(server.game_update.is_none());
        assert!(server.initial_game_info.is_none());
        assert_eq!(server.client_state, ClientState::Disconnected);
        assert!(server.request_response.is_none());
        assert!(!server.request_pending);
    }

    #[test]
    fn test_server_close_resets_state() {
        let mut server = Server::new();

        // Modify some state
        server.request_pending = true;
        server.request_response = Some(Ok(()));
        server.client_state = ClientState::Connected;

        // Close should reset everything
        server.close();

        assert!(server.connection_data.is_none());
        assert_eq!(server.client_state, ClientState::Disconnected);
        assert!(server.request_response.is_none());
        assert!(!server.request_pending);
    }

    #[test]
    fn test_take_request_response() {
        let mut server = Server::new();

        // Initially none
        assert!(server.take_request_response().is_none());

        // Set a response
        server.request_response = Some(Ok(()));
        assert!(server.take_request_response().is_some());

        // Should be consumed
        assert!(server.take_request_response().is_none());
    }

    #[test]
    fn test_take_request_response_with_error() {
        let mut server = Server::new();

        server.request_response = Some(Err("Test error".to_string()));
        let response = server.take_request_response();

        assert!(response.is_some());
        assert!(response.unwrap().is_err());
    }

    #[test]
    fn test_game_update_take() {
        let mut server = Server::new();

        // Initially none
        assert!(server.game_update().is_none());

        // Set an update
        server.game_update = Some(GameUpdate {
            snapshot: GameSnapshot {
                engine: common::protocol::EngineSnapshot {
                    tanks: vec![],
                    projectiles: vec![],
                },
                state: common::protocol::GameState::Waiting,
                game_master: 1,
                round_number: 1,
            },
            events: vec![],
        });

        assert!(server.game_update().is_some());
        // Should be consumed
        assert!(server.game_update().is_none());
    }

    #[test]
    fn test_initial_game_info_take() {
        let mut server = Server::new();

        // Initially none
        assert!(server.initial_game_info().is_none());

        // Set game info
        server.initial_game_info = Some(InitialGameInfo {
            game_code: GameCode("1234".to_string()),
            player_id: 0 as PlayerId,
            num_rounds: 3,
            map_name: MapName::Basic,
            game_master: 1,
        });

        assert!(server.initial_game_info().is_some());
        // Should be consumed
        assert!(server.initial_game_info().is_none());
    }

    #[test]
    fn test_client_id_none_when_not_connected() {
        let server = Server::new();
        assert!(server.client_id().is_none());
    }

    #[test]
    fn test_assert_state_passes_when_correct() {
        let server = Server::new();
        server.assert_state(ClientState::Disconnected); // Should not panic
    }

    #[test]
    #[should_panic(expected = "Server is in invalid state")]
    fn test_assert_state_panics_when_wrong() {
        let server = Server::new();
        server.assert_state(ClientState::Connected); // Should panic
    }

    // Test state machine transitions via handle_*_state methods
    // These require request_pending to be true to properly complete

    #[test]
    fn test_handle_disconnected_state_handshake_ok() {
        let mut server = Server::new();
        server.request_pending = true;

        let result = server
            .handle_disconnected_state(ServerMessage::HandshakeResponse(HandshakeResponse::Ok));

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ClientState::Connected);
        assert!(!server.request_pending);
        assert!(server.request_response.is_some());
        assert!(server.request_response.as_ref().unwrap().is_ok());
    }

    #[test]
    fn test_handle_disconnected_state_api_mismatch() {
        let mut server = Server::new();
        server.request_pending = true;

        let result = server.handle_disconnected_state(ServerMessage::HandshakeResponse(
            HandshakeResponse::ApiMismatch,
        ));

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("API mismatch"));
    }

    #[test]
    fn test_handle_disconnected_state_server_full() {
        let mut server = Server::new();
        server.request_pending = true;

        let result = server.handle_disconnected_state(ServerMessage::HandshakeResponse(
            HandshakeResponse::ServerFull,
        ));

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("server is full"));
    }

    #[test]
    fn test_handle_disconnected_state_error_message() {
        let mut server = Server::new();
        server.request_pending = true;

        let result =
            server.handle_disconnected_state(ServerMessage::Error("Custom error".to_string()));

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Custom error"));
    }

    #[test]
    fn test_handle_disconnected_state_invalid_message() {
        let mut server = Server::new();
        server.request_pending = true;

        let result = server.handle_disconnected_state(ServerMessage::LeaveGameAck);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid server message"));
    }

    #[test]
    fn test_handle_connected_state_create_game_ok() {
        let mut server = Server::new();
        server.client_state = ClientState::Connected;
        server.request_pending = true;

        let game_info = InitialGameInfo {
            game_code: GameCode("5678".to_string()),
            player_id: 1,
            num_rounds: 5,
            map_name: MapName::Basic,
            game_master: 100,
        };

        let result = server.handle_connected_state(ServerMessage::CreateGameReponse(
            CreateGameResponse::Ok(game_info.clone()),
        ));

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ClientState::Playing);
        assert!(server.initial_game_info.is_some());
        let info = server.initial_game_info.unwrap();
        assert_eq!(info.game_code.0, "5678");
    }

    #[test]
    fn test_handle_connected_state_create_game_too_many() {
        let mut server = Server::new();
        server.client_state = ClientState::Connected;
        server.request_pending = true;

        let result = server.handle_connected_state(ServerMessage::CreateGameReponse(
            CreateGameResponse::TooManyGames,
        ));

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ClientState::Connected);
        assert!(server.request_response.as_ref().unwrap().is_err());
    }

    #[test]
    fn test_handle_connected_state_join_game_ok() {
        let mut server = Server::new();
        server.client_state = ClientState::Connected;
        server.request_pending = true;

        let game_info = InitialGameInfo {
            game_code: GameCode("9999".to_string()),
            player_id: 2,
            num_rounds: 3,
            map_name: MapName::Basic,
            game_master: 50,
        };

        let result = server.handle_connected_state(ServerMessage::JoinGameResponse(
            JoinGameResponse::Ok(game_info),
        ));

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ClientState::Playing);
    }

    #[test]
    fn test_handle_connected_state_join_game_full() {
        let mut server = Server::new();
        server.client_state = ClientState::Connected;
        server.request_pending = true;

        let result = server
            .handle_connected_state(ServerMessage::JoinGameResponse(JoinGameResponse::GameFull));

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ClientState::Connected);
        let resp = server.request_response.as_ref().unwrap();
        assert!(resp.is_err());
        assert!(resp.as_ref().unwrap_err().contains("full"));
    }

    #[test]
    fn test_handle_connected_state_join_game_invalid_code() {
        let mut server = Server::new();
        server.client_state = ClientState::Connected;
        server.request_pending = true;

        let result = server.handle_connected_state(ServerMessage::JoinGameResponse(
            JoinGameResponse::InvalidCode,
        ));

        assert!(result.is_ok());
        let resp = server.request_response.as_ref().unwrap();
        assert!(resp.as_ref().unwrap_err().contains("invalid code"));
    }

    #[test]
    fn test_handle_connected_state_join_game_started() {
        let mut server = Server::new();
        server.client_state = ClientState::Connected;
        server.request_pending = true;

        let result = server.handle_connected_state(ServerMessage::JoinGameResponse(
            JoinGameResponse::GameStarted,
        ));

        assert!(result.is_ok());
        let resp = server.request_response.as_ref().unwrap();
        assert!(resp.as_ref().unwrap_err().contains("already started"));
    }

    #[test]
    fn test_handle_playing_state_game_update() {
        let mut server = Server::new();
        server.client_state = ClientState::Playing;

        let update = GameUpdate {
            snapshot: GameSnapshot {
                engine: common::protocol::EngineSnapshot {
                    tanks: vec![],
                    projectiles: vec![],
                },
                state: common::protocol::GameState::Battle(60),
                game_master: 1,
                round_number: 2,
            },
            events: vec![],
        };

        let result = server.handle_playing_state(ServerMessage::GameUpdate(update));

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ClientState::Playing);
        assert!(server.game_update.is_some());
    }

    #[test]
    fn test_handle_playing_state_start_countdown_ack() {
        let mut server = Server::new();
        server.client_state = ClientState::Playing;
        server.request_pending = true;

        let result = server.handle_playing_state(ServerMessage::StartCountdownAck);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ClientState::Playing);
        assert!(server.request_response.as_ref().unwrap().is_ok());
    }

    #[test]
    fn test_handle_playing_state_leave_game_ack() {
        let mut server = Server::new();
        server.client_state = ClientState::Playing;
        server.request_pending = true;

        let result = server.handle_playing_state(ServerMessage::LeaveGameAck);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ClientState::Connected);
    }

    #[test]
    fn test_handle_playing_state_error() {
        let mut server = Server::new();
        server.client_state = ClientState::Playing;

        let result = server.handle_playing_state(ServerMessage::Error("Game error".to_string()));

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Game error"));
    }

    #[test]
    fn test_handle_playing_state_invalid_message() {
        let mut server = Server::new();
        server.client_state = ClientState::Playing;

        let result =
            server.handle_playing_state(ServerMessage::HandshakeResponse(HandshakeResponse::Ok));

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid server message"));
    }

    #[test]
    fn test_complete_request_without_pending_fails() {
        let mut server = Server::new();
        server.request_pending = false;

        let result = server.complete_request(Ok(()), ClientState::Connected);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no request was made"));
    }

    #[test]
    fn test_complete_request_with_existing_response_fails() {
        let mut server = Server::new();
        server.request_pending = true;
        server.request_response = Some(Ok(()));

        let result = server.complete_request(Ok(()), ClientState::Connected);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no request was made"));
    }
}
