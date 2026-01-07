use crate::net::protocol::{PlayerId, Team};

#[derive(Debug, Clone)]
pub struct HumanInfo {
    pub id: PlayerId,
    pub nickname: String,
    pub team: Team,
}

impl HumanInfo {
    pub fn new(id: PlayerId, nickname: String, team: Team) -> Self {
        Self { id, nickname, team }
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
