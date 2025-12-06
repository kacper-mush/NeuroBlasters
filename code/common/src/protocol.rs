use bincode::{Decode, Encode};
use glam::Vec2;
use thiserror::Error;

pub const API_VERSION: ApiVersion = ApiVersion(2);

/// All messages that the client can send to the server.
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum ClientMessage {
    /// Begin a session.
    Connect { api_version: u16, nickname: String },
    /// Request graceful shutdown.
    Disconnect,
    /// Create a game.
    GameCreate,
    /// Join an existing game by code.
    GameJoin { game_code: GameCode },
    /// Leave the current game (only before the game starts).
    GameLeave,
    /// Request the game countdown to start.
    GameStartCountdown { seconds: u32 },
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
    ConnectOk { session_id: SessionId },
    /// Game created.
    GameCreateOk { game_code: GameCode },
    /// Successfully joined game.
    GameJoinOk { tick_id: TickId, state: GameUpdate },
    /// Game update broadcast every tick.
    GameUpdate { tick_id: TickId, update: GameUpdate },
    /// Game leave result; players can't leave once the game started.
    GameLeaveOk,
    /// Game instance begins.
    GameStart,
    /// Game instance completes.
    GameEnd { result: GameResult },
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
    GameMap { map: MapDefinition },
    /// Unified error channel carrying all server → client errors.
    Error(ServerError),
}

/// All server → client errors carried through `ServerMessage::Error`.
#[derive(Debug, Clone, PartialEq, Eq, Error, Encode, Decode)]
pub enum ServerError {
    #[error("Handshake error")]
    Connect(#[from] ConnectError),
    #[error("Game creation error")]
    GameCreate(#[from] GameCreateError),
    #[error("Game join error")]
    GameJoin(#[from] GameJoinError),
    #[error("Game leave error")]
    GameLeave(#[from] GameLeaveError),
    #[error("Game countdown error")]
    GameCountdown(#[from] CountdownError),
    #[error("Input error at tick {tick_id:?}")]
    Input { tick_id: TickId },
    #[error("Server error")]
    General,
}

#[derive(Clone, PartialEq, Eq, Debug, Error, Encode, Decode)]
pub enum ConnectError {
    #[error("api version mismatch: requested {requested}, expected {expected}")]
    ApiVersionMismatch { requested: u16, expected: u16 },
    #[error("client attempted duplicate handshake (session {session_id:?})")]
    DuplicateHandshake { session_id: SessionId },
    #[error("client must complete handshake before sending messages")]
    HandshakeRequired,
}

#[derive(Clone, PartialEq, Eq, Debug, Error, Encode, Decode)]
pub enum GameCreateError {
    #[error("client already belongs to game {game_code:?}")]
    AlreadyInGame { game_code: GameCode },
}

#[derive(Clone, PartialEq, Eq, Debug, Error, Encode, Decode)]
pub enum GameJoinError {
    #[error("client already belongs to game {game_code:?}")]
    AlreadyInGame { game_code: GameCode },
    #[error("game code {game_code:?} is invalid")]
    InvalidCode { game_code: GameCode },
    #[error("game {game_code:?} was not found")]
    NotFound { game_code: GameCode },
    #[error("game {game_code:?} is not accepting new players")]
    NotJoinable { game_code: GameCode },
}

#[derive(Clone, PartialEq, Eq, Debug, Error, Encode, Decode)]
pub enum GameLeaveError {
    #[error("client is not part of any game")]
    NotInGame,
    #[error("game already started")]
    GameInProgress,
}

#[derive(Clone, PartialEq, Eq, Debug, Error, Encode, Decode)]
pub enum CountdownError {
    #[error("client is not part of any game")]
    NotInGame,
    #[error("countdown duration must be greater than zero")]
    InvalidSeconds,
    #[error("at least two players are required to start the countdown")]
    NotEnoughPlayers,
    #[error("countdown not allowed in current game state")]
    NotWaiting,
}

// Identifier newtypes -------------------------------------------------------

/// Protocol / API version negotiated at connect time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Encode, Decode)]
pub struct ApiVersion(pub u16);

/// Unique identifier of a client session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Encode, Decode)]
pub struct SessionId(pub u64);

/// Human–facing game code used to join games.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Encode, Decode)]
pub struct GameCode(pub String);

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

impl Default for InputPayload {
    fn default() -> Self {
        Self {
            move_axis: Vec2::ZERO,
            aim_pos: Vec2::ZERO,
            shoot: false,
        }
    }
}

/// Lightweight description of who is in the game.
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct GameMember {
    pub session_id: SessionId,
    pub nickname: String,
}

/// Authoritative per–tick game state snapshot.
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct GameStateSnapshot {
    pub players: Vec<PlayerState>,
    pub projectiles: Vec<Projectile>,
    pub time_remaining: f32,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct GameUpdate {
    pub members: Vec<GameMember>,
    pub state: GameStateUpdate,
    pub events: Vec<GameEvent>,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum GameStateUpdate {
    Waiting,
    Countdown {
        countdown_seconds_left: u32,
    },
    Started {
        snapshot: GameStateSnapshot,
    },
    Ended {
        winner: Option<Team>,
        snapshot: GameStateSnapshot,
    },
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

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum GameEvent {
    PlayerJoined { nickname: String },
    PlayerLeft { nickname: String },
    CountdownStarted { seconds: u32 },
    CountdownTick { seconds_left: u32 },
    CountdownFinished,
    CountdownCancelled,
    GameStarted,
    GameEnded { winner: Option<Team> },
    Kill(KillEvent),
}

// Tells who killed whom.
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct KillEvent {
    pub killer_id: PlayerId,
    pub victim_id: PlayerId,
}
