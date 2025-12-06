use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

use common::{
    CountdownError, RoomCode, RoomCreateError, RoomEvent, RoomJoinError, RoomLeaveError,
    RoomMember, RoomState, RoomUpdate, ServerMessage,
};
use rand::RngCore;
use renet::ClientId;

use crate::{
    ROOM_CODE_ALPHABET, ROOM_CODE_LENGTH, ROOM_IDLE_TIMEOUT, ServerApp, SessionInfo,
    countdown::{CountdownAdvance, CountdownTimer},
    game::{GameInstance, GameStartContext, placeholder_game_update, placeholder_map},
};

#[derive(Default)]
pub struct Room {
    members: HashSet<ClientId>,
    pending_events: Vec<RoomEvent>,
    countdown: Option<CountdownTimer>,
    empty_since: Option<Instant>,
    game: Option<GameInstance>,
}

impl Room {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_member(&mut self, client_id: ClientId, nickname: String) -> bool {
        if self.members.insert(client_id) {
            self.pending_events
                .push(RoomEvent::PlayerJoined { nickname });
            self.empty_since = None;
            true
        } else {
            false
        }
    }

    pub fn remove_member(&mut self, client_id: ClientId, nickname: String, now: Instant) -> bool {
        if self.members.remove(&client_id) {
            self.pending_events.push(RoomEvent::PlayerLeft { nickname });
            if self.members.is_empty() {
                self.empty_since = Some(now);
            }
            if self.members.len() < 2 {
                self.cancel_countdown();
            }
            true
        } else {
            false
        }
    }

    pub fn start_countdown(&mut self, seconds: u32) {
        self.countdown = Some(CountdownTimer::new(seconds));
        self.pending_events
            .push(RoomEvent::CountdownStarted { seconds });
    }

    pub fn advance_countdown(&mut self, delta: Duration) -> CountdownAdvance {
        let mut result = CountdownAdvance::default();
        if let Some(timer) = self.countdown.as_mut() {
            let (events, finished) = timer.advance(delta);
            if !events.is_empty() {
                self.pending_events.extend(events);
                result.emitted_events = true;
            }
            if finished {
                self.countdown = None;
                result.finished = true;
            }
        }
        result
    }

    pub fn can_start_game(&self) -> bool {
        self.members.len() >= 2
    }

    pub fn begin_game(&mut self, now: Instant) -> Option<GameStartContext> {
        if self.game.is_some() || !self.can_start_game() {
            return None;
        }

        let (instance, context) = GameInstance::start(now);
        self.game = Some(instance);
        Some(context)
    }

    pub fn has_pending_events(&self) -> bool {
        !self.pending_events.is_empty()
    }

    pub fn drain_events(&mut self) -> Vec<RoomEvent> {
        std::mem::take(&mut self.pending_events)
    }

    pub fn member_ids(&self) -> Vec<ClientId> {
        self.members.iter().copied().collect()
    }

    pub fn countdown_seconds_left(&self) -> Option<u32> {
        self.countdown.as_ref().map(|timer| timer.seconds_left())
    }

    pub fn should_remove(&self, now: Instant, timeout: Duration) -> bool {
        self.members.is_empty()
            && self
                .empty_since
                .is_some_and(|since| now.duration_since(since) >= timeout)
    }

    pub fn cancel_countdown(&mut self) -> bool {
        if self.countdown.take().is_some() {
            self.pending_events.push(RoomEvent::CountdownCancelled);
            true
        } else {
            false
        }
    }
}

impl ServerApp {
    pub(super) fn update_rooms(&mut self, delta: Duration) {
        let now = Instant::now();
        let mut rooms_to_update = Vec::new();
        let mut rooms_to_start = Vec::new();

        for (code, room) in self.rooms.iter_mut() {
            let countdown_progress = room.advance_countdown(delta);
            if countdown_progress.emitted_events {
                rooms_to_update.push(code.clone());
            }
            if countdown_progress.finished {
                rooms_to_start.push(code.clone());
            }
        }

        for code in rooms_to_update {
            self.broadcast_room_update(&code);
        }

        self.rooms
            .retain(|_, room| !room.should_remove(now, ROOM_IDLE_TIMEOUT));

        for code in rooms_to_start {
            self.bootstrap_room_game(&code, now);
        }
    }

