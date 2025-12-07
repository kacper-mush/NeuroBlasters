pub enum ClientState {
    // We have a socket, but don't know who this is yet
    Handshaking, 
    
    // We know who they are, they are idling
    Lobby { 
        player_id: PlayerID, 
        nickname: String 
    },
    
    // They are in a gameS
    InGame { 
        player_id: PlayerID, 
        nickname: String, 
        game_code: GameCode 
    },
}

// 2. INTERNAL EVENTS: Results of processing messages
pub enum InternalEvent {
    Authenticated { player_id: PlayerID, nickname: String },
    GameJoined { game_code: GameCode },
    GameLeft,
    Error(String), // Useful for "Game Full" or "Wrong Version"
}

impl ClientState {
    pub fn transition(self, event: InternalEvent) -> Self {
        match (self, event) {
            // 1. Handshake -> Lobby
            (ClientState::Handshaking, InternalEvent::Authenticated { player_id, nickname }) => {
                ClientState::Lobby { player_id, nickname }
            }

            // 2. Lobby -> InGame
            (ClientState::Lobby { player_id, nickname }, InternalEvent::GameJoined { game_code }) => {
                ClientState::InGame { player_id, nickname, game_code }
            }

            // 3. InGame -> Lobby (Leaving)
            (ClientState::InGame { player_id, nickname, .. }, InternalEvent::GameLeft) => {
                ClientState::Lobby { player_id, nickname }
            }

            // 5. Invalid Transitions (e.g., Joining game while in Handshaking)
            (state, event) => {
                println!("Invalid transition! State: {:?}, Event: {:?}", "...", "...");
                // Depending on severity, you might want to disconnect the client here
                state
            }
        }
    }
}