use std::{
    collections::HashMap,
    net::{SocketAddr, UdpSocket},
    time::{Duration, Instant},
};

use common::{
    API_VERSION, ApiVersion, ClientMessage, ServerError, ServerMessage, SessionId,
    decode_client_message, encode_server_message,
};
use rand::{RngCore, SeedableRng, rngs::StdRng};
use renet::{ClientId, ConnectionConfig, RenetServer, ServerEvent};
use renet_netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig};
use tokio::time::{self, MissedTickBehavior};
use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::EnvFilter;

const SERVER_PORT: u16 = 8080;
const MAX_CLIENTS: usize = 64;
const PROTOCOL_ID: u64 = 0;
const RELIABLE_CHANNEL_ID: u8 = 0;
const TICK_INTERVAL: Duration = Duration::from_micros(33_333); // ≈30 Hz

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
    sessions: HashMap<ClientId, SessionState>,
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
                    self.sessions.insert(client_id, SessionState::Handshaking);
                }
                ServerEvent::ClientDisconnected { client_id, reason } => {
                    info!(client_id = %client_id, ?reason, "client disconnected");
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
                    Ok(message) => self.handle_client_message(client_id, message),
                    Err(err) => {
                        debug!(client_id = %client_id, %err, "failed to decode client message");
                        self.send_error(client_id, ServerError::General);
                    }
                }
            }
        }
    }

    fn handle_client_message(&mut self, client_id: ClientId, message: ClientMessage) {
        trace!(client_id = %client_id, ?message, "received client message");
        match message {
            ClientMessage::Connect {
                api_version,
                nickname,
            } => self.handle_connect_message(client_id, api_version, nickname),
            ClientMessage::Disconnect => self.handle_disconnect_request(client_id),
            other => self.handle_unimplemented_message(client_id, other),
        }
    }

    fn handle_connect_message(&mut self, client_id: ClientId, api_version: u16, nickname: String) {
        let requested_version = ApiVersion(api_version);

        if requested_version != API_VERSION {
            debug!(
                client_id = %client_id,
                requested = requested_version.0,
                expected = API_VERSION.0,
                "api version mismatch"
            );
            self.send_error(client_id, ServerError::Connect);
            self.server.disconnect(client_id);
            return;
        }

        let should_complete_handshake = {
            match self.sessions.get(&client_id) {
                Some(SessionState::Handshaking) => true,
                Some(SessionState::Connected(existing)) => {
                    debug!(
                        client_id = %client_id,
                        session_id = existing.session_id.0,
                        "client attempted to handshake twice"
                    );
                    self.send_error(client_id, ServerError::Connect);
                    false
                }
                None => {
                    warn!(client_id = %client_id, "received handshake message for unknown client");
                    false
                }
            }
        };

        if !should_complete_handshake {
            return;
        }

        let session_id = self.next_session_id();
        let nickname = nickname.trim().to_owned();
        info!(
            client_id = %client_id,
            session_id = session_id.0,
            nickname = %nickname,
            "handshake successful"
        );

        self.sessions.insert(
            client_id,
            SessionState::Connected(SessionInfo {
                session_id,
                nickname,
            }),
        );

        self.send_message(client_id, ServerMessage::ConnectOk { session_id });
    }

    fn handle_disconnect_request(&mut self, client_id: ClientId) {
        debug!(client_id = %client_id, "client requested disconnect");
        self.sessions.remove(&client_id);
        self.server.disconnect(client_id);
    }

    fn handle_unimplemented_message(&mut self, client_id: ClientId, message: ClientMessage) {
        if !self.ensure_connected(client_id) {
            return;
        }

        debug!(client_id = %client_id, ?message, "message type unimplemented");
        self.send_error(client_id, ServerError::General);
    }

    fn ensure_connected(&mut self, client_id: ClientId) -> bool {
        match self.sessions.get(&client_id) {
            Some(SessionState::Connected(_)) => true,
            Some(SessionState::Handshaking) => {
                warn!(client_id = %client_id, "received message before handshake completed");
                self.send_error(client_id, ServerError::Connect);
                false
            }
            None => {
                warn!(client_id = %client_id, "received message for unknown client");
                false
            }
        }
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

    fn next_session_id(&mut self) -> SessionId {
        SessionId(self.rng.next_u64())
    }
}

#[derive(Debug)]
enum SessionState {
    Handshaking,
    Connected(SessionInfo),
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct SessionInfo {
    session_id: SessionId,
    nickname: String,
}
