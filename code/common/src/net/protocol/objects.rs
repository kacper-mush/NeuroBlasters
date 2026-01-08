pub use renet::ClientId;

use bincode::{Decode, Encode};
use glam::Vec2;
use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;

use crate::{game::player::PlayerInfo, protocol::GameCode};

pub type PlayerId = u16;

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct RectWall {
    #[bincode(with_serde)]
    pub min: Vec2,
    #[bincode(with_serde)]
    pub max: Vec2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub enum Team {
    Blue,
    Red,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct Tank {
    pub player_info: PlayerInfo,
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
    pub owner_info: PlayerInfo,
    #[bincode(with_serde)]
    pub position: Vec2,
    #[bincode(with_serde)]
    pub velocity: Vec2,
    pub radius: f32,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct InputPayload {
    #[bincode(with_serde)]
    pub move_axis: Vec2,
    #[bincode(with_serde)]
    pub aim_pos: Vec2,
    pub shoot: bool,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct MapDefinition {
    pub width: f32,
    pub height: f32,
    pub walls: Vec<RectWall>,
    #[bincode(with_serde)]
    pub spawn_points: Vec<(Team, Vec2)>,
}

#[derive(EnumIter, Copy, Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub enum MapName {
    Basic,
    Loss,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct GameSnapshot {
    pub engine: EngineSnapshot,
    pub state: GameState,
    pub game_master: ClientId,
    pub round_number: u8,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct EngineSnapshot {
    pub tanks: Vec<Tank>,
    pub projectiles: Vec<Projectile>,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum GameState {
    Waiting,
    Countdown(u64),
    Battle(u64),
    Results {
        winner: Team,
        blue_score: u8,
        red_score: u8,
    },
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct KillEvent {
    pub killer_info: PlayerInfo,
    pub victim_info: PlayerInfo,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct InitialGameInfo {
    pub game_code: GameCode,
    pub player_id: PlayerId,
    pub num_rounds: u8,
    pub map_name: MapName,
    pub game_master: ClientId,
}
