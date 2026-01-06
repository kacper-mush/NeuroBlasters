use crate::ai::BotContext;
use crate::game::{FIRE_RATE, Player, RectWall};
use burn::tensor::backend::Backend;
use burn::tensor::{Tensor, TensorData};
use glam::Vec2;

pub const FEATURE_COUNT: usize = 12;

const SENSOR_MAX_DIST: f32 = 200.0;

pub fn extract_features<B: Backend>(ctx: &BotContext, device: &B::Device) -> Tensor<B, 2> {
    let mut features = Vec::with_capacity(FEATURE_COUNT);

    let map_diagonal = (ctx.map.width.powi(2) + ctx.map.height.powi(2)).sqrt();

    // --- PREPARE COORDINATE TRANSFORM ---
    // Rotates the world so "Up" is the direction the bot is facing.
    let rot = ctx.me.rotation;
    let (sin, cos) = rot.sin_cos();

    // Rotates a vector from World Space -> Bot Local Space
    let to_local = |v: Vec2| -> Vec2 { Vec2::new(v.x * cos + v.y * sin, -v.x * sin + v.y * cos) };

    // --- SELF STATUS ---
    features.push((ctx.me.health / 100.0).clamp(0.0, 1.0));
    features.push((ctx.me.weapon_cooldown / FIRE_RATE).clamp(0.0, 1.0));

    // Absolute position (still useful for general map awareness)
    features.push(ctx.me.position.x / ctx.map.width);
    features.push(ctx.me.position.y / ctx.map.height);

    // --- ENEMY SENSOR (RELATIVE) ---
    if let Some(enemy) = find_closest_enemy(ctx) {
        let diff = enemy.position - ctx.me.position;
        let dist = diff.length();

        let world_dir = diff / (dist + 0.001);
        let local_dir = to_local(world_dir);

        features.push((dist / map_diagonal).clamp(0.0, 1.0));
        features.push(local_dir.x); // +1 = In Front, -1 = Behind
        features.push(local_dir.y); // +1 = Right, -1 = Left
        features.push(1.0); // Enemy Found flag
    } else {
        features.push(1.0); // Max dist
        features.push(0.0);
        features.push(0.0);
        features.push(0.0); // No Enemy
    }

    // --- WALL SENSORS (RELATIVE) ---
    // Cast rays Front/Back/Left/Right relative to bot
    let front = Vec2::new(cos, sin);
    let right = Vec2::new(sin, -cos);

    features.push(raycast_normalized(ctx, front)); // Front
    features.push(raycast_normalized(ctx, -front)); // Back
    features.push(raycast_normalized(ctx, -right)); // Left
    features.push(raycast_normalized(ctx, right)); // Right

    let data = TensorData::new(features, [1, FEATURE_COUNT]);
    Tensor::from_data(data, device)
}

// --- Helpers ---

fn find_closest_enemy<'a>(ctx: &BotContext<'a>) -> Option<&'a Player> {
    ctx.players
        .iter()
        .filter(|p| p.id != ctx.me.id && p.team != ctx.me.team && p.health > 0.0)
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
        if let Some(dist) = ray_aabb_intersect(origin, direction, wall) {
            if dist < min_dist {
                min_dist = dist;
            }
        }
    }

    min_dist / SENSOR_MAX_DIST
}

fn ray_aabb_intersect(origin: Vec2, dir: Vec2, wall: &RectWall) -> Option<f32> {
    let mut t_min = 0.0f32;
    let mut t_max = f32::MAX;

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

    if t_min > t_max || t_max < 0.0 {
        return None;
    }
    Some(t_min.max(0.0))
}
