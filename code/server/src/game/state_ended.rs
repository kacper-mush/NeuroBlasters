use common::{GameStateSnapshot, Team};

pub struct EndedState {
    pub winner: Option<Team>,
    pub snapshot: GameStateSnapshot,
}
