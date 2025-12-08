pub use renet::ClientId;

use bincode::{Decode, Encode};
use glam::Vec2;

pub const API_VERSION: ApiVersion = ApiVersion(2);

// --- MESSAGES ---

/// Messages from Client -> Server
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum ClientMessage {
    Handshake { api_version: ApiVersion, nickname: String },
    CreateGame,
    JoinGame { game_code: GameCode },
    LeaveGame,
    StartGame,
    /// Player input for the current game tick
    GameInput(InputPayload), 
}

/// Messages from Server -> Client
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum ServerMessage {
    Ok,
    /// Response to CreateGame or JoinGame
    GameJoined { game_code: GameCode },
    /// The authoritative world state + events
    GameUpdate(GameUpdate),
    /// Something went wrong (e.g., "Game Full")
    Error(String),
}

// --- IDENTIFIERS ---

/// Protocol / API version negotiated at connect time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Encode, Decode)]
pub struct ApiVersion(pub u16);

/// Human–facing lobby code used to join games.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Encode, Decode)]
pub struct GameCode(pub String);

// --- GAME ENTITIES ---

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
pub struct Player {
    pub id: ClientId,
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
    pub owner_id: ClientId,
    #[bincode(with_serde)]
    pub position: Vec2,
    #[bincode(with_serde)]
    pub velocity: Vec2,
    pub radius: f32,
}

// --- PAYLOADS ---

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct InputPayload {
    #[bincode(with_serde)]
    pub move_axis: Vec2,
    #[bincode(with_serde)]
    pub aim_pos: Vec2,
    pub shoot: bool,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode, Default)]
pub struct MapDefinition {
    pub width: f32,
    pub height: f32,
    pub walls: Vec<RectWall>,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum GameStateSnapshot {
    Waiting,
    Battle {
        players: Vec<Player>,
        projectiles: Vec<Projectile>,
    },
    Ended {
        winner: Team
    }
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct GameUpdate {
    pub players: Vec<(ClientId, String)>,
    pub state: GameStateSnapshot,
    pub events: Vec<GameEvent>,
}

/// One-shot events for the UI/Audio (not persistent state)
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum GameEvent {
    PlayerJoined(String),
    PlayerLeft(String),
    GameStarted(MapDefinition),
    GameEnded(Team),
    Kill(KillEvent),
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct KillEvent {
    pub killer_id: ClientId,
    pub victim_id: ClientId,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum ServerError {
    General,
    GameFull,
    InvalidState,
}