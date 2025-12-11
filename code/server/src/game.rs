use common::ai::{BotAgent, BotDifficulty};
use common::game::engine::{GameEngine, GameTickResult};
use common::protocol::{
    ClientId, GameEvent, GameStateSnapshot, InputPayload, MapDefinition, Player, Team,
};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
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
    bots: Vec<(ClientId, BotAgent)>,
}

impl BattleData {
    pub fn new(map: MapDefinition, mut players: Players) -> Result<Self, String> {
        if map.spawn_points.len() < players.len() {
            return Err("Too many players for this map".to_string());
        }

        // TODO: the whole bot functionality could be sent to a different file.
        let generate_bot_id = |player_ids: Vec<&ClientId>| -> ClientId {
            let mut rng = StdRng::from_os_rng();
            loop {
                let id: u64 = rng.random();
                if !player_ids.contains(&&id) {
                    break id;
                }
            }
        };

        // Fill up left spaces with bots
        let mut bots: Vec<(ClientId, BotAgent)> = Vec::new();

        for _ in 0..map.spawn_points.len() - players.len() {
            let bot_id = generate_bot_id(players.keys().collect());
            bots.push((bot_id, BotAgent::new(BotDifficulty::Wanderer, 222)));
            players.insert(bot_id, "Bot".to_string());
        }

        let mut engine = GameEngine::new(map.clone());

        let mut spawn_points = map.spawn_points.clone();
        let mut curr_team = Team::Blue;

        for (client_id, nickname) in players {
            let pos = spawn_points
                .iter()
                .position(|&(t, _)| t == curr_team)
                .unwrap();
            let (team, spawn) = spawn_points.remove(pos);
            engine.add_player(Player::new(client_id, nickname, team, spawn));
            curr_team = if curr_team == Team::Blue {
                Team::Red
            } else {
                Team::Blue
            };
        }

        Ok(Self {
            engine,
            inputs: HashMap::new(),
            bots,
        })
    }

    pub fn process_input(&mut self, client_id: ClientId, input: InputPayload) {
        self.inputs.insert(client_id, input);
    }

    pub fn tick(&mut self, dt: f32) -> GameTickResult {
        // Let each bot do their thing
        self.bots.iter_mut().for_each(|(id, bot)| {
            let player = self.engine.players.iter().find(|p| p.id == *id);
            if let Some(player) = player {
                let input = bot.generate_input(
                    player,
                    &self.engine.players,
                    &self.engine.projectiles,
                    &self.engine.map,
                    dt,
                );
                self.inputs.insert(*id, input);
            }
        });

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
        //self.outgoing_events.clear();
        if matches!(self.state, GameState::Battle(_)) {
            self.handle_command(GameCommand::Tick(dt))
                .expect("Handling a game tick should never fail");
        }
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
            (_, GameCommand::Leave(client_id)) => match self.players.remove(&client_id) {
                Some(nickname) => {
                    self.outgoing_events.push(GameEvent::PlayerLeft(nickname));
                    Ok(())
                }
                None => Err("Player was not in the game.".to_string()),
            },

            (GameState::Waiting, GameCommand::StartGame(_requester)) => {
                if self.players.len() < 2 {
                    Err("At least 2 players needed to start the game".to_string())
                } else {
                    // TODO: pass a map selection to be loaded
                    let map = MapDefinition::load();

                    let battle_data = BattleData::new(map.clone(), self.players.clone())?;

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
