pub mod simulation;
mod state_countdown;
mod state_ended;
mod state_started;
mod state_waiting;

use std::collections::HashSet;
use std::time::{Duration, Instant};

use common::{
    CountdownError, GameCode, GameCreateError, GameEvent, GameJoinError, GameLeaveError,
    GameMember, GameResult, GameStateSnapshot, GameStateUpdate, GameUpdate, InputPayload,
    MapDefinition, ServerError, ServerMessage, Team, TickId,
};
use rand::{RngCore, rngs::StdRng};
use renet::ClientId;
use tracing::error;

use crate::connection::SessionInfo;
use crate::{GAME_CODE_ALPHABET, GAME_CODE_LENGTH, GAME_IDLE_TIMEOUT, ServerApp};

use self::simulation::GameInstance;
use self::state_countdown::{CountdownAdvance, CountdownState};
use self::state_ended::EndedState;
use self::state_started::{StartedAdvance, StartedState};
use self::state_waiting::WaitingState;

pub const MIN_PLAYERS_TO_START: usize = 2;

#[derive(Debug, Clone)]
pub enum TickState {
    Waiting,
    Countdown {
        seconds_left: u32,
    },
    Started {
        snapshot: GameStateSnapshot,
    },
    Ended {
        winner: Option<Team>,
        snapshot: GameStateSnapshot,
    },
}

pub struct GameTick {
    pub tick_id: TickId,
    pub member_ids: Vec<ClientId>,
    pub state: TickState,
    pub events: Vec<GameEvent>,
    pub map: Option<MapDefinition>,
}

pub struct Game {
    members: HashSet<ClientId>,
    pending_events: Vec<GameEvent>,
    state: GameState,
    empty_since: Option<Instant>,
    tick_id: TickId,
}

enum GameState {
    Waiting(WaitingState),
    Countdown(CountdownState),
    Started(StartedState),
    Ended(EndedState),
}

struct WaitingView<'a> {
    state: &'a mut WaitingState,
    members: &'a mut HashSet<ClientId>,
    pending_events: &'a mut Vec<GameEvent>,
    empty_since: &'a mut Option<Instant>,
}

impl<'a> WaitingView<'a> {
    fn add_member(&mut self, client_id: ClientId, nickname: String) -> bool {
        self.state.add_member(
            self.members,
            self.pending_events,
            self.empty_since,
            client_id,
            nickname,
        )
    }

    fn start_countdown(&mut self, seconds: u32) -> CountdownState {
        self.state.start_countdown(self.pending_events, seconds)
    }
}

impl Default for Game {
    fn default() -> Self {
        Self {
            members: HashSet::new(),
            pending_events: Vec::new(),
            state: GameState::Waiting(WaitingState),
            empty_since: None,
            tick_id: TickId(0),
        }
    }
}

impl Game {
    pub fn new() -> Self {
        Self::default()
    }

