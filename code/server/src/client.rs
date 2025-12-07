use common::protocol::{ClientMessage, GameCode};
use crate::game_manager::GameManager;

// --- 1. THE STATE ---
// This is the "Memory" of the client.
#[derive(Clone, Debug)]
pub enum ClientState {
    /// Initial state. Socket connected, but no handshake yet.
    Handshaking,
    /// Authenticated and idle.
    Lobby { nickname: String },
    /// Currently inside a game instance.
    InGame {
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
    HandshakeCompleted { nickname: String },
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
            (ClientState::Handshaking, ClientEvent::HandshakeCompleted { nickname }) => {
                ClientState::Lobby { nickname }
            }

            // Lobby -> InGame (Join/Create)
            (ClientState::Lobby { nickname }, ClientEvent::GameJoined { game_code }) 
            | (ClientState::Lobby { nickname }, ClientEvent::GameCreated { game_code }) => {
                ClientState::InGame { nickname, game_code }
            }

            // InGame -> Lobby
            (ClientState::InGame { nickname, .. }, ClientEvent::GameLeft) => {
                ClientState::Lobby { nickname }
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

/// Takes an incoming message, checks the world state, and decides what Event (if any) occurs.
/// Returns:
/// - Ok(Some(Event)) -> Logic successful, apply this event.
/// - Ok(None) -> Logic successful, but state didn't change (e.g., chat message).
/// - Err(String) -> Logic failed (e.g., "Game Full"), send error to client.
pub fn handle_message(
    state: &ClientState,
    msg: ClientMessage,
    gm: GameManager,
    my_id: common::protocol::ClientId,
) -> Result<Option<ClientEvent>, String> {
    match (state, msg) {
        // --- HANDSHAKE ---
        (ClientState::Handshaking, ClientMessage::Handshake { api_version: _, nickname }) => {
            // TODO: Check API version
            Ok(Some(ClientEvent::HandshakeCompleted { nickname }))
        }

        // --- CREATE GAME ---
        (ClientState::Lobby { .. }, ClientMessage::CreateGame) => {
            let code = gm.create_game(my_id);
            Ok(Some(ClientEvent::GameCreated { game_code: code }))
        }

        // --- JOIN GAME ---
        (ClientState::Lobby { .. }, ClientMessage::JoinGame { game_code }) => {
            let code = GameCode(game_code);
            match gm.join_game(&code) {
                Ok(_) => Ok(Some(ClientEvent::GameJoined { game_code: code })),
                Err(e) => Err(e),
            }
        }

        // --- LEAVE GAME ---
        (ClientState::InGame { .. }, ClientMessage::LeaveGame) => {
            gm.remove_player_from_game(*my_id);
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