    pub(super) fn broadcast_room_update(&mut self, room_code: &RoomCode) {
        let Some(room) = self.rooms.get_mut(room_code) else {
            return;
        };

        if !room.has_pending_events() {
            return;
        }

        let member_ids = room.member_ids();
        let countdown_seconds_left = room.countdown_seconds_left();
        let events = room.drain_events();
        if member_ids.is_empty() || events.is_empty() {
            return;
        }

        let state = self.build_room_state(&member_ids, countdown_seconds_left);
        let update = RoomUpdate { state, events };
        for client_id in member_ids {
            self.send_message(
                client_id,
                ServerMessage::RoomUpdate {
                    update: update.clone(),
                },
            );
        }
    }

    fn bootstrap_room_game(&mut self, room_code: &RoomCode, started_at: Instant) {
        let (member_ids, start_context) = {
            let Some(room) = self.rooms.get_mut(room_code) else {
                return;
            };

            if !room.can_start_game() {
                room.cancel_countdown();
                return;
            }

            let member_ids = room.member_ids();
            let Some(context) = room.begin_game(started_at) else {
                return;
            };

            (member_ids, context)
        };

        if member_ids.is_empty() {
            return;
        }

        let map = placeholder_map();
        let update = placeholder_game_update();

        for client_id in member_ids {
            self.send_message(client_id, ServerMessage::GameStart);
            self.send_message(client_id, ServerMessage::GameMap { map: map.clone() });
            self.send_message(
                client_id,
                ServerMessage::GameUpdate {
                    tick_id: start_context.initial_tick_id,
                    update: update.clone(),
                },
            );
        }
    }

    pub(super) fn detach_client_from_room(&mut self, client_id: ClientId) -> Option<RoomCode> {
        let (room_code, nickname) = {
            let session = self.sessions.get_mut(&client_id)?;
            let code = session.room_code.take()?;
            (code, session.nickname.clone())
        };

        if let Some(room) = self.rooms.get_mut(&room_code) {
            room.remove_member(client_id, nickname, Instant::now());
        }
        Some(room_code)
    }

    pub(super) fn handle_room_create(
        &mut self,
        client_id: ClientId,
        session: &SessionInfo,
    ) -> Result<(), RoomCreateError> {
        if let Some(room_code) = &session.room_code {
            return Err(RoomCreateError::AlreadyInRoom {
                room_code: room_code.clone(),
            });
        }

        let room_code = self.generate_room_code();
        let mut room = Room::new();
        room.add_member(client_id, session.nickname.clone());
        self.rooms.insert(room_code.clone(), room);

        if let Some(session) = self.sessions.get_mut(&client_id) {
            session.room_code = Some(room_code.clone());
        }

        self.send_message(
            client_id,
            ServerMessage::RoomCreateOk {
                room_code: room_code.clone(),
            },
        );
        self.broadcast_room_update(&room_code);
        Ok(())
    }

    pub(super) fn handle_room_join(
        &mut self,
        client_id: ClientId,
        session: &SessionInfo,
        room_code: RoomCode,
    ) -> Result<(), RoomJoinError> {
        if let Some(current_room) = &session.room_code {
            return Err(RoomJoinError::AlreadyInRoom {
                room_code: current_room.clone(),
            });
        }

        if !Self::is_valid_room_code(&room_code) {
            return Err(RoomJoinError::InvalidCode {
                room_code: room_code.clone(),
            });
        }

        {
            let room = self
                .rooms
                .get_mut(&room_code)
                .ok_or_else(|| RoomJoinError::NotFound {
                    room_code: room_code.clone(),
                })?;

            if !room.add_member(client_id, session.nickname.clone()) {
                return Err(RoomJoinError::AlreadyInRoom {
                    room_code: room_code.clone(),
                });
            }
        }

        if let Some(session) = self.sessions.get_mut(&client_id) {
            session.room_code = Some(room_code.clone());
        }

        let state = self.build_room_state_for_code(&room_code);
        self.send_message(client_id, ServerMessage::RoomJoinOk { state });
        self.broadcast_room_update(&room_code);
        Ok(())
    }

    pub(super) fn handle_room_leave(&mut self, client_id: ClientId) -> Result<(), RoomLeaveError> {
        let Some(room_code) = self.detach_client_from_room(client_id) else {
            return Err(RoomLeaveError::NotInRoom);
        };

        self.send_message(client_id, ServerMessage::RoomLeaveOk);
        self.broadcast_room_update(&room_code);
        Ok(())
    }