    fn waiting_view(&mut self) -> Option<WaitingView<'_>> {
        match &mut self.state {
            GameState::Waiting(state) => Some(WaitingView {
                state,
                members: &mut self.members,
                pending_events: &mut self.pending_events,
                empty_since: &mut self.empty_since,
            }),
            _ => None,
        }
    }

    fn started_state_mut(&mut self) -> Option<&mut StartedState> {
        match &mut self.state {
            GameState::Started(state) => Some(state),
            _ => None,
        }
    }

    pub fn remove_member(&mut self, client_id: ClientId, nickname: String, now: Instant) -> bool {
        if !self.members.remove(&client_id) {
            return false;
        }

        self.pending_events.push(GameEvent::PlayerLeft { nickname });

        match &mut self.state {
            GameState::Countdown(state) => {
                if matches!(
                    state.handle_player_left(self.members.len()),
                    CountdownAdvance::Cancelled
                ) {
                    self.cancel_countdown();
                }
            }
            GameState::Started(state) => {
                if let StartedAdvance::Ended { winner, snapshot } =
                    state.handle_player_left(client_id)
                {
                    self.transition_to_ended(winner, snapshot);
                }
            }
            _ => {}
        }

        if self.members.is_empty() {
            self.empty_since = Some(now);
        }
        true
    }

    pub fn advance(&mut self, delta: Duration, rng: &mut StdRng) -> Option<GameTick> {
        if self.members.is_empty() {
            return None;
        }

        let mut map = None;

        match &mut self.state {
            GameState::Waiting(_) => {}
            GameState::Countdown(state) => {
                match state.advance(delta, self.members.len(), &mut self.pending_events) {
                    CountdownAdvance::Finished => {
                        map = self.start_game(rng);
                    }
                    CountdownAdvance::Cancelled => {
                        self.cancel_countdown();
                    }
                    CountdownAdvance::Continue => {}
                }
            }
            GameState::Started(started) => {
                if let StartedAdvance::Ended { winner, snapshot } =
                    started.advance(delta, &mut self.pending_events)
                {
                    self.transition_to_ended(winner, snapshot);
                }
            }
            GameState::Ended(_) => {}
        }

        let tick_id = self.next_tick_id();
        let events = std::mem::take(&mut self.pending_events);

        Some(GameTick {
            tick_id,
            member_ids: self.member_ids(),
            state: self.current_state_snapshot(),
            events,
            map,
        })
    }

    pub fn should_remove(&self, now: Instant, timeout: Duration) -> bool {
        self.members.is_empty()
            && self
                .empty_since
                .is_some_and(|since| now.duration_since(since) >= timeout)
    }

    pub fn member_ids(&self) -> Vec<ClientId> {
        self.members.iter().copied().collect()
    }

    fn current_state_snapshot(&self) -> TickState {
        match &self.state {
            GameState::Waiting(_) => TickState::Waiting,
            GameState::Countdown(state) => TickState::Countdown {
                seconds_left: state.timer.seconds_left(),
            },
            GameState::Started(state) => TickState::Started {
                snapshot: state.snapshot(),
            },
            GameState::Ended(state) => TickState::Ended {
                winner: state.winner,
                snapshot: state.snapshot.clone(),
            },
        }
    }

    fn next_tick_id(&mut self) -> TickId {
        let id = self.tick_id;
        self.tick_id.0 += 1;
        id
    }

    fn start_game(&mut self, rng: &mut StdRng) -> Option<MapDefinition> {
        match GameInstance::start(&self.member_ids(), rng) {
            Ok(instance) => {
                let map = instance.get_map().clone();
                self.pending_events.push(GameEvent::GameStarted);
                self.state = GameState::Started(StartedState { instance });
                Some(map)
            }
            Err(err) => {
                error!(?err, "failed to start game");
                self.pending_events.push(GameEvent::CountdownCancelled);
                self.state = GameState::Waiting(WaitingState);
                None
            }
        }
    }

    fn cancel_countdown(&mut self) {
        if matches!(self.state, GameState::Countdown(_)) {
            self.pending_events.push(GameEvent::CountdownCancelled);
            self.state = GameState::Waiting(WaitingState);
        }
    }

    fn transition_to_ended(&mut self, winner: Option<Team>, snapshot: GameStateSnapshot) {
        if matches!(self.state, GameState::Ended(_)) {
            return;
        }
        self.pending_events.push(GameEvent::GameEnded { winner });
        self.state = GameState::Ended(EndedState { winner, snapshot });
    }
}

impl ServerApp {
    pub(super) fn update_games(&mut self, delta: Duration) {
        let now = Instant::now();
        let mut ticks = Vec::new();

        for (code, game) in self.games.iter_mut() {
            if let Some(tick) = game.advance(delta, &mut self.rng) {
                ticks.push((code.clone(), tick));
            }
        }

        for (code, tick) in ticks {
            self.dispatch_game_tick(&code, tick);
        }

        self.games
            .retain(|_, game| !game.should_remove(now, GAME_IDLE_TIMEOUT));
    }

    pub(super) fn broadcast_game_update(&mut self, game_code: &GameCode) {
        let Some(game) = self.games.get_mut(game_code) else {
            return;
        };

        if let Some(tick) = game.advance(Duration::ZERO, &mut self.rng) {
            self.dispatch_game_tick(game_code, tick);
        }
    }

