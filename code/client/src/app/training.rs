use crate::app::game::Game;
use crate::app::{AppContext, Transition, View, ViewId};
use crate::ui::CANONICAL_SCREEN_WIDTH;
use crate::ui::{self};
use ::rand::SeedableRng;
use ::rand::rngs::StdRng;
use burn::backend::Wgpu;

use common::ai::BotContext;
use common::game::engine::GameEngine;
use common::net::protocol::{InputPayload, MapDefinition, PlayerId, Tank};
use common::rl::{BotBrain, extract_features};
use glam::Vec2;
use macroquad::prelude::*;

type ClientBackend = Wgpu;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum TrainingMode {
    Spectator,
    HumanVsAi,
}

pub(crate) struct Training {
    game_engine: GameEngine,
    brain: BotBrain<ClientBackend>,
    mode: TrainingMode,
    human_id: Option<PlayerId>,
    rng: StdRng,
}

impl Training {
    pub fn new(brain: BotBrain<ClientBackend>, mode: TrainingMode) -> Self {
        let mut game_engine = GameEngine::new(MapDefinition::load());
        let spawn_points = &game_engine.map.spawn_points;
        let mut human_id = None;

        match mode {
            TrainingMode::Spectator => {
                for i in 0..4 {
                    if let Some((team, pos)) = spawn_points.get(i + 4) {
                        game_engine.tanks.push(Tank::new(
                            common::game::player::PlayerInfo::new(
                                i as PlayerId,
                                format!("Blue {}", i),
                                *team,
                            ),
                            *pos,
                        ));
                    }
                }
                for i in 0..4 {
                    if let Some((team, pos)) = spawn_points.get(i) {
                        game_engine.tanks.push(Tank::new(
                            common::game::player::PlayerInfo::new(
                                (i + 4) as PlayerId,
                                format!("Red {}", i),
                                *team,
                            ),
                            *pos,
                        ));
                    }
                }
            }
            TrainingMode::HumanVsAi => {
                if let Some((team, pos)) = spawn_points.get(4) {
                    let pid = 0;
                    human_id = Some(pid);
                    game_engine.tanks.push(Tank::new(
                        common::game::player::PlayerInfo::new(pid, "Player".into(), *team),
                        *pos,
                    ));
                }
                for i in 0..4 {
                    if let Some((team, pos)) = spawn_points.get(i) {
                        let pid = (i + 1) as PlayerId;
                        game_engine.tanks.push(Tank::new(
                            common::game::player::PlayerInfo::new(pid, format!("Bot {}", i), *team),
                            *pos,
                        ));
                    }
                }
            }
        }

        Self {
            game_engine,
            human_id,
            mode,
            brain,
            rng: StdRng::from_os_rng(),
        }
    }

    fn bot_action_to_input(actions: &[f32], ctx: &BotContext) -> InputPayload {
        let move_fwd = actions[0].tanh();
        let move_side = actions[1].tanh();
        let aim_fwd = actions[2];
        let aim_side = actions[3];
        let shoot_val = actions[4];

        let (sin, cos) = ctx.me.rotation.sin_cos();
        let world_move = Vec2::new(
            move_fwd * cos - move_side * sin,
            move_fwd * sin + move_side * cos,
        );
        let world_aim_dir = Vec2::new(
            aim_fwd * cos - aim_side * sin,
            aim_fwd * sin + aim_side * cos,
        );
        let final_aim_dir = if world_aim_dir.length_squared() < 0.001 {
            Vec2::new(cos, sin)
        } else {
            world_aim_dir.normalize()
        };
        let aim_pos = ctx.me.position + (final_aim_dir * 100.0);

        InputPayload {
            move_axis: world_move,
            aim_pos,
            shoot: shoot_val > 0.0,
        }
    }
}

impl View for Training {
    fn update(&mut self, _ctx: &mut AppContext) -> Transition {
        if is_key_pressed(KeyCode::R) {
            *self = Self::new(self.brain.clone(), self.mode);
            return Transition::None;
        }
        if is_key_pressed(KeyCode::Escape) {
            return Transition::Pop;
        }

        let dt = get_frame_time();
        let mut inputs = std::collections::HashMap::new();

        if let Some(hid) = self.human_id {
            let input = Game::gather_user_input(&self.game_engine);
            inputs.insert(hid, input);
        }

        for player in &self.game_engine.tanks {
            if player.health <= 0.0 {
                continue;
            }
            if Some(player.player_info.id) == self.human_id {
                continue;
            }

            let ctx = BotContext {
                me: player,
                players: &self.game_engine.tanks,
                projectiles: &self.game_engine.projectiles,
                map: &self.game_engine.map,
                dt,
                rng: &mut self.rng,
            };
            let output = self
                .brain
                .forward(extract_features(&ctx, &Default::default()));
            let values = output.into_data().to_vec::<f32>().unwrap();
            inputs.insert(
                player.player_info.id,
                Self::bot_action_to_input(&values, &ctx),
            );
        }

        self.game_engine.tick(dt, inputs);

        Transition::None
    }

    fn draw(&mut self, _ctx: &AppContext, _has_input: bool) {
        let x_mid = CANONICAL_SCREEN_WIDTH / 2.;

        Game::draw_game_board(&self.game_engine, self.human_id);

        let mode_str = match self.mode {
            TrainingMode::Spectator => "SPECTATOR",
            TrainingMode::HumanVsAi => "PLAYING",
        };
        ui::Text::new_scaled(20).draw(&format!("{} | Reset: R | Exit: ESC", mode_str), x_mid, 30.);
    }

    fn get_id(&self) -> ViewId {
        ViewId::Training
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_game_spectator() {
        let device = Default::default();
        let brain = BotBrain::<ClientBackend>::new(&device);

        let state = Training::new(brain, TrainingMode::Spectator);

        assert_eq!(state.mode, TrainingMode::Spectator);
        assert!(state.human_id.is_none());
        // 4 vs 4
        assert_eq!(state.game_engine.tanks.len(), 8);
    }

    #[test]
    fn test_init_game_human_vs_ai() {
        let device = Default::default();
        let brain = BotBrain::<ClientBackend>::new(&device);

        let state = Training::new(brain, TrainingMode::HumanVsAi);

        assert_eq!(state.mode, TrainingMode::HumanVsAi);
        assert_eq!(state.human_id, Some(0));
        assert_eq!(state.game_engine.tanks.len(), 5);
    }
}
