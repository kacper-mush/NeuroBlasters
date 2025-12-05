mod room;

use std::{collections::HashMap, net::SocketAddr, net::UdpSocket, time::Duration, time::Instant};

use common::{
    API_VERSION, ApiVersion, ClientMessage, RoomCode, RoomMember, RoomState, RoomUpdate,
    ServerError, ServerMessage, SessionId, decode_client_message, encode_server_message,
};
use rand::{RngCore, SeedableRng, rngs::StdRng};
use renet::{ClientId, ConnectionConfig, RenetServer, ServerEvent};
use renet_netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig};
use room::Room;
use thiserror::Error;
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
        self.update_rooms(delta);
        // Send the queued packets to the clients.
        self.transport.send_packets(&mut self.server);

        Ok(())
    }

    fn update_rooms(&mut self, delta: Duration) {
        let now = Instant::now();
        let mut rooms_to_update = Vec::new();
        let mut rooms_to_remove = Vec::new();

        for (code, room) in self.rooms.iter_mut() {
            if room.advance_countdown(delta) {
                rooms_to_update.push(code.clone());
            }

            if room.should_remove(now, ROOM_IDLE_TIMEOUT) {
                rooms_to_remove.push(code.clone());
            }
        }

        for code in rooms_to_update {
            self.broadcast_room_update(&code);
        }

        for code in rooms_to_remove {
            self.rooms.remove(&code);
        }
    }

    fn broadcast_room_update(&mut self, room_code: &RoomCode) {
        let Some(room) = self.rooms.get_mut(room_code) else {
            return;
        };

        if !room.has_pending_events() {
            return;
        }

        let member_ids = room.member_ids();
        let events = room.drain_events();
        if member_ids.is_empty() || events.is_empty() {
            return;
        }

        let state = self.build_room_state(&member_ids);
        let update = RoomUpdate { state, events };
        for client_id in member_ids {
            self.send_message(
                client_id,
                ServerMessage::RoomUpdate {
                    update: update.clone(),
                },
            );
        }
    }

    fn build_room_state(&self, members: &[ClientId]) -> RoomState {
        let mut list: Vec<RoomMember> = members
            .iter()
            .filter_map(|client_id| self.sessions.get(client_id))
            .map(|session| RoomMember {
                session_id: session.session_id,
                nickname: session.nickname.clone(),
            })
            .collect();
        list.sort_by(|a, b| a.nickname.cmp(&b.nickname));
        RoomState { members: list }
    }

    fn build_room_state_for_code(&self, room_code: &RoomCode) -> RoomState {
        self.rooms
            .get(room_code)
            .map(|room| self.build_room_state(&room.member_ids()))
            .unwrap_or_else(|| RoomState {
                members: Vec::new(),
            })
    }

    fn detach_client_from_room(&mut self, client_id: ClientId) -> Option<RoomCode> {
        let Some((room_code, nickname)) = self.sessions.get(&client_id).and_then(|session| {
            session
                .room_code
                .clone()
                .map(|code| (code, session.nickname.clone()))
        }) else {
            return None;
        };

        if let Some(room) = self.rooms.get_mut(&room_code) {
            room.remove_member(client_id, nickname, Instant::now());
        }
        if let Some(session) = self.sessions.get_mut(&client_id) {
            session.room_code = None;
        }
        Some(room_code)
    }

    fn client_room_code(&self, client_id: ClientId) -> Option<RoomCode> {
        self.sessions
            .get(&client_id)
            .and_then(|session| session.room_code.clone())
    }

    fn generate_room_code(&mut self) -> RoomCode {
        loop {
            let code: String = (0..ROOM_CODE_LENGTH)
                .map(|_| {
                    let idx = (self.rng.next_u32() as usize) % ROOM_CODE_ALPHABET.len();
                    ROOM_CODE_ALPHABET[idx] as char
                })
                .collect();
            let room_code = RoomCode(code);
            if !self.rooms.contains_key(&room_code) {
                break room_code;
            }
        }
    }

    fn normalize_room_code(room_code: RoomCode) -> Option<RoomCode> {
        let normalized = room_code.0.trim().to_ascii_uppercase();
        if normalized.len() != ROOM_CODE_LENGTH {
            return None;
        }

        if normalized
            .chars()
            .all(|c| c.is_ascii() && ROOM_CODE_ALPHABET.contains(&(c as u8)))
        {
            Some(RoomCode(normalized))
        } else {
            None
        }
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
                    Ok(message) => {
                        if let Err(err) = self.handle_client_message(client_id, message) {
                            self.send_error(client_id, err);
                        }
                    }
                    Err(err) => {
                        debug!(client_id = %client_id, %err, "failed to decode client message");
                        self.send_error(client_id, ServerError::General);
                    }
                }
            }
        }
    }

    fn handle_client_message(
        &mut self,
        client_id: ClientId,
        message: ClientMessage,
    ) -> Result<(), ServerError> {
        trace!(client_id = %client_id, ?message, "received client message");
        match message {
            ClientMessage::Connect {
                api_version,
                nickname,
            } => self.handle_connect_message(client_id, api_version, nickname)?,
            ClientMessage::Disconnect => self.handle_disconnect_request(client_id)?,
            ClientMessage::RoomCreate => self.handle_room_create(client_id)?,
            ClientMessage::RoomJoin { room_code } => self.handle_room_join(client_id, room_code)?,
            ClientMessage::RoomLeave => self.handle_room_leave(client_id)?,
            ClientMessage::RoomStartCountdown { seconds } => {
                self.handle_room_start_countdown(client_id, seconds)?
            }
            other => self.handle_unimplemented_message(client_id, other)?,
        }
        Ok(())
    }

    fn handle_connect_message(
        &mut self,
        client_id: ClientId,
        api_version: u16,
        nickname: String,
    ) -> Result<(), ConnectError> {
        let requested_version = ApiVersion(api_version);

        if requested_version != API_VERSION {
            self.server.disconnect(client_id);
            return Err(ConnectError::ApiVersionMismatch {
                requested: requested_version.0,
                expected: API_VERSION.0,
            });
        }

        // If we already have an authenticated session for this client_id,
        // treat this as a duplicate handshake attempt.
        if let Some(existing) = self.sessions.get(&client_id) {
            return Err(ConnectError::DuplicateHandshake {
                session_id: existing.session_id,
            });
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
            SessionInfo {
                session_id,
                nickname,
                room_code: None,
            },
        );

        self.send_message(client_id, ServerMessage::ConnectOk { session_id });
        Ok(())
    }

    fn handle_disconnect_request(&mut self, client_id: ClientId) -> Result<(), DisconnectError> {
        debug!(client_id = %client_id, "client requested disconnect");
        if !self.sessions.contains_key(&client_id) {
            return Err(DisconnectError::NotConnected);
        }

        if let Some(room_code) = self.detach_client_from_room(client_id) {
            self.broadcast_room_update(&room_code);
        }

        self.sessions.remove(&client_id);
        self.server.disconnect(client_id);
        Ok(())
    }

    fn handle_room_create(&mut self, client_id: ClientId) -> Result<(), RoomCreateError> {
        let (nickname, current_room) = self
            .sessions
            .get(&client_id)
            .map(|session| (session.nickname.clone(), session.room_code.clone()))
            .ok_or(RoomCreateError::NotConnected)?;

        if let Some(room_code) = current_room {
            return Err(RoomCreateError::AlreadyInRoom { room_code });
        }

        let room_code = self.generate_room_code();
        let mut room = Room::new();
        room.add_member(client_id, nickname.clone());
        self.rooms.insert(room_code.clone(), room);

        if let Some(session) = self.sessions.get_mut(&client_id) {
            session.room_code = Some(room_code.clone());
        }

        self.send_message(
            client_id,
            ServerMessage::RoomCreateOk {
                room_code: room_code.clone(),
            },
        );
        self.broadcast_room_update(&room_code);
        Ok(())
    }

    fn handle_room_join(
        &mut self,
        client_id: ClientId,
        room_code: RoomCode,
    ) -> Result<(), RoomJoinError> {
        let (nickname, current_room) = self
            .sessions
            .get(&client_id)
            .map(|session| (session.nickname.clone(), session.room_code.clone()))
            .ok_or(RoomJoinError::NotConnected)?;

        if let Some(room_code) = current_room {
            return Err(RoomJoinError::AlreadyInRoom { room_code });
        }

        let normalized = Self::normalize_room_code(room_code.clone())
            .ok_or_else(|| RoomJoinError::InvalidCode { room_code })?;

        {
            let room = self
                .rooms
                .get_mut(&normalized)
                .ok_or_else(|| RoomJoinError::NotFound {
                    room_code: normalized.clone(),
                })?;

            if !room.add_member(client_id, nickname) {
                return Err(RoomJoinError::AlreadyInRoom {
                    room_code: normalized.clone(),
                });
            }
        }

        if let Some(session) = self.sessions.get_mut(&client_id) {
            session.room_code = Some(normalized.clone());
        }

        let state = self.build_room_state_for_code(&normalized);
        self.send_message(client_id, ServerMessage::RoomJoinOk { state });
        self.broadcast_room_update(&normalized);
        Ok(())
    }

    fn handle_room_leave(&mut self, client_id: ClientId) -> Result<(), RoomLeaveError> {
        if self.sessions.get(&client_id).is_none() {
            return Err(RoomLeaveError::NotConnected);
        }

        let Some(room_code) = self.detach_client_from_room(client_id) else {
            return Err(RoomLeaveError::NotInRoom);
        };

        self.send_message(client_id, ServerMessage::RoomLeaveOk);
        self.broadcast_room_update(&room_code);
        Ok(())
    }

    fn handle_room_start_countdown(
        &mut self,
        client_id: ClientId,
        seconds: u32,
    ) -> Result<(), CountdownError> {
        if self.sessions.get(&client_id).is_none() {
            return Err(CountdownError::NotConnected);
        }

        if seconds == 0 {
            return Err(CountdownError::InvalidSeconds);
        }

        let room_code = self
            .client_room_code(client_id)
            .ok_or(CountdownError::NotInRoom)?;

        {
            let room = self
                .rooms
                .get_mut(&room_code)
                .ok_or(CountdownError::NotInRoom)?;
            room.start_countdown(seconds);
        }
        self.broadcast_room_update(&room_code);
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
                return Err(ServerError::Connect);
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

    fn next_session_id(&mut self) -> SessionId {
        SessionId(self.rng.next_u64())
    }
}