    pub(super) fn detach_client_from_game(&mut self, client_id: ClientId) -> Option<GameCode> {
        let (game_code, nickname) = {
            let session = self.sessions.get_mut(&client_id)?;
            let code = session.game_code.take()?;
            (code, session.nickname.clone())
        };

        if let Some(game) = self.games.get_mut(&game_code) {
            game.remove_member(client_id, nickname, Instant::now());
        }
        Some(game_code)
    }

    pub(super) fn handle_game_create(
        &mut self,
        client_id: ClientId,
        session: &SessionInfo,
    ) -> Result<(), GameCreateError> {
        if let Some(game_code) = &session.game_code {
            return Err(GameCreateError::AlreadyInGame {
                game_code: game_code.clone(),
            });
        }

        let game_code = self.generate_game_code();
        let mut game = Game::new();
        {
            let mut waiting = game
                .waiting_view()
                .expect("newly created game should be in waiting state");
            waiting.add_member(client_id, session.nickname.clone());
        }
        self.games.insert(game_code.clone(), game);

        if let Some(session) = self.sessions.get_mut(&client_id) {
            session.game_code = Some(game_code.clone());
        }

        self.send_message(
            client_id,
            ServerMessage::GameCreateOk {
                game_code: game_code.clone(),
            },
        );
        self.broadcast_game_update(&game_code);
        Ok(())
    }

    pub(super) fn handle_game_join(
        &mut self,
        client_id: ClientId,
        session: &SessionInfo,
        game_code: GameCode,
    ) -> Result<(), GameJoinError> {
        if let Some(current_game) = &session.game_code {
            return Err(GameJoinError::AlreadyInGame {
                game_code: current_game.clone(),
            });
        }

        if !Self::is_valid_game_code(&game_code) {
            return Err(GameJoinError::InvalidCode {
                game_code: game_code.clone(),
            });
        }

        let tick = {
            let game = self
                .games
                .get_mut(&game_code)
                .ok_or_else(|| GameJoinError::NotFound {
                    game_code: game_code.clone(),
                })?;

            {
                let mut waiting =
                    game.waiting_view()
                        .ok_or_else(|| GameJoinError::NotJoinable {
                            game_code: game_code.clone(),
                        })?;

                if !waiting.add_member(client_id, session.nickname.clone()) {
                    return Err(GameJoinError::AlreadyInGame { game_code });
                }
            }

            game.advance(Duration::ZERO, &mut self.rng)
        };

        if let Some(session) = self.sessions.get_mut(&client_id) {
            session.game_code = Some(game_code.clone());
        }

        if let Some(tick) = tick {
            let update = self.build_game_update(&tick);
            self.send_message(
                client_id,
                ServerMessage::GameJoinOk {
                    tick_id: tick.tick_id,
                    state: update.clone(),
                },
            );
            self.dispatch_game_tick_with_update(&game_code, tick, update);
        }
        Ok(())
    }

    pub(super) fn handle_game_leave(&mut self, client_id: ClientId) -> Result<(), GameLeaveError> {
        let game_code = {
            let session = self
                .sessions
                .get(&client_id)
                .ok_or(GameLeaveError::NotInGame)?;
            session.game_code.clone().ok_or(GameLeaveError::NotInGame)?
        };

        {
            let game = self
                .games
                .get(&game_code)
                .ok_or(GameLeaveError::NotInGame)?;
            if matches!(game.state, GameState::Started(_)) {
                return Err(GameLeaveError::GameInProgress);
            }
        }

        let Some(game_code) = self.detach_client_from_game(client_id) else {
            return Err(GameLeaveError::NotInGame);
        };

        self.send_message(client_id, ServerMessage::GameLeaveOk);
        self.broadcast_game_update(&game_code);
        Ok(())
    }

    pub(super) fn handle_game_start_countdown(
        &mut self,
        session: &SessionInfo,
        seconds: u32,
    ) -> Result<(), CountdownError> {
        if seconds == 0 {
            return Err(CountdownError::InvalidSeconds);
        }

        let game_code = session.game_code.clone().ok_or(CountdownError::NotInGame)?;

        {
            let game = self
                .games
                .get_mut(&game_code)
                .ok_or(CountdownError::NotInGame)?;

            let member_count = game.members.len();
            if member_count < MIN_PLAYERS_TO_START {
                return Err(CountdownError::NotEnoughPlayers);
            }
            let countdown_state = {
                let mut waiting = game.waiting_view().ok_or(CountdownError::NotWaiting)?;
                waiting.start_countdown(seconds)
            };
            game.state = GameState::Countdown(countdown_state);
        }

        self.broadcast_game_update(&game_code);
        Ok(())
    }

