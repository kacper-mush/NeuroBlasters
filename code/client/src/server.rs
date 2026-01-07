use std::fmt::Debug;
use std::{mem, net::ToSocketAddrs};

use common::protocol::{
    API_VERSION, CreateGameResponse, GameUpdate, HandshakeResponse, InitialGameInfo, JoinGameResponse
};
use common::{
    codec::{decode_server_message, encode_client_message},
    game::engine::GameEngine,
    game::player::is_valid_username,
    protocol::{
        ApiVersion, ClientMessage, GameCode, GameEvent, MapDefinition, ServerMessage, Team,
    },
};
use rand::Rng;

use std::net::UdpSocket;
use std::time::{Instant, SystemTime};

use renet::{ClientId, ConnectionConfig, RenetClient};
use renet_netcode::{ClientAuthentication, NetcodeClientTransport};

/// Represents in what state of the communication the client is
pub(crate) enum ClientState {
    /// Initial state
    Disconnected,
    /// Waiting for the server to acknowledge connection
    Handshaking,
    /// Connected to the server, can create / join games
    Connected,
    /// Requested a room creation, waiting for response
    WaitingForGameCreation,
    /// Requested a room join, waiting for response
    WaitingForGameJoin,
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
            ClientState::Handshaking => "Handshaking",
            ClientState::Connected => "Connected",
            ClientState::WaitingForGameCreation => "WaitingForGameCreation",
            ClientState::WaitingForGameJoin => "WaitingForGameJoin",
            ClientState::Playing { .. } => "Playing",
            ClientState::Error(_) => "Error",
        };
        f.write_str(name)
    }
}

pub(crate) struct Server {
    client: RenetClient,
    transport: NetcodeClientTransport,
    last_tick: Instant,
    client_id: ClientId,
    pub client_state: ClientState,
    /// If request failed, the client can check why
    request_fail_reason: Option<String>,
}

const PROTOCOL_ID: u64 = 0;
const RELIABLE_CHANNEL_ID: u8 = 0;

impl Server {
    pub fn new(mut servername: String, username: String) -> Result<Self, String> {
        is_valid_username(&username)?;

        // If no port suffix present, append the 8080 port which is the default for our server
        if !servername.contains(':') {
            servername.push_str(":8080");
        }

        let server_addr = servername
            .to_socket_addrs()
            .ok()
            .and_then(|mut iter| iter.next())
            .ok_or("Server not found.".to_string())?;

        let connection_config = ConnectionConfig::default();

        let client = RenetClient::new(connection_config);

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

        let mut server = Self {
            client,
            client_id,
            transport,
            last_tick: Instant::now(),
            client_state: ClientState::Disconnected,
            request_fail_reason: None,
        };

        server
            .send_client_message(ClientMessage::Handshake {
                api_version: API_VERSION,
                nickname: username,
            })
            .or(Err("Handshake attempt failed."))?;

        Ok(server)
    }

    pub fn get_id(&self) -> ClientId {
        self.client_id
    }

    pub fn tick(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick);
        self.last_tick = now;

        self.transport.update(dt, &mut self.client)?;

        if self.client.is_connected() {
            self.process_server_messages()?;
            self.transport.send_packets(&mut self.client)?;
        }

