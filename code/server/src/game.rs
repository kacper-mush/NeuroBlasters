use std::collections::VecDeque;
use common::protocol::{GameCode, InputPayload, MapDefinition, GameEvent};
use common::game::engine::GameEngine; // Your existing physics engine

pub enum GameState {
    Waiting {
        players: Vec<(ClientId, String)>,
    },
    Battle {
        // The engine owns the players and physics state now
        engine: GameEngine, 
    },
    Ended {
        winner: common::protocol::Team,
    },
}

pub enum GameCommand {
    Join(PlayerId),
    Leave(PlayerId),
    StartGame(PlayerId),
    Input(PlayerId, InputPayload),
}

pub struct Game {
    pub code: GameCode,
    pub state: GameState,
    
    pub command_queue: VecDeque<GameCommand>,
    pub outgoing_events: Vec<GameEvent>,
}

impl Game {
    pub fn new(code: GameCode) -> Self {
        Self {
            code,
            state: GameState::Waiting { players: Vec::new() },
            command_queue: VecDeque::new(),
            outgoing_events: Vec::new(),
        }
    }

    pub fn tick(&mut self, dt: f32) {
        // A. Clear previous tick's events
        self.outgoing_events.clear();

        // B. Process Input Commands
        while let Some(cmd) = self.command_queue.pop_front() {
            self.handle_command(cmd);
        }

        // C. Update Logic (The Simulation)
        self.update_simulation(dt);
    }

    fn handle_command(&mut self, cmd: GameCommand) {
        match (&mut self.state, cmd) {
            // --- WAITING LOGIC ---
            (GameState::Waiting { players }, GameCommand::Join(pid)) => {
                if !players.contains(&pid) {
                    players.push(pid);
                    self.outgoing_events.push(GameEvent::PlayerJoined(pid));
                }
            }
            
            (GameState::Waiting { players }, GameCommand::Leave(pid)) => {
                if let Some(pos) = players.iter().position(|x| *x == pid) {
                    players.remove(pos);
                    self.outgoing_events.push(GameEvent::PlayerLeft(pid));
                }
            }

            // TRANSITION: Waiting -> Battle
            (GameState::Waiting { players }, GameCommand::StartGame(_requester)) => {
                if players.len() >= 2 { // Min players check
                    // 1. Initialize the Engine
                    let map = MapDefinition::default(); // Load from file in reality
                    let mut engine = GameEngine::new(map.clone());
                    
                    // 2. Add existing players to the engine
                    for &p_id in players.iter() {
                         // You'll need a helper to create default PlayerState (spawn point etc)
                         // engine.add_player(create_player_state(p_id));
                    }

                    // 3. Switch State
                    self.state = GameState::Battle { engine };
                    
                    // 4. Notify Clients
                    self.outgoing_events.push(GameEvent::GameStarted(map));
                }
            }

            // --- BATTLE LOGIC ---
            (GameState::Battle { engine }, GameCommand::Input(pid, input)) => {
                 // Just buffer it. The engine will use it in step C.
                 // Note: You need to modify your engine to accept an input *buffer* // rather than a snapshot if you want smooth movement, 
                 // but for now, direct application is fine.
            }
            
            (GameState::Battle { engine }, GameCommand::Leave(pid)) => {
                // In battle, leaving means "Remove from physics"
                // engine.remove_player(pid);
                self.outgoing_events.push(GameEvent::PlayerLeft(pid));
            }

            // --- ENDED LOGIC ---
            (GameState::Ended { .. }, GameCommand::Leave(pid)) => {
                 self.outgoing_events.push(GameEvent::PlayerLeft(pid));
            }

            // Invalid commands are ignored (e.g. Input while Waiting)
            _ => {}
        }
    }

    fn update_simulation(&mut self, dt: f32) {
        if let GameState::Battle { engine } = &mut self.state {
            // 1. Tick the physics
            // We assume you have a way to pass the inputs collected this frame to the engine
            let kills = engine.tick(dt, &std::collections::HashMap::new()); 
            
            // 2. Check Win Condition
            if let Some(winner_team) = common::game::check_round_winner(&engine.state.players) {
                self.state = GameState::Ended { winner: winner_team };
                self.outgoing_events.push(GameEvent::GameEnded(winner_team));
            }
        }
    }
}