#[derive(Debug, Clone)]
struct SessionInfo {
    session_id: SessionId,
    nickname: String,
    room_code: Option<RoomCode>,
}

#[derive(Debug, Error)]
enum ConnectError {
    #[error("api version mismatch: requested {requested}, expected {expected}")]
    ApiVersionMismatch { requested: u16, expected: u16 },
    #[error("client attempted duplicate handshake (session {session_id:?})")]
    DuplicateHandshake { session_id: SessionId },
}

#[derive(Debug, Error)]
enum DisconnectError {
    #[error("client is not connected")]
    NotConnected,
}

#[derive(Debug, Error)]
enum RoomCreateError {
    #[error("client is not connected")]
    NotConnected,
    #[error("client already belongs to room {room_code:?}")]
    AlreadyInRoom { room_code: RoomCode },
}

#[derive(Debug, Error)]
enum RoomJoinError {
    #[error("client is not connected")]
    NotConnected,
    #[error("client already belongs to room {room_code:?}")]
    AlreadyInRoom { room_code: RoomCode },
    #[error("room code {room_code:?} is invalid")]
    InvalidCode { room_code: RoomCode },
    #[error("room {room_code:?} was not found")]
    NotFound { room_code: RoomCode },
}

#[derive(Debug, Error)]
enum RoomLeaveError {
    #[error("client is not connected")]
    NotConnected,
    #[error("client is not part of any room")]
    NotInRoom,
}

#[derive(Debug, Error)]
enum CountdownError {
    #[error("client is not connected")]
    NotConnected,
    #[error("client is not part of any room")]
    NotInRoom,
    #[error("countdown duration must be greater than zero")]
    InvalidSeconds,
}

impl From<ConnectError> for ServerError {
    fn from(_: ConnectError) -> Self {
        ServerError::Connect
    }
}

impl From<DisconnectError> for ServerError {
    fn from(_: DisconnectError) -> Self {
        ServerError::Connect
    }
}

impl From<RoomCreateError> for ServerError {
    fn from(_: RoomCreateError) -> Self {
        ServerError::RoomCreate
    }
}

impl From<RoomJoinError> for ServerError {
    fn from(_: RoomJoinError) -> Self {
        ServerError::RoomJoin
    }
}

impl From<RoomLeaveError> for ServerError {
    fn from(_: RoomLeaveError) -> Self {
        ServerError::RoomLeave
    }
}

impl From<CountdownError> for ServerError {
    fn from(_: CountdownError) -> Self {
        ServerError::General
    }
}
