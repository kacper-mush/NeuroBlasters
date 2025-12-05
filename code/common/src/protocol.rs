use thiserror::Error;
// Message enums -------------------------------------------------------------

/// All messages that the client can send to the server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientMessage {
    /// Begin a session.
    Connect {
        api_version: u16,
        nickname: String,
    },
    /// Request graceful shutdown.
    Disconnect,
    /// Create a lobby.
    RoomCreate,
    /// Join an existing lobby by code.
    RoomJoin {
        room_code: RoomCode,
    },
    /// Leave the current lobby (only before the game starts).
    RoomLeave,
    /// Provide input for a future simulation tick.
    Input {
        tick_id: TickId,
        payload: InputPayload,
    },
}

/// All messages that the server can send to the client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerMessage {
    /// Confirm handshake.
    ConnectOk {
        session_id: SessionId,
        heartbeat_interval_ms: u32,
    },
    /// Lobby created.
    RoomCreateOk {
        room_code: RoomCode,
    },
    /// Successfully joined lobby.
    RoomJoinOk {
        state: RoomState,
    },
    /// Lobby state broadcast whenever player list / settings change.
    RoomState {
        state: RoomState,
    },
    /// Room leave result; players can't leave the room once game started.
    RoomLeaveOk,
    /// Game instance begins.
    GameStart {
        game_id: GameId,
        // ? teams: Vec<TeamAssignment>,
    },
    /// Game instance completes.
    GameEnd {
        game_id: GameId,
        result: GameResult,
    },
    /// Per-round start signal.
    RoundStart {
        round_id: RoundId,
        game_time_ms: u64,
    },
    /// Per-round finish signal.
    RoundEnd {
        round_id: RoundId,
        summary: RoundSummary,
    },
    /// Static map transfer.
    GameMap {
        game_id: GameId,
        map: MapDefinition,
    },
    /// Authoritative per-tick state.
    GameState {
        game_id: GameId,
        tick_id: TickId,
        state: GameStateSnapshot,
    },
    /// Unified error channel carrying all server → client errors.
    Error(ServerError),
}

// TODO: Add source errors for more complex errors.

/// All server → client errors carried through `ServerMessage::Error`.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ServerError {
    #[error("Handshake error")]
    Connect,
    /// Lobby creation failed.
    #[error("Room creation error")]
    RoomCreate,
    /// Joining a lobby failed.
    #[error("Room join error")]
    RoomJoin,
    /// Leaving a lobby failed.
    #[error("Room leave error")]
    RoomLeave,
    /// Input for a given tick was rejected.
    #[error("Input error at tick {tick_id:?}")]
    Input {
        tick_id: TickId,
    },
    /// Catch–all fatal / internal errors not covered by the categories above.
    #[error("Server error")]
    General,
}

// Concrete message payload structs -----------------------------------------

// Identifier newtypes -------------------------------------------------------

/// Protocol / API version negotiated at connect time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ApiVersion(pub u16);

/// Unique identifier of a client session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(pub u64);

/// Human–facing lobby code used to join rooms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoomCode(pub u64);

/// Unique identifier of a game instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GameId(pub u64);

/// Unique identifier of a round within a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoundId(pub u64);

/// Simulation tick identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TickId(pub u64);

/// Player identifier used across the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerId(pub u64);

/// Low–level client identifier (transport / connection).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(pub u64);

// Placeholder payload types -------------------------------------------------

/// Logical input payload sent from the client for a single tick.
///
/// To be defined: structure of commands, compression scheme, prediction keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputPayload {
    // TODO: define input fields (movement, abilities, etc.).
}

/// Full lobby / room state snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomState {
    // TODO: define room state (players, settings, phase, etc.).
}

/// Result of a completed game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameResult {
    // TODO: define game result (winning team, draw, cancellation reason, etc.).
}

/// Summary information for a completed round.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundSummary {
    // TODO: define per–round statistics.
}

/// Static map definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapDefinition {
    // TODO: define map id, bounds, shapes, spawn points, etc.
}

/// Authoritative per–tick game state snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameStateSnapshot {
    // TODO: define players, projectiles, pickups, events, etc.
}
