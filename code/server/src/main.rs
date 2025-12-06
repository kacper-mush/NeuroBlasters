mod connection;
mod room;

use std::{collections::HashMap, net::SocketAddr, net::UdpSocket, time::Duration, time::Instant};

use common::{
    ClientMessage, ConnectError, RoomCode, ServerError, ServerMessage, decode_client_message,
    encode_server_message,
};
use connection::SessionInfo;
use rand::{SeedableRng, rngs::StdRng};
use renet::{ClientId, ConnectionConfig, RenetServer, ServerEvent};
use renet_netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig};
use room::Room;
use tokio::time::{self, MissedTickBehavior};
use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::EnvFilter;

const SERVER_PORT: u16 = 8080;
const MAX_CLIENTS: usize = 64;
const PROTOCOL_ID: u64 = 0;
const RELIABLE_CHANNEL_ID: u8 = 0;
const TICK_INTERVAL: Duration = Duration::from_micros(33_333); // ≈30 Hz
const ROOM_CODE_LENGTH: usize = 6;
const ROOM_IDLE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const ROOM_CODE_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

type AppResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> AppResult<()> {
    init_tracing();

    let mut app = ServerApp::new()?;
    info!(
        addresses = ?app.public_addresses(),
        max_clients = MAX_CLIENTS,
        "listening for clients"
    );

    let mut ticker = time::interval(TICK_INTERVAL);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("received Ctrl+C, shutting down immediately");
                break;
            }
            _ = ticker.tick() => {
                if let Err(err) = app.tick() {
                    error!(error = %err, "server tick failed");
                }
            }
        }
    }

    app.shutdown();
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}

struct ServerApp {
    server: RenetServer,
    transport: NetcodeServerTransport,
    /// Only authenticated (handshaken) clients are stored here.
    sessions: HashMap<ClientId, SessionInfo>,
    rooms: HashMap<RoomCode, Room>,
    rng: StdRng,
    last_tick: Instant,
}

impl ServerApp {
    fn new() -> AppResult<Self> {
        let current_time = Duration::ZERO;
        let public_addr: SocketAddr = ([0, 0, 0, 0], SERVER_PORT).into();
        let server_config = ServerConfig {
            current_time,
            max_clients: MAX_CLIENTS,
            protocol_id: PROTOCOL_ID,
            public_addresses: vec![public_addr],
            // TODO: Add authentication.
            authentication: ServerAuthentication::Unsecure,
        };

        let socket = UdpSocket::bind(public_addr)?;
        let transport = NetcodeServerTransport::new(server_config, socket)?;
        let server = RenetServer::new(ConnectionConfig::default());

        Ok(Self {
            server,
            transport,
            sessions: HashMap::new(),
            rooms: HashMap::new(),
            rng: StdRng::from_os_rng(),
            last_tick: Instant::now(),
        })
    }

    fn public_addresses(&self) -> Vec<SocketAddr> {
        self.transport.addresses()
    }

    fn tick(&mut self) -> AppResult<()> {
        let now = Instant::now();
        let delta = now - self.last_tick;
        self.last_tick = now;

        // Recieve packets from clients.
        self.transport.update(delta, &mut self.server)?;
        // Advance the reliable-UDP simulation.
        self.server.update(delta);
        self.process_events();
        self.process_messages();
        // Update room countdowns, members and broadcast updates.
        self.update_rooms(delta);
        // Send the queued packets to the clients.
        self.transport.send_packets(&mut self.server);

        Ok(())
    }

    fn shutdown(&mut self) {
        info!("disconnecting all clients");
        self.transport.disconnect_all(&mut self.server);
        self.sessions.clear();
    }

    /// Process server events - client connections and disconnections.
    fn process_events(&mut self) {
        while let Some(event) = self.server.get_event() {
            match event {
                ServerEvent::ClientConnected { client_id } => {
                    info!(client_id = %client_id, "client connected");
                }
                ServerEvent::ClientDisconnected { client_id, reason } => {
                    info!(client_id = %client_id, ?reason, "client disconnected");
                    if let Some(room_code) = self.detach_client_from_room(client_id) {
                        self.broadcast_room_update(&room_code);
                    }
                    self.sessions.remove(&client_id);
                }
            }
        }
    }

    /// Deserialize and handle client messages.
    fn process_messages(&mut self) {
        let client_ids = self.server.clients_id();
        for client_id in client_ids {
            while let Some(bytes) = self.server.receive_message(client_id, RELIABLE_CHANNEL_ID) {
                match decode_client_message(bytes.as_ref()) {
                    Ok(message) => self.route_client_message(client_id, message),
                    Err(err) => {
                        debug!(client_id = %client_id, %err, "failed to decode client message");
                        self.send_error(client_id, ServerError::General);
                    }
                }
            }
        }
    }

    fn route_client_message(&mut self, client_id: ClientId, message: ClientMessage) {
        match message {
            ClientMessage::Connect {
                api_version,
                nickname,
            } => {
                if let Err(err) = self.handle_connect_message(client_id, api_version, nickname) {
                    self.send_error(client_id, err.into());
                }
            }
            other => {
                let Some(session_snapshot) = self.sessions.get(&client_id).cloned() else {
                    self.send_error(client_id, ConnectError::HandshakeRequired.into());
                    return;
                };
                if let Err(err) = self.handle_client_message(client_id, &session_snapshot, other) {
                    self.send_error(client_id, err);
                }
            }
        }
    }

    fn handle_client_message(
        &mut self,
        client_id: ClientId,
        session: &SessionInfo,
        message: ClientMessage,
    ) -> Result<(), ServerError> {
        trace!(client_id = %client_id, ?message, "received client message");
        match message {
            ClientMessage::Disconnect => self.handle_disconnect_request(client_id)?,
            ClientMessage::RoomCreate => self.handle_room_create(client_id, session)?,
            ClientMessage::RoomJoin { room_code } => {
                self.handle_room_join(client_id, session, room_code)?
            }
            ClientMessage::RoomLeave => self.handle_room_leave(client_id)?,
            ClientMessage::RoomStartCountdown { seconds } => {
                self.handle_room_start_countdown(session, seconds)?
            }
            other => self.handle_unimplemented_message(client_id, other)?,
        }
        Ok(())
    }

    fn handle_unimplemented_message(
        &mut self,
        client_id: ClientId,
        message: ClientMessage,
    ) -> Result<(), ServerError> {
        if !self.sessions.contains_key(&client_id) {
            if self.server.clients_id().contains(&client_id) {
                warn!(
                    client_id = %client_id,
                    "received message before handshake completed"
                );
                return Err(ConnectError::HandshakeRequired.into());
            } else {
                warn!(client_id = %client_id, "received message for unknown client");
            }
            return Ok(());
        }

        debug!(client_id = %client_id, ?message, "message type unimplemented");
        Err(ServerError::General)
    }

    fn send_error(&mut self, client_id: ClientId, error: ServerError) {
        self.send_message(client_id, ServerMessage::Error(error));
    }

    fn send_message(&mut self, client_id: ClientId, message: ServerMessage) {
        match encode_server_message(&message) {
            Ok(payload) => self
                .server
                .send_message(client_id, RELIABLE_CHANNEL_ID, payload),
            Err(err) => {
                error!(client_id = %client_id, %err, ?message, "failed to encode server message");
            }
        }
    }
}
