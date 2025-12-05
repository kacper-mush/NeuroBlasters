// common/src/protocol.rs
use glam::Vec2;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// --- 1. Identifier Types ---
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ApiVersion(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoomCode(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GameId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoundId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TickId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientId(pub u64);


// --- 2. Game Entities (The "Everything" you requested) ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerInput {
    pub move_axis: Vec2, // Normalized WASD vector
    pub aim_pos: Vec2,   // Mouse position in world coordinates
    pub shoot: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RectWall {
    pub min: Vec2,
    pub max: Vec2,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Map {
    pub width: f32,
    pub height: f32,
    pub walls: Vec<RectWall>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerState {
    pub id: PlayerId,
    pub position: Vec2,
    pub velocity: Vec2,
    pub rotation: f32,
    pub radius: f32,
    pub speed: f32,
    pub health: f32,
    pub weapon_cooldown: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Projectile {
    pub id: u64,
    pub owner_id: PlayerId,
    pub position: Vec2,
    pub velocity: Vec2,
    pub radius: f32,
}


// --- 3. Message Payloads ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameStateSnapshot {
    pub players: Vec<PlayerState>,
    pub projectiles: Vec<Projectile>,
    pub time_remaining: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoomState {
    pub player_ids: Vec<PlayerId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameResult {
    pub winner_id: Option<PlayerId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoundSummary {
    pub duration_seconds: f32,
}


// --- 4. The Messages ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientMessage {
    Connect { api_version: u16, nickname: String },
    Disconnect,
    RoomCreate,
    RoomJoin { room_code: RoomCode },
    RoomLeave,
    Input {
        tick_id: TickId,
        payload: PlayerInput,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerMessage {
    ConnectOk { session_id: SessionId },
    RoomCreateOk { room_code: RoomCode },
    RoomJoinOk { state: RoomState },
    RoomState { state: RoomState },
    RoomLeaveOk,
    GameStart { game_id: GameId },
    GameEnd { game_id: GameId, result: GameResult },
    RoundStart { round_id: RoundId, game_time_ms: u64 },
    RoundEnd { round_id: RoundId, summary: RoundSummary },
    GameMap { game_id: GameId, map: Map },
    GameState {
        game_id: GameId,
        tick_id: TickId,
        state: GameStateSnapshot,
    },
    Error(ServerError),
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum ServerError {
    #[error("Handshake error")]
    Connect,
    #[error("Room creation error")]
    RoomCreate,
    #[error("Room join error")]
    RoomJoin,
    #[error("Room leave error")]
    RoomLeave,
    #[error("Input error at tick {tick_id:?}")]
    Input { tick_id: TickId },
    #[error("Server error")]
    General,
}