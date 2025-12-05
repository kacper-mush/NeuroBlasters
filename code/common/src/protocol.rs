use bincode::{Decode, Encode};
use glam::Vec2;
use thiserror::Error;

// Message enums -------------------------------------------------------------

pub const API_VERSION: ApiVersion = ApiVersion(1);

/// All messages that the client can send to the server.
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum ClientMessage {
    /// Begin a session.
    Connect { api_version: u16, nickname: String },
    /// Request graceful shutdown.
    Disconnect,
    /// Create a lobby.
    RoomCreate,
    /// Join an existing lobby by code.
    RoomJoin { room_code: RoomCode },
    /// Leave the current lobby (only before the game starts).
    RoomLeave,
    /// Request the room countdown to start.
    RoomStartCountdown { seconds: u32 },
    /// Provide input for a future simulation tick.
    Input {
        tick_id: TickId,
        payload: InputPayload,
    },
}

/// All messages that the server can send to the client.
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum ServerMessage {
    /// Confirm handshake.
    ConnectOk {
        session_id: SessionId,
    },
    /// Lobby created.
    RoomCreateOk {
        room_code: RoomCode,
    },
    /// Successfully joined lobby.
    RoomJoinOk { state: RoomState },
    /// Lobby update broadcast whenever player list / countdown change.
    RoomUpdate { update: RoomUpdate },
    /// Room leave result; players can't leave the room once game started.
    RoomLeaveOk,
    /// Game instance begins.
    GameStart {
        game_id: GameId,
    },
    /// Game instance completes.
    GameEnd {
        game_id: GameId,
        result: GameResult,
    },
    /// Per-round start signal.
    RoundStart {
        round_id: RoundId,
        game_time_ms: u64,
    },
    /// Per-round finish signal.
    RoundEnd {
        round_id: RoundId,
        summary: RoundSummary,
    },
    /// Static map transfer.
    GameMap {
        game_id: GameId,
        map: MapDefinition,
    },
    /// Authoritative per-tick state.
    GameState {
        game_id: GameId,
        tick_id: TickId,
        state: GameStateSnapshot,
    },
    PlayerKilled {
        kill_event: KillEvent,
    },
    /// Unified error channel carrying all server → client errors.
    Error(ServerError),
}

/// All server → client errors carried through `ServerMessage::Error`.
#[derive(Debug, Clone, PartialEq, Eq, Error, Encode, Decode)]
pub enum ServerError {
    #[error("Handshake error")]
    Connect,
    /// Lobby creation failed.
    #[error("Room creation error")]
    RoomCreate,
    /// Joining a lobby failed.
    #[error("Room join error")]
    RoomJoin,
    /// Leaving a lobby failed.
    #[error("Room leave error")]
    RoomLeave,
    /// Input for a given tick was rejected.
    #[error("Input error at tick {tick_id:?}")]
    Input { tick_id: TickId },
    /// Catch–all fatal / internal errors not covered by the categories above.
    #[error("Server error")]
    General,
}

// Concrete message payload structs -----------------------------------------

// Identifier newtypes -------------------------------------------------------

/// Protocol / API version negotiated at connect time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Encode, Decode)]
pub struct ApiVersion(pub u16);

/// Unique identifier of a client session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Encode, Decode)]
pub struct SessionId(pub u64);

/// Human–facing lobby code used to join rooms.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Encode, Decode)]
pub struct RoomCode(pub String);

/// Unique identifier of a game instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Encode, Decode)]
pub struct GameId(pub u64);

/// Unique identifier of a round within a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Encode, Decode)]
pub struct RoundId(pub u64);

/// Simulation tick identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Encode, Decode)]
pub struct TickId(pub u64);

/// Player identifier used across the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Encode, Decode)]
pub struct PlayerId(pub u64);

/// Low–level client identifier (transport / connection).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Encode, Decode)]
pub struct ClientId(pub u64);

// Additional Game Entities --------------------------------------------------

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct RectWall {
    #[bincode(with_serde)]
    pub min: Vec2,
    #[bincode(with_serde)]
    pub max: Vec2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
pub enum Team {
    Blue,
    Red,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct PlayerState {
    pub id: PlayerId,
    pub team: Team,
    #[bincode(with_serde)]
    pub position: Vec2,
    #[bincode(with_serde)]
    pub velocity: Vec2,
    pub rotation: f32,
    pub radius: f32,
    pub speed: f32,
    pub health: f32,
    pub weapon_cooldown: f32,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct Projectile {
    pub id: u64,
    pub owner_id: PlayerId,
    #[bincode(with_serde)]
    pub position: Vec2,
    #[bincode(with_serde)]
    pub velocity: Vec2,
    pub radius: f32,
}

// Payload types -------------------------------------------------------------

/// Logical input payload sent from the client for a single tick.
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct InputPayload {
    #[bincode(with_serde)]
    pub move_axis: Vec2,
    #[bincode(with_serde)]
    pub aim_pos: Vec2,
    pub shoot: bool,
}

/// Full lobby / room state snapshot.
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct RoomState {
    pub members: Vec<RoomMember>,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct RoomMember {
    pub session_id: SessionId,
    pub nickname: String,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct RoomUpdate {
    pub state: RoomState,
    pub events: Vec<RoomEvent>,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum RoomEvent {
    PlayerJoined { nickname: String },
    PlayerLeft { nickname: String },
    CountdownStarted { seconds: u32 },
    CountdownTick { seconds_left: u32 },
    CountdownFinished,
}

/// Result of a completed game.
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct GameResult {
    pub winner: Option<Team>,
}

/// Summary information for a completed round.
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct RoundSummary {
    pub duration_seconds: f32,
}

/// Static map definition.
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct MapDefinition {
    pub width: f32,
    pub height: f32,
    pub walls: Vec<RectWall>,
}

/// Authoritative per–tick game state snapshot.
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct GameStateSnapshot {
    pub players: Vec<PlayerState>,
    pub projectiles: Vec<Projectile>,
    pub time_remaining: f32,
}

// Tells who killed whom.
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct KillEvent {
    pub killer_id: PlayerId,
    pub victim_id: PlayerId,
}
