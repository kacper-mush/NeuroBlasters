use crate::protocol::{
    GameDelta, GameSnapshot, MapData, PlayerState, PlayerSummary, ProjectileState,
    RoomDeltaPayload, RoomSettings, RoomStatePayload, Vec2,
};
use parking_lot::{Mutex, RwLock};
use rand::{Rng, SeedableRng, distributions::Alphanumeric, rngs::StdRng};
use renet::ClientId;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use thiserror::Error;

const ROOM_CODE_LENGTH: usize = 4;

#[derive(Debug, Clone)]
pub(super) struct RoomCreateOutcome {
    pub room_code: String,
    pub state: RoomStatePayload,
}

#[derive(Debug, Clone)]
pub(super) struct RoomBroadcast {
    pub recipients: Vec<ClientId>,
    pub delta: RoomDeltaPayload,
}

#[derive(Debug, Clone)]
pub(super) struct JoinOutcome {
    pub state: RoomStatePayload,
    pub broadcast: Option<RoomBroadcast>,
}

#[derive(Debug, Clone)]
pub(super) struct LeaveOutcome {
    pub broadcast: Option<RoomBroadcast>,
}

#[derive(Debug, Error)]
pub(super) enum StateError {
    #[error("client is not registered")]
    UnknownClient,
    #[error("nickname already set")]
    NicknameAlreadySet,
    #[error("nickname must be set before interacting with rooms")]
    MissingNickname,
    #[error("client is already in a room")]
    AlreadyInRoom,
    #[error("room not found")]
    RoomNotFound,
    #[error("room is full")]
    RoomFull,
    #[error("client is not in a room")]
    NotInRoom,
}

#[derive(Debug)]
pub(super) struct SharedState {
    sessions: RwLock<HashMap<ClientId, Session>>,
    rooms: RwLock<HashMap<String, Room>>,
    code_rng: Mutex<StdRng>,
}

#[derive(Debug, Clone)]
struct Session {
    nickname: Option<String>,
    room_code: Option<String>,
}

#[derive(Debug, Clone)]
struct Room {
    code: String,
    state_id: u64,
    settings: RoomSettings,
    players: BTreeMap<ClientId, PlayerSummary>,
}

