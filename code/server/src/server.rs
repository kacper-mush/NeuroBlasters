use std::{collections::HashMap, net::SocketAddr, net::UdpSocket, time::Duration, time::Instant};

use common::codec::{decode_client_message, encode_server_message};
use common::protocol::{API_VERSION, ClientMessage, ServerMessage, HandshakeResponse, CrateGameReponse, JoinGameResponse, LeaveGameResponse, StartCountdownResponse};

use crate::client::{Client, ClientState};
use crate::game::GameCommand;
use crate::game_manager::GameManager;

use renet::{ClientId, ConnectionConfig, RenetServer, ServerEvent};
use renet_netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig};
use tracing::{debug, info};

const SERVER_PORT: u16 = 8080;
const MAX_CLIENTS: usize = 64;
const PROTOCOL_ID: u64 = 0;
const RELIABLE_CHANNEL_ID: u8 = 0;

type AppResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct ServerApp {
    server: RenetServer,
    transport: NetcodeServerTransport,

    clients: HashMap<ClientId, Client>,
    game_manager: GameManager,

    last_tick: Instant,
}

impl ServerApp {
    pub fn new() -> AppResult<Self> {
        let current_time = Duration::ZERO;
        let public_addr: SocketAddr = ([0, 0, 0, 0], SERVER_PORT).into();
        let server_config = ServerConfig {
            current_time,
            max_clients: MAX_CLIENTS,
            protocol_id: PROTOCOL_ID,
            public_addresses: vec![public_addr],
            authentication: ServerAuthentication::Unsecure,
        };

        let socket = UdpSocket::bind(public_addr)?;
        let transport = NetcodeServerTransport::new(server_config, socket)?;
        let server = RenetServer::new(ConnectionConfig::default());

        info!("Server listening on port {}", SERVER_PORT);

        Ok(Self {
            server,
            transport,
            clients: HashMap::new(),
            game_manager: GameManager::new(),
            last_tick: Instant::now(),
        })
    }

    pub fn tick(&mut self) -> AppResult<()> {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f32();
        self.last_tick = now;

        self.transport
            .update(Duration::from_secs_f32(dt), &mut self.server)?;
        self.server.update(Duration::from_secs_f32(dt));

        self.process_net_events();
        self.process_client_messages();

        let updates = self.game_manager.tick(dt);

        for (recipients, update) in updates {
            // Encode once, send bytes to many.
            if let Ok(payload) = encode_server_message(&ServerMessage::GameUpdate(update)) {
                for client_id in recipients {
                    self.server
                        .send_message(client_id, RELIABLE_CHANNEL_ID, payload.clone());
                }
            }
        }

        self.transport.send_packets(&mut self.server);

        Ok(())
    }

    pub fn shutdown(&mut self) {
        self.transport.disconnect_all(&mut self.server);
    }

    fn process_net_events(&mut self) {
        while let Some(event) = self.server.get_event() {
            match event {
                ServerEvent::ClientConnected { client_id } => {
                    info!(%client_id, "Client connected");
                    self.clients.insert(client_id, Client::default());
                }
                ServerEvent::ClientDisconnected { client_id, reason } => {
                    info!(%client_id, ?reason, "Client disconnected");
                    // If this client was in a game, he will be idle and unable to reconnect.
                    self.clients.remove(&client_id);
                }
            }
        }
    }

    fn process_client_messages(&mut self) {
        let client_ids = self.server.clients_id();
        for client_id in client_ids {
            while let Some(bytes) = self.server.receive_message(client_id, RELIABLE_CHANNEL_ID) {
                let msg = match decode_client_message(bytes.as_ref()) {
                    Ok(m) => m,
                    Err(e) => {
                        debug!(%client_id, %e, "Failed to decode message");
                        continue;
                    }
                };

                match self.handle_message(client_id, msg) {
                    Err(e) => self.send_message(client_id, ServerMessage::Error(e.to_string())),
                    Ok(msg) => {
                        if let Some(msg) = msg {
                            self.send_message(client_id, msg);
                        }
                    }
                }
            }
        }
    }

    fn handle_message(
        &mut self,
        client_id: ClientId,
        message: ClientMessage,
    ) -> Result<Option<ServerMessage>, String> {
        // Handle handshake
        if let ClientMessage::Handshake { api_version, nickname } = message {
            if api_version != API_VERSION {
                return Err("Api version mismatch".to_string());
            } else {
                if self.clients.contains_key(&client_id) {
                    return Err("Client already connected".to_string());
                } else {
                    self.clients.insert(client_id, Client { nickname: nickname.clone(), state: ClientState::Lobby });
                    return Ok(Some(ServerMessage::HandshakeResponse(HandshakeResponse::Ok)));
                }
            }
        }

        // Handle other messages
        let client = self.clients.get_mut(&client_id).ok_or("Unknown sender")?;

        let (response, new_state) = match (&client.state, message) { 
            (ClientState::Lobby, ClientMessage::CreateGame { map, rounds }) => {
                let game_code = self.game_manager.create_game(client_id, map, rounds);
                self.game_manager.handle_game_command(
                    &game_code,
                    GameCommand::Join{ client_id, nickname: client.nickname.clone() },
                )?;
                (
                    Some(ServerMessage::CrateGameReponse(CrateGameReponse::Ok { game_code: game_code.clone() })),
                    Some(ClientState::InGame { game_code }),
                )
            }

            (ClientState::Lobby, ClientMessage::JoinGame { game_code }) => {
                self.game_manager.handle_game_command(
                    &game_code,
                    GameCommand::Join{ client_id, nickname: client.nickname.clone() },
                )?;
                (
                    Some(ServerMessage::JoinGameResponse(JoinGameResponse::Ok)),
                    Some(ClientState::InGame { game_code }),
                )
            }

            (ClientState::InGame { game_code}, msg) => {
                match msg {
                    ClientMessage::LeaveGame => {
                        self.game_manager
                            .handle_game_command(game_code, GameCommand::Leave{ client_id })?;
                        (Some(ServerMessage::LeaveGameResponse(LeaveGameResponse::Ok)), Some(ClientState::Lobby))
                    }
                    ClientMessage::StartGame => {
                        self.game_manager
                            .handle_game_command(game_code, GameCommand::StartCountdown{ client_id })?;
                        (Some(ServerMessage::StartCountdownResponse(StartCountdownResponse::Ok)), None)
                    }
                    ClientMessage::GameInput(input) => {
                        self.game_manager
                            .handle_game_command(game_code, GameCommand::Input{ client_id, input })?;
                        (None, None)
                    }
                    _ => return Err("Invalid message in current state".to_string()),
                }
            }
            (_, _) => return Err("Invalid message in current state".to_string()),
        };

        if let Some(s) = new_state {
            client.state = s;
        }

        Ok(response)
    }

    fn send_message(&mut self, client_id: ClientId, message: ServerMessage) {
        if let Ok(payload) = encode_server_message(&message) {
            self.server
                .send_message(client_id, RELIABLE_CHANNEL_ID, payload);
        }
    }
}
