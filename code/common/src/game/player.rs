use bincode::{Decode, Encode};

use crate::net::protocol::{PlayerId, Team};

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct PlayerInfo {
    pub id: PlayerId,
    pub nickname: String,
    pub team: Team,
}

impl PlayerInfo {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_username_accepts_alphanumeric_and_underscore() {
        assert!(is_valid_username("Player_123").is_ok());
        assert!(is_valid_username("abc").is_ok());
        assert!(is_valid_username("A_B_C_1_2_3_4_5").is_ok());
    }

    #[test]
    fn invalid_username_too_short() {
        assert!(is_valid_username("ab").is_err());
        assert!(is_valid_username("").is_err());
    }

    #[test]
    fn invalid_username_too_long() {
        assert!(is_valid_username("12345678901234567").is_err()); // 17 chars
    }

    #[test]
    fn invalid_username_special_chars() {
        assert!(is_valid_username("user@name").is_err());
        assert!(is_valid_username("user name").is_err());
        assert!(is_valid_username("user-name").is_err());
    }
}
