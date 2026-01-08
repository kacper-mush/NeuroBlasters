use crate::ai::BotContext;
use crate::game::{FIRE_RATE, RectWall, Tank};
use burn::tensor::backend::Backend;
use burn::tensor::{Tensor, TensorData};
use glam::Vec2;

pub const FEATURE_COUNT: usize = 12;

const SENSOR_MAX_DIST: f32 = 200.0;

pub fn extract_features<B: Backend>(ctx: &BotContext, device: &B::Device) -> Tensor<B, 2> {
    let mut features = Vec::with_capacity(FEATURE_COUNT);

    let map_diagonal = (ctx.map.width.powi(2) + ctx.map.height.powi(2)).sqrt();

    // Self Status
    features.push((ctx.me.health / 100.0).clamp(0.0, 1.0));

    // Cooldown Normalization:
    // 0.0 = Ready to fire
    // 1.0 = Just fired
    features.push((ctx.me.weapon_cooldown / FIRE_RATE).clamp(0.0, 1.0));

    // Position
    features.push(ctx.me.position.x / ctx.map.width);
    features.push(ctx.me.position.y / ctx.map.height);

    // Enemy Sensors
    if let Some(enemy) = find_closest_enemy(ctx) {
        let diff = enemy.position - ctx.me.position;
        let dist = diff.length();
        let dir = diff / (dist + 0.001);

        features.push((dist / map_diagonal).clamp(0.0, 1.0));
        features.push(dir.x);
        features.push(dir.y);
        features.push(1.0);
    } else {
        features.push(1.0);
        features.push(0.0);
        features.push(0.0);
        features.push(0.0);
    }

    // Wall Sensors
    features.push(raycast_normalized(ctx, Vec2::new(0.0, -1.0)));
    features.push(raycast_normalized(ctx, Vec2::new(0.0, 1.0)));
    features.push(raycast_normalized(ctx, Vec2::new(-1.0, 0.0)));
    features.push(raycast_normalized(ctx, Vec2::new(1.0, 0.0)));

    let data = TensorData::new(features, [1, FEATURE_COUNT]);
    Tensor::from_data(data, device)
}

// --- Helpers ---

fn find_closest_enemy<'a>(ctx: &BotContext<'a>) -> Option<&'a Tank> {
    ctx.players
        .iter()
        .filter(|p| {
            p.player_info.id != ctx.me.player_info.id
                && p.player_info.team != ctx.me.player_info.team
                && p.health > 0.0
        })
        .min_by(|a, b| {
            let da = ctx.me.position.distance_squared(a.position);
            let db = ctx.me.position.distance_squared(b.position);
            da.partial_cmp(&db).unwrap()
        })
}

fn raycast_normalized(ctx: &BotContext, direction: Vec2) -> f32 {
    let origin = ctx.me.position;
    let mut min_dist = SENSOR_MAX_DIST;

    for wall in &ctx.map.walls {
        if let Some(dist) = ray_aabb_intersect(origin, direction, wall)
            && dist < min_dist
        {
            min_dist = dist;
        }
    }

    min_dist / SENSOR_MAX_DIST
}

fn ray_aabb_intersect(origin: Vec2, dir: Vec2, wall: &RectWall) -> Option<f32> {
    let mut t_min = 0.0f32;
    let mut t_max = f32::MAX;

    // X axis
    if dir.x.abs() < 1e-6 {
        if origin.x < wall.min.x || origin.x > wall.max.x {
            return None;
        }
    } else {
        let t1 = (wall.min.x - origin.x) / dir.x;
        let t2 = (wall.max.x - origin.x) / dir.x;
        let (t_near, t_far) = if t1 < t2 { (t1, t2) } else { (t2, t1) };
        t_min = t_min.max(t_near);
        t_max = t_max.min(t_far);
    }

    // Y axis
    if dir.y.abs() < 1e-6 {
        if origin.y < wall.min.y || origin.y > wall.max.y {
            return None;
        }
    } else {
        let t1 = (wall.min.y - origin.y) / dir.y;
        let t2 = (wall.max.y - origin.y) / dir.y;
        let (t_near, t_far) = if t1 < t2 { (t1, t2) } else { (t2, t1) };
        t_min = t_min.max(t_near);
        t_max = t_max.min(t_far);
    }

    if t_min > t_max {
        return None;
    }

    if t_max < 0.0 {
        return None;
    }

    Some(t_min.max(0.0))
}
