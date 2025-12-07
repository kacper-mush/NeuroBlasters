use bincode::{Decode, Encode};
use glam::Vec2;
use thiserror::Error;

// Message enums -------------------------------------------------------------

pub const API_VERSION: ApiVersion = ApiVersion(2);

/// All messages that the client can send to the server.
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum ClientMessage {
    Handshake { api_version: u16, nickname: String },
    CreateGame,
    JoinGame { game_code: String },
    LeaveGame,
}

/// Protocol / API version negotiated at connect time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Encode, Decode)]
pub struct ApiVersion(pub u16);

/// Unique identifier of a client session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Encode, Decode)]
pub struct SessionId(pub u64);

/// Human–facing lobby code used to join games.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Encode, Decode)]
pub struct GameCode(pub String);

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

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct GameUpdate {
    pub state: GameStateSnapshot,
    pub events: Vec<GameEvent>,
}

// Tells who killed whom.
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct KillEvent {
    pub killer_id: PlayerId,
    pub victim_id: PlayerId,
}
