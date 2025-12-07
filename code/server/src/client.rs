use common::protocol::{ClientMessage, GameCode, PlayerId};

// --- 1. THE STATE ---
// This is the "Memory" of the client.
#[derive(Clone, Debug)]
pub enum ClientState {
    /// Initial state. Socket connected, but no handshake yet.
    Handshaking,
    /// Authenticated and idle.
    Lobby { player_id: PlayerId, nickname: String },
    /// Currently inside a game instance.
    InGame {
        player_id: PlayerId,
        nickname: String,
        game_code: GameCode,
    },
}

impl Default for ClientState {
    fn default() -> Self {
        Self::Handshaking
    }
}

// --- 2. THE EVENTS ---
// "Facts" that have happened. These are infallible.
#[derive(Clone, Debug)]
pub enum ClientEvent {
    HandshakeCompleted { player_id: PlayerId, nickname: String },
    GameCreated { game_code: GameCode },
    GameJoined { game_code: GameCode },
    GameLeft,
    // Note: Disconnect isn't here because it destroys the state entirely.
}

// --- 3. THE LOGIC (DFA Application) ---
impl ClientState {
    /// The "reducer". Takes the current state + an event => new state.
    /// This function NEVER fails. It assumes the event is valid.
    pub fn apply(self, event: ClientEvent) -> Self {
        match (self, event) {
            // Handshake -> Lobby
            (ClientState::Handshaking, ClientEvent::HandshakeCompleted { player_id, nickname }) => {
                ClientState::Lobby { player_id, nickname }
            }

            // Lobby -> InGame (Join/Create)
            (ClientState::Lobby { player_id, nickname }, ClientEvent::GameJoined { game_code }) 
            | (ClientState::Lobby { player_id, nickname }, ClientEvent::GameCreated { game_code }) => {
                ClientState::InGame { player_id, nickname, game_code }
            }

            // InGame -> Lobby
            (ClientState::InGame { player_id, nickname, .. }, ClientEvent::GameLeft) => {
                ClientState::Lobby { player_id, nickname }
            }

            // Fallback for bugs (e.g., getting a 'GameJoined' event while still Handshaking)
            (state, event) => {
                tracing::error!("INVALID TRANSITION: State {:?} + Event {:?}", state, event);
                state
            }
        }
    }
}

// --- 4. THE DECISION LAYER (Validation) ---

/// Defines what the Client logic needs from the global Game Manager.
/// You will implement this in `main.rs` or `game_manager.rs`.
pub trait GameManager {
    fn create_game(&mut self, player_id: PlayerId) -> GameCode;
    fn join_game(&mut self, game_code: &GameCode, player_id: PlayerId) -> Result<(), String>;
    fn remove_player_from_game(&mut self, player_id: PlayerId);
}

/// Takes an incoming message, checks the world state, and decides what Event (if any) occurs.
/// Returns:
/// - Ok(Some(Event)) -> Logic successful, apply this event.
/// - Ok(None) -> Logic successful, but state didn't change (e.g., chat message).
/// - Err(String) -> Logic failed (e.g., "Game Full"), send error to client.
pub fn handle_message(
    state: &ClientState,
    msg: ClientMessage,
    gm: &mut impl GameManager,
) -> Result<Option<ClientEvent>, String> {
    match (state, msg) {
        // --- HANDSHAKE ---
        (ClientState::Handshaking, ClientMessage::Handshake { api_version: _, nickname }) => {
            // TODO: Check API version
            let player_id = PlayerId(rand::random());
            Ok(Some(ClientEvent::HandshakeCompleted { player_id, nickname }))
        }

        // --- CREATE GAME ---
        (ClientState::Lobby { player_id, .. }, ClientMessage::CreateGame) => {
            let code = gm.create_game(*player_id);
            Ok(Some(ClientEvent::GameCreated { game_code: code }))
        }

        // --- JOIN GAME ---
        (ClientState::Lobby { player_id, .. }, ClientMessage::JoinGame { game_code }) => {
            let code = GameCode(game_code);
            match gm.join_game(&code, *player_id) {
                Ok(_) => Ok(Some(ClientEvent::GameJoined { game_code: code })),
                Err(e) => Err(e),
            }
        }

        // --- LEAVE GAME ---
        (ClientState::InGame { player_id, .. }, ClientMessage::LeaveGame) => {
            gm.remove_player_from_game(*player_id);
            Ok(Some(ClientEvent::GameLeft))
        }

        // --- INVALID REQUESTS ---
        (ClientState::Handshaking, _) => Err("You must handshake first.".to_string()),
        (ClientState::InGame { .. }, ClientMessage::JoinGame { .. }) => Err("Already in a game.".to_string()),
        (ClientState::Lobby { .. }, ClientMessage::LeaveGame) => Err("Not in a game.".to_string()),
        
        // Catch-all
        _ => Err("Command not supported in this state.".to_string()),
    }
}