use std::fmt::Debug;
use std::{mem, net::ToSocketAddrs};

use common::protocol::{
    API_VERSION, CreateGameResponse, GameUpdate, HandshakeResponse, InitialGameInfo,
    JoinGameResponse, StartCountdownResponse,
};
use common::{
    codec::{decode_server_message, encode_client_message},
    game::player::is_valid_username,
    protocol::{ClientMessage, ServerMessage},
};
use rand::Rng;
use std::sync::mpsc::Receiver;

use std::net::UdpSocket;
use std::time::{Instant, SystemTime};

use renet::{ClientId, ConnectionConfig, RenetClient};
use renet_netcode::{ClientAuthentication, NetcodeClientTransport};

/// Represents in what state of the communication the client is
pub(crate) enum ClientState {
    /// Initial state
    Disconnected,
    /// Connected to the server, can create / join games
    Connected,
    /// In a game
    Playing {
        initial_game_info: InitialGameInfo,
        update: Option<GameUpdate>,
    },
    /// Unrecoverable server error, client should drop connection
    Error(String),
}

impl Debug for ClientState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ClientState::Disconnected => "Disconnected",
            ClientState::Connected => "Connected",
            ClientState::Playing { .. } => "Playing",
            ClientState::Error(_) => "Error",
        };
        f.write_str(name)
    }
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
    pub client_state: ClientState,
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
            let result = Server::connect_blocking(servername, username);
            let _ = tx.send(result);
        });
    }

    fn connect_blocking(
        mut servername: String,
        username: String,
    ) -> Result<ConnectionData, String> {
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
        let server_addr = addrs.first().copied().ok_or("Server not found.".to_string())?;

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

    // pub fn get_id(&self) -> ClientId {
    //     self.client_id
    // }

    pub fn tick(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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

        if let Some(mut connection_data) = self.connection_data.take() {
            connection_data
                .transport
                .update(dt, &mut connection_data.client)?;

            if connection_data.client.is_connected() {
                self.process_server_messages(&mut connection_data.client)?;
                connection_data
                    .transport
                    .send_packets(&mut connection_data.client)?;
            }
            self.connection_data = Some(connection_data);
        }

        Ok(())
    }

    fn process_server_messages(
        &mut self,
        client: &mut RenetClient,
    ) -> Result<(), Box<dyn std::error::Error>> {
        while let Some(message) = client.receive_message(RELIABLE_CHANNEL_ID) {
            let server_msg = decode_server_message(&message)?;
            self.process_message(server_msg);
        }
        Ok(())
    }

    fn process_message(&mut self, server_msg: ServerMessage) {
        //println!("Got server message: {:?}", server_msg);
        let old_state = mem::replace(&mut self.client_state, ClientState::Disconnected);
        //println!("Client state was before: {:?}", old_state);

        self.client_state = match old_state {
            ClientState::Disconnected => self.handle_disconnected_state(server_msg),

            ClientState::Connected => self.handle_connected_state(server_msg),

            ClientState::Playing {
                initial_game_info,
                update,
            } => self.handle_playing_state(server_msg, initial_game_info, update),

            ClientState::Error(err) => {
                panic!(
                    "Processing a message when the server is already in an inrecoverable error state! Reason for earlier failure: {}",
                    err
                )
            }
        };

        //println!("State is now: {:?}", self.client_state);
    }

    fn handle_disconnected_state(&mut self, server_msg: ServerMessage) -> ClientState {
        match server_msg {
            ServerMessage::HandshakeResponse(resp) => match resp {
                HandshakeResponse::Ok => self.complete_request(Ok(()), ClientState::Connected),
                HandshakeResponse::ApiMismatch => {
                    ClientState::Error("Server error: API mismatch.".into())
                }
                HandshakeResponse::ServerFull => {
                    ClientState::Error("Server error: server is full.".into())
                }
            },

            ServerMessage::Error(error) => {
                ClientState::Error(format!("Server error while handshaking: {}", error))
            }

            _ => ClientState::Error("Got invalid server message while handshaking.".into()),
        }
    }

    fn handle_connected_state(&mut self, server_msg: ServerMessage) -> ClientState {
        match server_msg {
            ServerMessage::CreateGameReponse(resp) => match resp {
                CreateGameResponse::Ok(initial_game_info) => self.complete_request(
                    Ok(()),
                    ClientState::Playing {
                        initial_game_info,
                        update: None,
                    },
                ),
                CreateGameResponse::TooManyGames => self.complete_request(
                    Err("Server game limit exhausted.".into()),
                    ClientState::Connected,
                ),
            },

            ServerMessage::JoinGameResponse(resp) => match resp {
                JoinGameResponse::Ok(initial_game_info) => self.complete_request(
                    Ok(()),
                    ClientState::Playing {
                        initial_game_info,
                        update: None,
                    },
                ),
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

            ServerMessage::Error(error) => ClientState::Error(format!("Server errror: {}", error)),

            _ => ClientState::Error("Got invalid server message.".into()),
        }
    }

    fn handle_playing_state(
        &mut self,
        server_msg: ServerMessage,
        initial_game_info: InitialGameInfo,
        update: Option<GameUpdate>,
    ) -> ClientState {
        match server_msg {
            ServerMessage::GameUpdate(new_update) => ClientState::Playing {
                initial_game_info,
                update: Some(new_update),
            },

            ServerMessage::StartCountdownResponse(resp) => match resp {
                StartCountdownResponse::Ok => self.complete_request(
                    Ok(()),
                    ClientState::Playing {
                        initial_game_info,
                        update,
                    },
                ),
                StartCountdownResponse::NotEnoughPlayers => self.complete_request(
                    Err("Not enough players to start game.".into()),
                    ClientState::Connected,
                ),
            },

            ServerMessage::LeaveGameAck => self.complete_request(Ok(()), ClientState::Connected),

            ServerMessage::Error(e) => ClientState::Error(format!(
                "Got error response from server while in game: {}",
                e
            )),

            _ => ClientState::Error("Got invalid server message while in game.".into()),
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
                ClientState::Playing {
                    initial_game_info: _,
                    update: _,
                },
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
    fn complete_request(
        &mut self,
        response: Result<(), String>,
        success: ClientState,
    ) -> ClientState {
        // In both cases there is some unwanted response; the first case is simple, but in the second
        // we have a guarantee from the send_client_message function that we do not make 2 consecutive requests,
        // so this new response must also be at server's fault
        if !self.request_pending || self.request_response.is_some() {
            return ClientState::Error("Server sent response but no request was made.".into());
        }

        self.request_response = Some(response);
        self.request_pending = false;
        success
    }

    pub fn take_request_response(&mut self) -> Option<Result<(), String>> {
        self.request_response.take()
    }

    pub fn close(&mut self) {
        self.connection_data.take();
        self.connect_rx = None;
        self.request_pending = false;
        self.request_response = None;
        self.client_state = ClientState::Disconnected;
    }
}
