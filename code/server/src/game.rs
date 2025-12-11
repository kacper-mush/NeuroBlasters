use common::game::engine::{GameEngine, GameTickResult};
use common::protocol::{ClientId, GameEvent, GameStateSnapshot, InputPayload, MapDefinition, Team};
use std::collections::HashMap;

type Players = HashMap<ClientId, String>;

pub struct Game {
    pub players: Players,
    state: GameState,
    pub outgoing_events: Vec<GameEvent>,
}

enum GameState {
    Waiting,
    Battle(BattleData),
    Ended(EndedData),
}

pub enum GameCommand {
    Join(ClientId, String),
    Leave(ClientId),
    StartGame(ClientId),
    Input(ClientId, InputPayload),
    Tick(f32),
}

struct BattleData {
    engine: GameEngine,
    inputs: HashMap<ClientId, InputPayload>,
}

impl BattleData {
    pub fn new(map: MapDefinition, _players: Players) -> Self {
        let engine = GameEngine::new(map.clone());

        // TODO: Add this functionality!
        // for (client_id, _) in players {
        //     engine.add_player(client_id);
        // }

        Self {
            engine,
            inputs: HashMap::new(),
        }
    }

    pub fn process_input(&mut self, client_id: ClientId, input: InputPayload) {
        self.inputs.insert(client_id, input);
    }

    pub fn tick(&mut self, dt: f32) -> GameTickResult {
        let result = self.engine.tick(dt, &self.inputs);
        self.inputs.clear();
        result
    }
}

struct EndedData {
    winner: Team,
}

impl EndedData {
    pub fn get_data(&self) -> Team {
        self.winner
    }
}

impl Game {
    pub fn new() -> Self {
        Self {
            players: HashMap::new(),
            state: GameState::Waiting,
            outgoing_events: Vec::new(),
        }
    }

    pub fn get_snapshot(&self) -> GameStateSnapshot {
        match &self.state {
            GameState::Waiting => GameStateSnapshot::Waiting,

            GameState::Battle(battle_data) => GameStateSnapshot::Battle {
                players: battle_data.engine.players.clone(),
                projectiles: battle_data.engine.projectiles.clone(),
            },

            GameState::Ended(ended_data) => {
                let winner = ended_data.get_data();
                GameStateSnapshot::Ended { winner }
            }
        }
    }

    pub fn tick(&mut self, dt: f32) {
        self.handle_command(GameCommand::Tick(dt))
            .expect("Handling a game tick while in battle should never fail");
    }

    pub fn handle_command(&mut self, cmd: GameCommand) -> Result<(), String> {
        match (&mut self.state, cmd) {
            (GameState::Waiting, GameCommand::Join(client_id, nickname)) => {
                if let std::collections::hash_map::Entry::Vacant(e) = self.players.entry(client_id)
                {
                    e.insert(nickname.clone());
                    self.outgoing_events.push(GameEvent::PlayerJoined(nickname));
                    Ok(())
                } else {
                    Err("Player already in game.".to_string())
                }
            }
            (GameState::Waiting, GameCommand::Leave(client_id))
            | (GameState::Ended(_), GameCommand::Leave(client_id)) => {
                match self.players.remove(&client_id) {
                    Some(nickname) => {
                        self.outgoing_events.push(GameEvent::PlayerLeft(nickname));
                        Ok(())
                    }
                    None => Err("Player was not in the game.".to_string()),
                }
            }

            (GameState::Waiting, GameCommand::StartGame(_requester)) => {
                if self.players.len() < 2 {
                    Err("At least 2 players needed to start the game".to_string())
                } else {
                    // TODO: pass a chosen map
                    let map = MapDefinition::load();
                    let battle_data = BattleData::new(map.clone(), self.players.clone());

                    self.outgoing_events.push(GameEvent::GameStarted(map));
                    self.state = GameState::Battle(battle_data);

                    Ok(())
                }
            }

            (GameState::Battle(battle_data), GameCommand::Input(client_id, input)) => {
                battle_data.process_input(client_id, input);
                Ok(())
            }

            (GameState::Battle(battle_data), GameCommand::Tick(dt)) => {
                let result = battle_data.tick(dt);
                let mut kill_events = result
                    .kills
                    .iter()
                    .map(|kill_event| GameEvent::Kill(kill_event.clone()))
                    .collect();
                self.outgoing_events.append(&mut kill_events);

                if let Some(winner) = result.winner {
                    self.outgoing_events.push(GameEvent::GameEnded(winner));
                    self.state = GameState::Ended(EndedData { winner });
                }

                Ok(())
            }

            (_, GameCommand::Tick(_)) => {
                // If not in battle, do nothing.
                Ok(())
            }

            _ => Err("Command not allowed in current game state".to_string()),
        }
    }
}