impl SharedState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            sessions: RwLock::new(HashMap::new()),
            rooms: RwLock::new(HashMap::new()),
            code_rng: Mutex::new(StdRng::from_entropy()),
        })
    }

    pub fn register_client(&self, client_id: ClientId) {
        let mut sessions = self.sessions.write();
        sessions.entry(client_id).or_insert(Session {
            nickname: None,
            room_code: None,
        });
    }

    pub fn unregister_client(&self, client_id: ClientId) -> Option<RoomBroadcast> {
        let mut sessions = self.sessions.write();
        let session = sessions.remove(&client_id)?;
        drop(sessions);
        if let Some(code) = session.room_code {
            let mut rooms = self.rooms.write();
            let room = rooms.get_mut(&code)?;
            if room.remove_player(client_id).is_some() {
                let delta = room.build_leave_delta(vec![client_id]);
                let recipients = room.other_members(client_id);
                let broadcast = RoomBroadcast { recipients, delta };
                if room.players.is_empty() {
                    rooms.remove(&code);
                }
                return Some(broadcast);
            }
        }
        None
    }

    pub fn set_nickname(&self, client_id: ClientId, nickname: String) -> Result<(), StateError> {
        let mut sessions = self.sessions.write();
        let session = sessions
            .get_mut(&client_id)
            .ok_or(StateError::UnknownClient)?;
        match &session.nickname {
            Some(_) => Err(StateError::NicknameAlreadySet),
            None => {
                session.nickname = Some(nickname);
                Ok(())
            }
        }
    }

    pub fn create_room(&self, client_id: ClientId) -> Result<RoomCreateOutcome, StateError> {
        let mut sessions = self.sessions.write();
        let session = sessions
            .get_mut(&client_id)
            .ok_or(StateError::UnknownClient)?;
        if session.nickname.is_none() {
            return Err(StateError::MissingNickname);
        }
        if session.room_code.is_some() {
            return Err(StateError::AlreadyInRoom);
        }

        let mut rooms = self.rooms.write();
        let code = self.next_room_code(&rooms);
        let mut room = Room::new(code.clone());
        room.add_player(client_id, session.nickname.as_ref().unwrap().clone());
        let state = room.as_payload();
        rooms.insert(code.clone(), room);
        session.room_code = Some(code.clone());
        Ok(RoomCreateOutcome {
            room_code: code,
            state,
        })
    }

    pub fn join_room(&self, client_id: ClientId, code: &str) -> Result<JoinOutcome, StateError> {
        let mut sessions = self.sessions.write();
        let session = sessions
            .get_mut(&client_id)
            .ok_or(StateError::UnknownClient)?;
        if session.nickname.is_none() {
            return Err(StateError::MissingNickname);
        }
        if session.room_code.is_some() {
            return Err(StateError::AlreadyInRoom);
        }

        let mut rooms = self.rooms.write();
        let room = rooms.get_mut(code).ok_or(StateError::RoomNotFound)?;
        let capacity = (room.settings.team_size.max(1) as usize) * 2;
        if room.players.len() >= capacity {
            return Err(StateError::RoomFull);
        }
        let recipients = room.other_members(client_id);
        let summary = room.add_player(client_id, session.nickname.as_ref().unwrap().clone());
        let state = room.as_payload();
        let delta = room.build_join_delta(vec![summary]);
        session.room_code = Some(code.to_string());
        let broadcast = if recipients.is_empty() {
            None
        } else {
            Some(RoomBroadcast { recipients, delta })
        };
        Ok(JoinOutcome { state, broadcast })
    }

    pub fn leave_room(&self, client_id: ClientId) -> Result<LeaveOutcome, StateError> {
        let mut sessions = self.sessions.write();
        let session = sessions
            .get_mut(&client_id)
            .ok_or(StateError::UnknownClient)?;
        let code = session.room_code.clone().ok_or(StateError::NotInRoom)?;

        let mut rooms = self.rooms.write();
        let room = rooms.get_mut(&code).ok_or(StateError::RoomNotFound)?;
        if room.remove_player(client_id).is_none() {
            return Err(StateError::NotInRoom);
        }
        let recipients = room.other_members(client_id);
        let delta = room.build_leave_delta(vec![client_id]);
        session.room_code = None;
        if room.players.is_empty() {
            rooms.remove(&code);
        }
        let broadcast = if recipients.is_empty() {
            None
        } else {
            Some(RoomBroadcast { recipients, delta })
        };
        Ok(LeaveOutcome { broadcast })
    }

    pub fn active_rooms(&self) -> Vec<String> {
        self.rooms.read().keys().cloned().collect()
    }

    pub fn compose_snapshot(&self, room_code: &str, tick_id: u64) -> Option<GameSnapshot> {
        let rooms = self.rooms.read();
        let room = rooms.get(room_code)?;
        let players = room
            .players
            .values()
            .enumerate()
            .map(|(idx, player)| build_player_state(player, tick_id, idx))
            .collect::<Vec<_>>();
        let projectiles = build_projectiles(tick_id, players.len());
        Some(GameSnapshot {
            game_id: room.code.clone(),
            tick_id,
            checksum: checksum(tick_id, room.state_id),
            map: MapData::demo(),
            players,
            projectiles,
        })
    }

    pub fn compose_delta(
        &self,
        room_code: &str,
        tick_id: u64,
        base_tick: u64,
    ) -> Option<GameDelta> {
        let rooms = self.rooms.read();
        let room = rooms.get(room_code)?;
        let players = room
            .players
            .values()
            .enumerate()
            .map(|(idx, player)| build_player_state(player, tick_id, idx))
            .collect::<Vec<_>>();

        Some(GameDelta {
            game_id: room.code.clone(),
            tick_id,
            base_tick,
            checksum: checksum(tick_id, room.state_id),
            players_updated: players,
            players_removed: Vec::new(),
            projectiles_updated: Vec::new(),
            projectiles_removed: Vec::new(),
        })
    }

    fn next_room_code(&self, rooms: &HashMap<String, Room>) -> String {
        let mut rng = self.code_rng.lock();
        loop {
            let code: String = (&mut *rng)
                .sample_iter(&Alphanumeric)
                .map(|c| char::from(c).to_ascii_uppercase())
                .filter(|c| c.is_ascii_uppercase())
                .take(ROOM_CODE_LENGTH)
                .collect();
            if !rooms.contains_key(&code) {
                break code;
            }
        }
    }
}

