use std::time::Instant;

use common::{GameStateSnapshot, GameUpdate, MapDefinition, RectWall, TickId};
use glam::Vec2;

pub struct GameStartContext {
    pub initial_tick_id: TickId,
}

#[allow(dead_code)]
pub struct GameInstance {
    started_at: Instant,
    next_tick_id: TickId,
}

impl GameInstance {
    pub fn start(started_at: Instant) -> (Self, GameStartContext) {
        let initial_tick_id = TickId(0);
        (
            Self {
                started_at,
                next_tick_id: TickId(initial_tick_id.0 + 1),
            },
            GameStartContext { initial_tick_id },
        )
    }
}

pub fn placeholder_map() -> MapDefinition {
    MapDefinition {
        width: 1000.0,
        height: 1000.0,
        walls: vec![RectWall {
            min: Vec2::new(400.0, 400.0),
            max: Vec2::new(600.0, 600.0),
        }],
    }
}

pub fn placeholder_game_update() -> GameUpdate {
    GameUpdate {
        state: placeholder_game_state(),
        events: Vec::new(),
    }
}

fn placeholder_game_state() -> GameStateSnapshot {
    GameStateSnapshot {
        players: Vec::new(),
        projectiles: Vec::new(),
        time_remaining: 0.0,
    }
}

