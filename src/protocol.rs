use renet::{ChannelConfig, ConnectionConfig, SendType};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

pub const PROTOCOL_ID: u64 = 0x4E_42_41_49; // "NBAI"

pub const CLIENT_COMMAND_CHANNEL: u8 = 0;
pub const CLIENT_INPUT_CHANNEL: u8 = 1;
pub const SERVER_RELIABLE_CHANNEL: u8 = 0;
pub const SERVER_STATE_CHANNEL: u8 = 1;

const CHANNEL_MEMORY_BUDGET: usize = 256 * 1024;
const RESEND_MS: u64 = 150;

pub type Checksum = u64;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("serialization error: {0}")]
    Serialization(#[from] bincode::Error),
}

pub fn serialize_client_message(message: &ClientMessage) -> Result<Vec<u8>, ProtocolError> {
    bincode::serialize(message).map_err(ProtocolError::from)
}

pub fn deserialize_client_message(data: &[u8]) -> Result<ClientMessage, ProtocolError> {
    bincode::deserialize(data).map_err(ProtocolError::from)
}

pub fn serialize_server_message(message: &ServerMessage) -> Result<Vec<u8>, ProtocolError> {
    bincode::serialize(message).map_err(ProtocolError::from)
}

pub fn deserialize_server_message(data: &[u8]) -> Result<ServerMessage, ProtocolError> {
    bincode::deserialize(data).map_err(ProtocolError::from)
}

pub fn connection_config() -> ConnectionConfig {
    ConnectionConfig {
        available_bytes_per_tick: 60_000,
        server_channels_config: vec![
            ChannelConfig {
                channel_id: SERVER_RELIABLE_CHANNEL,
                max_memory_usage_bytes: CHANNEL_MEMORY_BUDGET,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(RESEND_MS),
                },
            },
            ChannelConfig {
                channel_id: SERVER_STATE_CHANNEL,
                max_memory_usage_bytes: CHANNEL_MEMORY_BUDGET,
                send_type: SendType::Unreliable,
            },
        ],
        client_channels_config: vec![
            ChannelConfig {
                channel_id: CLIENT_COMMAND_CHANNEL,
                max_memory_usage_bytes: CHANNEL_MEMORY_BUDGET,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(RESEND_MS),
                },
            },
            ChannelConfig {
                channel_id: CLIENT_INPUT_CHANNEL,
                max_memory_usage_bytes: CHANNEL_MEMORY_BUDGET,
                send_type: SendType::Unreliable,
            },
        ],
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    Connect {
        nickname: String,
        client_version: Option<String>,
    },
    Disconnect,
    RoomCreate,
    RoomJoin {
        room_code: String,
    },
    RoomLeave,
    Input {
        tick_id: u64,
        payload: Vec<u8>,
    },
    ResyncRequest {
        game_id: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    ConnectOk,
    ConnectError { error: String },
    ServerError { code: u16, message: String },
    RoomCreateOk { room_code: String },
    RoomCreateError { error: String },
    RoomJoinOk { room_state: RoomStatePayload },
    RoomJoinError { error: String },
    RoomDelta { delta: RoomDeltaPayload },
    RoomLeaveOk,
    RoomLeaveError { error: String },
    GameStart { game_id: String },
    GameEnd { game_id: String },
    RoundStart { round_id: u64 },
    RoundEnd { round_id: u64 },
    GameSnapshot { snapshot: GameSnapshot },
    GameDelta { delta: GameDelta },
    InputError { tick_id: u64, error: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoomStatePayload {
    pub room_code: String,
    pub state_id: u64,
    pub players: Vec<PlayerSummary>,
    pub settings: RoomSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoomDeltaPayload {
    pub room_code: String,
    pub state_id: u64,
    pub joined: Vec<PlayerSummary>,
    pub left: Vec<u64>,
    pub settings: Option<RoomSettings>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerSummary {
    pub client_id: u64,
    pub nickname: String,
    pub team: u8,
    pub is_ai: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoomSettings {
    pub map_name: String,
    pub team_size: u8,
    pub fills_with_ai: bool,
}

impl Default for RoomSettings {
    fn default() -> Self {
        Self {
            map_name: "demo".to_string(),
            team_size: 2,
            fills_with_ai: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameSnapshot {
    pub game_id: String,
    pub tick_id: u64,
    pub checksum: Checksum,
    pub map: MapData,
    pub players: Vec<PlayerState>,
    pub projectiles: Vec<ProjectileState>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameDelta {
    pub game_id: String,
    pub tick_id: u64,
    pub base_tick: u64,
    pub checksum: Checksum,
    pub players_updated: Vec<PlayerState>,
    pub players_removed: Vec<u64>,
    pub projectiles_updated: Vec<ProjectileState>,
    pub projectiles_removed: Vec<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapData {
    pub width: f32,
    pub height: f32,
    pub shapes: Vec<MapShape>,
    pub spawn_points: Vec<Vec2>,
}

impl MapData {
    pub fn demo() -> Self {
        Self {
            width: 1920.0,
            height: 1080.0,
            shapes: vec![
                MapShape::Rectangle {
                    origin: Vec2 { x: 400.0, y: 400.0 },
                    size: Vec2 { x: 250.0, y: 80.0 },
                },
                MapShape::Circle {
                    center: Vec2 {
                        x: 1100.0,
                        y: 520.0,
                    },
                    radius: 120.0,
                },
            ],
            spawn_points: vec![
                Vec2 { x: 200.0, y: 200.0 },
                Vec2 {
                    x: 1700.0,
                    y: 800.0,
                },
                Vec2 { x: 900.0, y: 540.0 },
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum MapShape {
    Circle { center: Vec2, radius: f32 },
    Rectangle { origin: Vec2, size: Vec2 },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerState {
    pub client_id: u64,
    pub nickname: String,
    pub position: Vec2,
    pub rotation: f32,
    pub health: f32,
    pub inventory: Vec<String>,
    pub team: u8,
    pub is_ai: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectileState {
    pub projectile_id: u64,
    pub owner_id: u64,
    pub position: Vec2,
    pub direction: Vec2,
    pub speed: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_client_message() {
        let message = ClientMessage::Input {
            tick_id: 42,
            payload: vec![1, 2, 3],
        };
        let bytes = serialize_client_message(&message).unwrap();
        let decoded = deserialize_client_message(&bytes).unwrap();
        assert!(matches!(decoded, ClientMessage::Input { tick_id, .. } if tick_id == 42));
    }

    #[test]
    fn roundtrip_server_message() {
        let message = ServerMessage::RoomCreateOk {
            room_code: "ABCD".into(),
        };
        let bytes = serialize_server_message(&message).unwrap();
        let decoded = deserialize_server_message(&bytes).unwrap();
        assert!(matches!(decoded, ServerMessage::RoomCreateOk { .. }));
    }

    #[test]
    fn connection_config_has_expected_channels() {
        let config = connection_config();
        assert_eq!(config.server_channels_config.len(), 2);
        assert_eq!(config.client_channels_config.len(), 2);
        assert!(
            config
                .server_channels_config
                .iter()
                .any(|c| c.channel_id == SERVER_STATE_CHANNEL)
        );
    }
}
