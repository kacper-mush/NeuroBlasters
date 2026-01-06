use glam::Vec2;
use renet::ClientId;

use crate::protocol::{Player, Team};

impl Player {
    // TODO: remove magic numbers
    pub fn new(id: ClientId, nickname: String, team: Team, position: Vec2) -> Self {
        Player {
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

pub fn is_valid_username(username: &str) -> Result<(), String> {
    let len = username.len();
    if !(3..=16).contains(&len) {
        return Err("Username must be between 3 and 16 characters long.".into());
    }

    if !username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err("Username can only consist of alphanumerics and underscores.".into());
    }

    Ok(())
}
