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
pub(crate) enum ServerError {
    Error,
}

impl Server {
    pub fn new() -> Self {
        let map = MapDefinition {
            width: 1920.,
            height: 1080.,
            walls: vec![
                RectWall {
                    min: (200.0, 200.0).into(),
                    max: (300.0, 400.0).into(),
                },
                RectWall {
                    min: (500.0, 100.0).into(),
                    max: (700.0, 150.0).into(),
                },
            ],
        };

        let mut game_engine = GameEngine::new(map);
        game_engine.add_player(PlayerState {
            id: PlayerId(1),
            position: (100.0, 100.0).into(),
            velocity: (0.0, 0.0).into(),
            rotation: 0.0,
            radius: 15.0,
            speed: 200.0,
            health: 100.0,
            weapon_cooldown: 0.0,
            team: Team::Blue,
        });

        game_engine.add_player(PlayerState {
            id: PlayerId(2),
            position: (500.0, 500.0).into(),
            velocity: (0.0, 0.0).into(),
            rotation: 0.0,
            radius: 15.0,
            speed: 200.0,
            health: 100.0,
            weapon_cooldown: 0.0,
            team: Team::Red,
        });

        game_engine.add_player(PlayerState {
            id: PlayerId(3),
            position: (1000.0, 100.0).into(),
            velocity: (0.0, 0.0).into(),
            rotation: 0.0,
            radius: 15.0,
            speed: 200.0,
            health: 100.0,
            weapon_cooldown: 0.0,
            team: Team::Blue,
        });

        Self {
            game_engine,
            inputs: HashMap::new(),
            bots: vec![
                (PlayerId(2), BotAgent::new(BotDifficulty::Terminator, 2137)),
                (PlayerId(3), BotAgent::new(BotDifficulty::Turret, 222)),
            ],
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

    pub fn start_game(&self) -> bool {
        true
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
