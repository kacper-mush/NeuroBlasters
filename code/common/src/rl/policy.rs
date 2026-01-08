use crate::ai::{BotContext, Policy};
use crate::net::protocol::InputPayload;
use crate::rl::extract_features;
use crate::rl::model::BotBrain;
use burn::module::Module;
use burn_ndarray::NdArray;
use glam::Vec2;

type BackendType = NdArray;

use std::sync::Mutex;
use std::sync::Arc;

#[derive(Clone)]
pub struct RlPolicy {
    brain: Arc<Mutex<BotBrain<BackendType>>>,
}

impl Default for RlPolicy {
    fn default() -> Self {
        let device = Default::default();
        Self {
            brain: Arc::new(Mutex::new(BotBrain::new(&device))),
        }
    }
}

impl Policy for RlPolicy {
    fn compute_input(&mut self, ctx: &mut BotContext) -> InputPayload {
        let device = Default::default();
        let features_tensor = extract_features::<BackendType>(ctx, &device);

        // 3. Forward pass
        // We lock the brain
        let brain = self.brain.lock().unwrap();
        let output = brain.forward(features_tensor);

        // 4. Get data
        let values = output.into_data().to_vec::<f32>().expect("Should be f32");
        
        // 5. Interpret
        let move_x = values[0].clamp(-1.0, 1.0);
        let move_y = values[1].clamp(-1.0, 1.0);
        
        let aim_x = values[2];
        let aim_y = values[3];
        
        let shoot_val = values[4];
        let shoot = shoot_val > 0.0;

        let move_axis = Vec2::new(move_x, move_y);
        let aim_dir = Vec2::new(aim_x, aim_y).normalize_or_zero();
        
        // Aim position is relative to player or absolute? 
        // InputPayload usually takes world space aim_pos.
        // Our bot outputs direction. So we project it out.
        let aim_pos = ctx.me.position + aim_dir * 200.0;

        InputPayload {
            move_axis,
            aim_pos,
            shoot,
        }
    }
}