    pub(super) fn handle_room_start_countdown(
        &mut self,
        session: &SessionInfo,
        seconds: u32,
    ) -> Result<(), CountdownError> {
        if seconds == 0 {
            return Err(CountdownError::InvalidSeconds);
        }

        let room_code = session.room_code.clone().ok_or(CountdownError::NotInRoom)?;

        {
            let room = self
                .rooms
                .get_mut(&room_code)
                .ok_or(CountdownError::NotInRoom)?;
            if !room.can_start_game() {
                return Err(CountdownError::NotEnoughPlayers);
            }
            room.start_countdown(seconds);
        }
        self.broadcast_room_update(&room_code);
        Ok(())
    }

    fn build_room_state(
        &self,
        members: &[ClientId],
        countdown_seconds_left: Option<u32>,
    ) -> RoomState {
        let mut list: Vec<RoomMember> = members
            .iter()
            .filter_map(|client_id| self.sessions.get(client_id))
            .map(|session| RoomMember {
                session_id: session.session_id,
                nickname: session.nickname.clone(),
            })
            .collect();
        list.sort_by(|a, b| a.nickname.cmp(&b.nickname));
        RoomState {
            members: list,
            countdown_seconds_left,
        }
    }

    fn build_room_state_for_code(&self, room_code: &RoomCode) -> RoomState {
        self.rooms
            .get(room_code)
            .map(|room| self.build_room_state(&room.member_ids(), room.countdown_seconds_left()))
            .unwrap_or_else(|| RoomState {
                members: Vec::new(),
                countdown_seconds_left: None,
            })
    }

    fn generate_room_code(&mut self) -> RoomCode {
        loop {
            let code: String = (0..ROOM_CODE_LENGTH)
                .map(|_| {
                    let idx = (self.rng.next_u32() as usize) % ROOM_CODE_ALPHABET.len();
                    ROOM_CODE_ALPHABET[idx] as char
                })
                .collect();
            let room_code = RoomCode(code);
            if !self.rooms.contains_key(&room_code) {
                return room_code;
            }
        }
    }

    fn is_valid_room_code(room_code: &RoomCode) -> bool {
        room_code.0.len() == ROOM_CODE_LENGTH
            && room_code
                .0
                .chars()
                .all(|c| c.is_ascii() && ROOM_CODE_ALPHABET.contains(&(c as u8)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_remove_members_track_events() {
        let mut room = Room::new();
        assert!(room.add_member(1, "alpha".into()));
        assert!(room.has_pending_events());
        let events = room.drain_events();
        assert!(matches!(events[0], RoomEvent::PlayerJoined { .. }));

        let now = Instant::now();
        assert!(room.remove_member(1, "alpha".into(), now));
        let events = room.drain_events();
        assert!(matches!(events[0], RoomEvent::PlayerLeft { .. }));
    }

    #[test]
    fn countdown_emits_ticks_and_finishes() {
        let mut room = Room::new();
        room.start_countdown(2);
        assert!(room.has_pending_events());
        let events = room.drain_events();
        assert!(matches!(
            events[0],
            RoomEvent::CountdownStarted { seconds: 2 }
        ));

        let result = room.advance_countdown(Duration::from_secs(1));
        assert!(result.emitted_events);
        let events = room.drain_events();
        assert!(matches!(
            events[0],
            RoomEvent::CountdownTick { seconds_left: 1 }
        ));

        let result = room.advance_countdown(Duration::from_secs(2));
        assert!(result.emitted_events);
        assert!(result.finished);
        let events = room.drain_events();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, RoomEvent::CountdownFinished))
        );
    }

    #[test]
    fn countdown_state_reflects_remaining_seconds() {
        let mut room = Room::new();
        assert_eq!(room.countdown_seconds_left(), None);

        room.start_countdown(5);
        assert_eq!(room.countdown_seconds_left(), Some(5));

        room.advance_countdown(Duration::from_millis(1500));
        assert_eq!(room.countdown_seconds_left(), Some(4));

        room.advance_countdown(Duration::from_secs(5));
        assert_eq!(room.countdown_seconds_left(), None);
    }

    #[test]
    fn countdown_cancels_when_players_leave() {
        let mut room = Room::new();
        let now = Instant::now();
        assert!(room.add_member(1, "alpha".into()));
        assert!(room.add_member(2, "bravo".into()));
        room.start_countdown(3);
        room.drain_events(); // discard start event

        assert!(room.remove_member(2, "bravo".into(), now));
        let events = room.drain_events();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, RoomEvent::CountdownCancelled)),
            "expected countdown cancel event"
        );
        assert_eq!(room.countdown_seconds_left(), None);
    }
}
