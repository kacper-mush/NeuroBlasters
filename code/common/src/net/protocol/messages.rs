pub use renet::ClientId;

use crate::protocol::InitialGameInfo;

use super::objects::{GameSnapshot, InputPayload, KillEvent, MapName, Team};
use bincode::{Decode, Encode};

pub const API_VERSION: ApiVersion = 8;

/// Messages from Client -> Server
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum ClientMessage {
    Handshake {
        api_version: ApiVersion,
        nickname: String,
    },
    CreateGame {
        map: MapName,
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
    CreateGameReponse(CreateGameResponse),
    JoinGameResponse(JoinGameResponse),
    LeaveGameAck,
    StartCountdownAck,
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
    ApiMismatch,
    ServerFull,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum CreateGameResponse {
    Ok(InitialGameInfo),
    TooManyGames,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum JoinGameResponse {
    Ok(InitialGameInfo),
    InvalidCode,
    GameFull,
    GameStarted,
}

/// Human–facing lobby code used to join games.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Encode, Decode)]
pub struct GameCode(pub String);

pub type ApiVersion = u16;