        Ok(())
    }

    fn process_server_messages(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        while let Some(message) = self.client.receive_message(RELIABLE_CHANNEL_ID) {
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
            ClientState::Handshaking => match server_msg {
                ServerMessage::HandshakeResponse(resp) => match resp {
                    HandshakeResponse::Ok => ClientState::Connected,
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
            },

            ClientState::WaitingForGameCreation => match server_msg {
                ServerMessage::CreateGameReponse(resp) => match resp {
                    CreateGameResponse::Ok(initial_game_info) => {
                        let map = MapDefinition::load_name(initial_game_info.map_name);
                        let mut game_engine = GameEngine::new(map);
                        ClientState::Playing {
                            initial_game_info,
                            update: None,
                        }
                    }
                    CreateGameResponse::TooManyGames => {
                        self.set_fail_reason("Server game limit exhausted.");
                        ClientState::Connected
                    }
                },
                ServerMessage::Error(error) => ClientState::Error(format!(
                    "Server errror when waiting for game creation: {}",
                    error
                )),
                _ => {
                    ClientState::Error("Got invalid server message while waiting for game.".into())
                }
            },

            ClientState::WaitingForGameJoin => match server_msg {
                ServerMessage::JoinGameResponse(resp) => match resp {
                    JoinGameResponse::Ok(initial_game_info) => {
                        let map = MapDefinition::load_name(initial_game_info.map_name);
                        let mut game_engine = GameEngine::new(map);
                        ClientState::Playing {
                            initial_game_info,
                            update: None,
                        }
                    }
                    JoinGameResponse::GameFull => {
                        self.set_fail_reason("Game is full.");
                        ClientState::Connected
                    }
                    JoinGameResponse::InvalidCode => {
                        self.set_fail_reason("Game does not exist (invalid code).");
                        ClientState::Connected
                    }
                    JoinGameResponse::GameStarted => {
                        self.set_fail_reason("Game has already started.");
                        ClientState::Connected
                    }
                },
                ServerMessage::Error(error) => ClientState::Error(format!(
                    "Server errror when waiting for game join: {}",
                    error
                )),
                _ => {
                    ClientState::Error("Got invalid server message while waiting for game.".into())
                }
            },

            ClientState::Playing {
                initial_game_info,
                game_engine,
            } => match server_msg {
                ServerMessage::GameUpdate(update) => {
                    update.
                },
                ServerMessage::Error(e) => {
                    eprintln!("Got error response from server while in game: {}", e);
                    ClientState::Playing { game_engine }
                }
                _ => {
                    eprintln!("Got invalid server message while in game.");
                    ClientState::Connected
                }
            },

            state => match server_msg {
                ServerMessage::Error(e) => {
                    eprintln!("Got error response from server while in game: {}", e);
                    state
                }
                _ => {
                    eprintln!(
                        "Client not expecting any message, but got: {:?}",
                        server_msg
                    );
                    state
                }
            },
        };

        //println!("State is now: {:?}", self.client_state);
    }

    pub fn send_client_message(
        &mut self,
        msg: ClientMessage,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // We check correctness here, and potentially change state
        match (&self.client_state, &msg) {
            // Trying to establish a connection
            (
                ClientState::Disconnected,
                ClientMessage::Handshake {
                    api_version: _,
                    nickname: _,
                },
            ) => {
                self.client_state = ClientState::Handshaking;
            }

            // Trying to create / join a game
            (
                ClientState::Connected,
                ClientMessage::CreateGame | ClientMessage::JoinGame { game_code: _ },
            ) => {
                self.client_state = ClientState::WaitingForRoom;
            }

            // Trying to start when in a room
            (
                ClientState::InRoom {
                    game_code: _,
                    player_names: _,
                },
                ClientMessage::StartGame,
            ) => {}

            // Trying to send input when playing
            (ClientState::Playing { game_engine: _ }, ClientMessage::GameInput(_)) => {}

            // Trying to leave
            (state, ClientMessage::LeaveGame) => match state {
                ClientState::InRoom {
                    game_code: _,
                    player_names: _,
                }
                | ClientState::Playing { game_engine: _ } => {
                    // Only in these two states leaving makes sense
                    self.client_state = ClientState::Connected;
                }
                _ => panic!("Trying to leave when not in a game!"),
            },

            // We decide to panic, because this is not a network fault, there is
            // something wrong with the client logic.
            _ => {
                panic!("Invalid message for current state!");
            }
        }

        let payload = encode_client_message(&msg)?;
        self.client.send_message(RELIABLE_CHANNEL_ID, payload);
        Ok(())
    }

    pub fn get_fresh_game(&mut self) -> Option<GameEngine> {
        if self.fresh {
            match &self.client_state {
                ClientState::Playing { game_engine } => Some(game_engine.clone()),
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn back_to_lobby(&mut self) {
        if matches!(self.client_state, ClientState::AfterGame { winner: _ }) {
            self.client_state = ClientState::Connected;
        } else {
            panic!("Called not when directly after game!");
        }
    }

    fn set_fail_reason(&mut self, reason: &str) {
        if self.request_fail_reason.is_some() {
            panic!("Another request failed but client ignored the previous fail!");
        }
        self.request_fail_reason = Some(reason.into());
    }

    pub fn get_request_fail_reason(&mut self) -> Option<String> {
        self.request_fail_reason.take()
    }
}