impl Room {
    fn new(code: String) -> Self {
        Self {
            code,
            state_id: 1,
            settings: RoomSettings::default(),
            players: BTreeMap::new(),
        }
    }

    fn add_player(&mut self, client_id: ClientId, nickname: String) -> PlayerSummary {
        let team = (self.players.len() as u8) % self.settings.team_size.max(1);
        let summary = PlayerSummary {
            client_id,
            nickname,
            team,
            is_ai: false,
        };
        self.players.insert(client_id, summary.clone());
        self.state_id += 1;
        summary
    }

    fn remove_player(&mut self, client_id: ClientId) -> Option<PlayerSummary> {
        let removed = self.players.remove(&client_id);
        if removed.is_some() {
            self.state_id += 1;
        }
        removed
    }

    fn as_payload(&self) -> RoomStatePayload {
        RoomStatePayload {
            room_code: self.code.clone(),
            state_id: self.state_id,
            players: self.players.values().cloned().collect(),
            settings: self.settings.clone(),
        }
    }

    fn build_join_delta(&self, joined: Vec<PlayerSummary>) -> RoomDeltaPayload {
        RoomDeltaPayload {
            room_code: self.code.clone(),
            state_id: self.state_id,
            joined,
            left: Vec::new(),
            settings: None,
        }
    }

    fn build_leave_delta(&self, left: Vec<ClientId>) -> RoomDeltaPayload {
        RoomDeltaPayload {
            room_code: self.code.clone(),
            state_id: self.state_id,
            joined: Vec::new(),
            left,
            settings: None,
        }
    }

    fn other_members(&self, exclude: ClientId) -> Vec<ClientId> {
        self.players
            .keys()
            .copied()
            .filter(|id| *id != exclude)
            .collect()
    }
}

fn build_player_state(summary: &PlayerSummary, tick_id: u64, idx: usize) -> PlayerState {
    let offset = idx as f32 * 64.0;
    let base = tick_id as f32 * 5.0;
    PlayerState {
        client_id: summary.client_id,
        nickname: summary.nickname.clone(),
        position: Vec2 {
            x: 150.0 + base + offset,
            y: 250.0 + offset,
        },
        rotation: ((tick_id + idx as u64) % 360) as f32,
        health: 100.0,
        inventory: vec!["rifle".to_string()],
        team: summary.team,
        is_ai: summary.is_ai,
    }
}

fn build_projectiles(tick_id: u64, count: usize) -> Vec<ProjectileState> {
    (0..count.min(3))
        .map(|idx| ProjectileState {
            projectile_id: tick_id + idx as u64,
            owner_id: idx as u64,
            position: Vec2 {
                x: 400.0 + idx as f32 * 25.0,
                y: 500.0 + idx as f32 * 15.0,
            },
            direction: Vec2 { x: 1.0, y: 0.0 },
            speed: 300.0 + idx as f32 * 10.0,
        })
        .collect()
}

fn checksum(tick_id: u64, state_id: u64) -> u64 {
    tick_id ^ (state_id << 8)
}
