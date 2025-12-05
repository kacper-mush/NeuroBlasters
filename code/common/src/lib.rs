pub mod protocol;
pub mod game_logic;

pub use protocol::*;

use glam::Vec2;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInput {
    pub move_axis: Vec2,
    pub shoot: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RectWall {
    pub min: Vec2,
    pub max: Vec2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Map {
    pub width: f32,
    pub height: f32,
    pub walls: Vec<RectWall>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub position: Vec2,
    pub velocity: Vec2,
    pub radius: f32,
    pub speed: f32,
}
