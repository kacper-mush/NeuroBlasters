use std::collections::HashMap;

use common::{
    ai::{BotAgent, BotDifficulty},
    game::engine::GameEngine,
    protocol::{
        GameStateSnapshot, InputPayload, MapDefinition, PlayerId, PlayerState, RectWall, Team,
    },
};
use macroquad::prelude::*;

pub(crate) struct Server {
    game_engine: GameEngine,
    inputs: HashMap<PlayerId, InputPayload>,
    bots: Vec<(PlayerId, BotAgent)>,
}
pub(crate) struct ServerError;

impl Server {
    pub fn new() -> Self {
        let map = MapDefinition {
            width: 1600.0,
            height: 900.0,
            walls: vec![
                RectWall {
                    min: (410.0, 658.0).into(),
                    max: (1194.0, 720.0).into(),
                },
                RectWall {
                    min: (417.0, 173.0).into(),
                    max: (1170.0, 238.0).into(),
                },
                RectWall {
                    min: (1157.0, 386.0).into(),
                    max: (1358.0, 431.0).into(),
                },
                RectWall {
                    min: (1326.0, 527.0).into(),
                    max: (1537.0, 570.0).into(),
                },
                RectWall {
                    min: (100.0, 535.0).into(),
                    max: (321.0, 584.0).into(),
                },
                RectWall {
                    min: (259.0, 372.0).into(),
                    max: (504.0, 427.0).into(),
                },
                RectWall {
                    min: (787.0, 322.0).into(),
                    max: (828.0, 566.0).into(),
                },
            ],
        };

        let spawn_points = [
            (460.0, 822.0),
            (634.0, 818.0),
            (851.0, 823.0),
            (1095.0, 823.0),
            (1061.0, 77.0),
            (840.0, 70.0),
            (666.0, 74.0),
            (479.0, 78.0),
        ];

        let default_player = PlayerState {
            id: PlayerId(1),
            position: (100.0, 100.0).into(),
            velocity: (0.0, 0.0).into(),
            rotation: 0.0,
            radius: 15.0,
            speed: 200.0,
            health: 100.0,
            weapon_cooldown: 0.0,
            team: Team::Blue,
        };

        let mut game_engine = GameEngine::new(map);
        let player_id = 1;
        let bot_id_range = (2, 8);
        let mut bot_vec = Vec::new();

        game_engine.add_player(PlayerState {
            id: PlayerId(player_id),
            position: spawn_points[0].into(),
            team: Team::Blue,
            ..default_player
        });

        for id in bot_id_range.0..=bot_id_range.1 {
            game_engine.add_player(PlayerState {
                id: PlayerId(id),
                position: spawn_points[(id - 1) as usize].into(),
                team: if id <= 4 { Team::Blue } else { Team::Red },
                ..default_player
            });
            bot_vec.push((PlayerId(id), BotAgent::new(BotDifficulty::Terminator, 2137)));
        }

        Self {
            game_engine,
            inputs: HashMap::new(),
            bots: bot_vec,
        }
    }

    pub fn connect(&self, servername: String) -> bool {
        servername == "sigma.net"
    }

    pub fn create_room(&self) -> bool {
        true
    }

    pub fn join_room(&self, room_code: u32) -> bool {
        room_code == 2137
    }

    pub fn get_player_list(&self) -> Result<Vec<String>, ServerError> {
        Ok(vec![
            "sigma1".into(),
            "xxxDestroyerxxx".into(),
            "sigma2".into(),
        ])
    }

    pub fn leave(&self) {}

    pub fn get_room_code(&self) -> Result<u32, ServerError> {
        Ok(2317)
    }

    /// Attempt to start the game. Will fail if not the host.
    pub fn start_game(&mut self) -> bool {
        *self = Self::new(); // refresh the map and stuff
        true
    }

    /// Check if game already started
    pub fn game_started(&self) -> bool {
        false
    }

    pub fn get_map(&self) -> MapDefinition {
        self.game_engine.map.clone()
    }

    pub fn tick(&mut self) {
        let world = self.get_tick();
        let map = self.get_map();

        self.bots.iter_mut().for_each(|(id, bot)| {
            let player = self.game_engine.state.players.iter().find(|p| p.id == *id);
            if let Some(player) = player {
                let input = bot.generate_input(player, &world, &map, get_frame_time());
                self.inputs.insert(*id, input);
            }
        });

        self.game_engine.tick(get_frame_time(), &self.inputs);
    }

    pub fn get_tick(&self) -> GameStateSnapshot {
        self.game_engine.state.clone()
    }

    pub fn send_input(&mut self, input: InputPayload) {
        self.inputs.insert(PlayerId(1), input);
    }
}