    pub(super) fn handle_input_message(
        &mut self,
        client_id: ClientId,
        session: &SessionInfo,
        tick_id: TickId,
        payload: InputPayload,
    ) -> Result<(), ServerError> {
        let game_code = session
            .game_code
            .as_ref()
            .ok_or(ServerError::Input { tick_id })?;

        let game = self
            .games
            .get_mut(game_code)
            .ok_or(ServerError::Input { tick_id })?;

        let Some(started) = game.started_state_mut() else {
            return Err(ServerError::Input { tick_id });
        };

        started.submit_input(client_id, payload);
        Ok(())
    }

    fn dispatch_game_tick(&mut self, game_code: &GameCode, tick: GameTick) {
        let update = self.build_game_update(&tick);
        self.dispatch_game_tick_with_update(game_code, tick, update);
    }

    fn dispatch_game_tick_with_update(
        &mut self,
        _game_code: &GameCode,
        tick: GameTick,
        update: GameUpdate,
    ) {
        if tick.member_ids.is_empty() {
            return;
        }

        let started = tick
            .events
            .iter()
            .any(|event| matches!(event, GameEvent::GameStarted));
        let ended_winner = tick.events.iter().find_map(|event| {
            if let GameEvent::GameEnded { winner } = event {
                Some(*winner)
            } else {
                None
            }
        });
        let map = tick.map.clone();

        for client_id in tick.member_ids {
            if started {
                self.send_message(client_id, ServerMessage::GameStart);
            }
            if let Some(map) = map.clone() {
                self.send_message(client_id, ServerMessage::GameMap { map });
            }
            if let Some(winner) = ended_winner {
                self.send_message(
                    client_id,
                    ServerMessage::GameEnd {
                        result: GameResult { winner },
                    },
                );
            }
            self.send_message(
                client_id,
                ServerMessage::GameUpdate {
                    tick_id: tick.tick_id,
                    update: update.clone(),
                },
            );
        }
    }

    fn build_game_update(&self, tick: &GameTick) -> GameUpdate {
        GameUpdate {
            members: self.build_members(&tick.member_ids),
            state: self.build_game_state_update(&tick.state),
            events: tick.events.clone(),
        }
    }

    fn build_game_state_update(&self, tick_state: &TickState) -> GameStateUpdate {
        match tick_state {
            TickState::Waiting => GameStateUpdate::Waiting,
            TickState::Countdown { seconds_left } => GameStateUpdate::Countdown {
                countdown_seconds_left: *seconds_left,
            },
            TickState::Started { snapshot } => GameStateUpdate::Started {
                snapshot: snapshot.clone(),
            },
            TickState::Ended { winner, snapshot } => GameStateUpdate::Ended {
                winner: *winner,
                snapshot: snapshot.clone(),
            },
        }
    }

    fn build_members(&self, member_ids: &[ClientId]) -> Vec<GameMember> {
        let mut members: Vec<GameMember> = member_ids
            .iter()
            .filter_map(|client_id| self.sessions.get(client_id))
            .map(|session| GameMember {
                session_id: session.session_id,
                nickname: session.nickname.clone(),
            })
            .collect();
        members.sort_by(|a, b| a.nickname.cmp(&b.nickname));
        members
    }

    fn generate_game_code(&mut self) -> GameCode {
        loop {
            let code: String = (0..GAME_CODE_LENGTH)
                .map(|_| {
                    let idx = (self.rng.next_u32() as usize) % GAME_CODE_ALPHABET.len();
                    GAME_CODE_ALPHABET[idx] as char
                })
                .collect();
            let game_code = GameCode(code);
            if !self.games.contains_key(&game_code) {
                return game_code;
            }
        }
    }

    fn is_valid_game_code(game_code: &GameCode) -> bool {
        game_code.0.len() == GAME_CODE_LENGTH
            && game_code
                .0
                .chars()
                .all(|c| c.is_ascii() && GAME_CODE_ALPHABET.contains(&(c as u8)))
    }
}
