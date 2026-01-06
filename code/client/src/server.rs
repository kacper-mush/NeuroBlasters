use std::fmt::Debug;
use std::{mem, net::ToSocketAddrs};

use common::{
    codec::{decode_server_message, encode_client_message},
    game::engine::GameEngine,
    game::player::is_valid_username,
    protocol::{
        ApiVersion, ClientMessage, GameCode, GameEvent, GameStateSnapshot, MapDefinition,
        ServerMessage, Team,
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
    /// Connected to the server, can create / join rooms
    Connected,
    /// Requested a room join / creation, waiting for response
    WaitingForRoom,
    /// In a room with players waiting for the game to start
    InRoom {
        game_code: GameCode,
        player_names: Vec<String>,
    },
    /// In a game
    Playing { game_engine: GameEngine },
    /// Game ended
    AfterGame { winner: Team },
    /// Bad state, server should be dropped
    Error,
}

impl Debug for ClientState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ClientState::Disconnected => "Disconnected",
            ClientState::Handshaking => "Handshaking",
            ClientState::Connected => "Connected",
            ClientState::WaitingForRoom => "WaitingForRoom",
            ClientState::InRoom { .. } => "InRoom",
            ClientState::Playing { .. } => "Playing",
            ClientState::AfterGame { .. } => "AfterGame",
            ClientState::Error => "Error",
        };
        f.write_str(name)
    }
}

pub(crate) struct Server {
    client: RenetClient,
    transport: NetcodeClientTransport,
    last_tick: Instant,
    fresh: bool,
    client_id: ClientId,
    pub client_state: ClientState,
}

const PROTOCOL_ID: u64 = 0;
const RELIABLE_CHANNEL_ID: u8 = 0;
const API_VERSION: ApiVersion = ApiVersion(2);

impl Server {
    pub fn new(mut servername: String, username: String) -> Result<Self, String> {
        if !is_valid_username(&username) {
            return Err("Username invalid! Only alphanumerics and underscores allowed.".into());
        }
        // If no port suffix present, append the 8080 port which is the default for our server
        if !servername.contains(':') {
            servername.push_str(":8080");
        }

        let server_addr = servername
            .to_socket_addrs()
            .ok()
            .and_then(|mut iter| iter.next())
            .ok_or("Server not found".to_string())?;

        let connection_config = ConnectionConfig::default();

        let client = RenetClient::new(connection_config);

        // Listen on all interfaces on any port, with the appropriate protocol
        let socket = if server_addr.is_ipv4() {
            UdpSocket::bind("0.0.0.0:0")
        } else {
            UdpSocket::bind("[::]:0")
        }
        .or(Err("Could not establish a connection".to_string()))?;

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
            .or(Err("Could not establish a connection"))?;

        let mut server = Self {
            client,
            client_id,
            transport,
            fresh: false,
            last_tick: Instant::now(),
            client_state: ClientState::Disconnected,
        };

        // TODO: Actual nickname adding
        server
            .send_client_message(ClientMessage::Handshake {
                api_version: API_VERSION,
                nickname: username,
            })
            .or(Err("Could not send handshake message"))?;

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
        self.fresh = true;
        Ok(())
    }

    fn process_message(&mut self, server_msg: ServerMessage) {
        //println!("Got server message: {:?}", server_msg);
        // Right now the action in most states for a bad server message is to just default
        // to a connected state, but later on it would probably be beneficial to add a bad message
        // counter with a memory timeout, and when too many bad messages are received in a short
        // amount of time, we go to ClientState::Error, as we probably desynced with the server
        let old_state = mem::replace(&mut self.client_state, ClientState::Disconnected);
        //println!("Client state was before: {:?}", old_state);

        self.client_state = match old_state {
            ClientState::Handshaking => match server_msg {
                ServerMessage::Ok => ClientState::Connected,
                ServerMessage::Error(error) => {
                    eprintln!("Server error while handshaking: {}", error);
                    ClientState::Error
                }
                _ => {
                    eprintln!("Got invalid server message while handshaking.");
                    ClientState::Error
                }
            },

            ClientState::WaitingForRoom => match server_msg {
                ServerMessage::GameJoined { game_code } => ClientState::InRoom {
                    game_code,
                    player_names: Vec::new(),
                },
                ServerMessage::Error(error) => {
                    eprintln!("Server errror when waiting for a room: {}", error);
                    ClientState::Connected // We don't need to drop the connection here
                }
                _ => {
                    eprintln!("Got invalid server message while waiting for room.");
                    ClientState::Connected
                }
            },

            ClientState::InRoom {
                game_code,
                player_names,
            } => match server_msg {
                ServerMessage::GameUpdate(update) => {
                    let player_names = update.players.into_iter().map(|(_id, name)| name).collect();
                    let events = update.events;
                    if !events.is_empty() {
                        println!("Game update in room events: {:?}", events);
                    }
                    match update.state {
                        // Nothing important happened
                        GameStateSnapshot::Waiting => ClientState::InRoom {
                            game_code,
                            player_names,
                        },

                        // The game has started
                        GameStateSnapshot::Battle {
                            players,
                            projectiles,
                        } => {
                            // There should be a map in the first Battle snapshot we recieve
                            let map: Option<MapDefinition> = events.iter().find_map(|e| {
                                if let GameEvent::GameStarted(map) = e {
                                    Some(map.clone())
                                } else {
                                    None
                                }
                            });

                            match map {
                                Some(map) => {
                                    let mut game_engine = GameEngine::new(map);
                                    game_engine.players = players;
                                    game_engine.projectiles = projectiles;
                                    ClientState::Playing { game_engine }
                                }
                                None => {
                                    eprintln!("Starting game failed: did not receive the map.");
                                    ClientState::Connected
                                }
                            }
                        }
                        _ => {
                            eprintln!("Got invalid server message while in room.");
                            ClientState::Connected
                        }
                    }
                }

                ServerMessage::Ok => {
                    // This can happen if we tried to start the game while in room,
                    // so that's okay. We have to wait for a game tick with a map anyways,
                    // so we do nothing here
                    ClientState::InRoom {
                        game_code,
                        player_names,
                    }
                }

                _ => {
                    eprintln!("Got invalid server message while in room.");
                    ClientState::InRoom {
                        game_code,
                        player_names,
                    }
                }
            },

            ClientState::Playing { mut game_engine } => match server_msg {
                ServerMessage::GameUpdate(update) => {
                    let events = update.events;
                    if !events.is_empty() {
                        println!("Playing events: {:?}", events);
                    }

                    match update.state {
                        // Game ended, leave
                        GameStateSnapshot::Ended { winner } => ClientState::AfterGame { winner },

                        // Game still going on
                        GameStateSnapshot::Battle {
                            players,
                            projectiles,
                        } => {
                            game_engine.players = players;
                            game_engine.projectiles = projectiles;
                            ClientState::Playing { game_engine }
                        }

                        _ => {
                            eprintln!("Got invalid server message while in game.");
                            ClientState::Connected
                        }
                    }
                }
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
}
