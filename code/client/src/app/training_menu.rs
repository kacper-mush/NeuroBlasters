use crate::app::{AppContext, Transition, View, ViewId};
use crate::ui::{
    BUTTON_H, BUTTON_W, Button, CANONICAL_SCREEN_MID_X, CANONICAL_SCREEN_MID_Y, Layout, TEXT_LARGE,
    Text,
};
use macroquad::prelude::*;

type ClientBackend = NdArray;

pub(crate) struct TrainingMenu {
    game_engine: GameEngine,
    bot_brain: Option<BotBrain<ClientBackend>>,
    rng: StdRng,
}

impl TrainingMenu {
    pub fn new() -> Self {
        let mut game_engine = GameEngine::new(MapDefinition::load());

        // FIX: Clone the spawn points first to avoid borrowing game_engine while mutating it
        let spawn_points = game_engine.map.spawn_points.clone();

        // 1. Spawn ALL Bots
        for (i, (team, pos)) in spawn_points.iter().enumerate() {
            let id = i as u64;
            let name = format!("{:?} Bot {}", team, id);
            game_engine.add_player(Player::new(id, name, *team, *pos));
        }

        // 2. Load the Model
        println!("Attempting to load model from assets/model...");
        let recorder = BinFileRecorder::<FullPrecisionSettings>::default();

        let bot_brain = match BotBrain::<ClientBackend>::new(&Default::default()).load_file(
            "assets/model",
            &recorder,
            &Default::default(),
        ) {
            Ok(brain) => {
                println!("Model loaded successfully!");
                Some(brain)
            }
            Err(e) => {
                eprintln!("Failed to load model: {}", e);
                None
            }
        };

        Self {
            game_engine,
            bot_brain,
            rng: StdRng::from_os_rng(),
        }
    }

    fn calc_transform(&self) -> (f32, f32, f32) {
        let (map_w, map_h) = (self.game_engine.map.width, self.game_engine.map.height);
        let (screen_w, screen_h) = (screen_width(), screen_height());
        let x_scaling = screen_w / map_w;
        let y_scaling = screen_h / map_h;

        if x_scaling < y_scaling {
            (x_scaling, 0., f32::abs(screen_h - map_h * x_scaling) / 2.)
        } else {
            (y_scaling, f32::abs(screen_w - map_w * y_scaling) / 2., 0.)
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

impl View for TrainingMenu {
    fn update(&mut self, _ctx: &mut AppContext) -> Transition {
        let dt = get_frame_time();

        let mut inputs = std::collections::HashMap::new();

        // RUN AI FOR EVERYONE
        if let Some(brain) = &self.bot_brain {
            // We need to iterate indices to avoid borrowing self.game_engine multiple times
            for player in &self.game_engine.players {
                if player.health <= 0.0 {
                    continue;
                }

                let ctx = BotContext {
                    me: player,
                    players: &self.game_engine.players,
                    projectiles: &self.game_engine.projectiles,
                    map: &self.game_engine.map,
                    dt,
                    rng: &mut self.rng,
                };

                let device = Default::default();
                let state = extract_features(&ctx, &device);

                let output = brain.forward(state);
                let values = output.into_data().to_vec::<f32>().unwrap();

                let input = Self::bot_action_to_input(&values, &ctx);
                inputs.insert(player.id, input);
            }
        }

        // Tick Game
        self.game_engine.tick(dt, &inputs);

        // Reset/Exit Logic
        if is_key_pressed(KeyCode::R) {
            *self = Self::new();
        }
        if is_key_pressed(KeyCode::Escape) {
            return Transition::Pop;
        }

        Transition::None
    }

    fn draw(&mut self, _ctx: &AppContext) {
        let x_mid = CANONICAL_SCREEN_MID_X;
        let y_mid = CANONICAL_SCREEN_MID_Y;
        let mut layout = Layout::new(y_mid - 50., 30.);

        Text::new_scaled(TEXT_LARGE).draw("Training coming soon!", x_mid, layout.next());
        layout.add(30.);

        self.back_clicked = Button::default()
            .draw_centered(x_mid, layout.next(), BUTTON_W, BUTTON_H, Some("Back"))
            .poll();
    }

    fn update(&mut self, _ctx: &mut AppContext) -> Transition {
        if self.back_clicked {
            Transition::Pop
        } else {
            Transition::None
        }
    }

    fn get_id(&self) -> ViewId {
        ViewId::TrainingMenu
    }
}
