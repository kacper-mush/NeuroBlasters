use glam::Vec2;

use crate::{game::player::PlayerInfo, net::protocol::Tank};

impl Tank {
    // TODO: remove magic numbers
    pub fn new(player_info: PlayerInfo, position: Vec2) -> Self {
        Tank {
            player_info,
            position,
            velocity: Vec2::ZERO,
            rotation: 0.0,
            radius: 15.0,
            speed: 200.0,
            health: 100.0,
            weapon_cooldown: 0.0,
        }
    }
}
