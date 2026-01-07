use glam::Vec2;

use crate::net::protocol::{PlayerId, Tank, Team};

impl Tank {
    // TODO: remove magic numbers
    pub fn new(id: PlayerId, nickname: String, team: Team, position: Vec2) -> Self {
        Tank {
            id,
            team,
            position,
            nickname,
            velocity: Vec2::ZERO,
            rotation: 0.0,
            radius: 15.0,
            speed: 200.0,
            health: 100.0,
            weapon_cooldown: 0.0,
        }
    }
}


