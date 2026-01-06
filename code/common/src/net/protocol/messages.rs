pub use renet::ClientId;

use super::objects::{GameSnapshot, InputPayload, KillEvent, MapId, PlayerId, Team};
use bincode::{Decode, Encode};

pub const API_VERSION: ApiVersion = 3;

/// Messages from Client -> Server
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum ClientMessage {
    Handshake {
        api_version: ApiVersion,
        nickname: String,
    },
    CreateGame {
        map: MapId,
        rounds: u8,
    },
    JoinGame {
        game_code: GameCode,
    },
    LeaveGame,
    StartCountdown,
    /// Player input for the current game tick
    GameInput(InputPayload),
}

/// Messages from Server -> Client
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum ServerMessage {
    HandshakeResponse(HandshakeResponse),
    CrateGameReponse(CrateGameReponse),
    JoinGameResponse(JoinGameResponse),
    LeaveGameResponse(LeaveGameResponse),
    StartCountdownResponse(StartCountdownResponse),
    GameUpdate(GameUpdate),
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct GameUpdate {
    pub snapshot: GameSnapshot,
    pub events: Vec<GameEvent>,
}

/// One-shot events for the UI/Audio (not persistent state)
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum GameEvent {
    PlayerJoined(String),
    PlayerLeft(String),
    RoundStarted,
    RoundEnded(Team),
    Kill(KillEvent),
}

// Change the error types to enum if needed

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum HandshakeResponse {
    Ok,
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum CrateGameReponse {
    Ok {
        game_code: GameCode,
        player_id: PlayerId,
    },
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum JoinGameResponse {
    Ok { player_id: PlayerId },
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum LeaveGameResponse {
    Ok,
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum StartCountdownResponse {
    Ok,
    Error(String),
}

/// Human–facing lobby code used to join games.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Encode, Decode)]
pub struct GameCode(pub String);

pub type ApiVersion = u16;
