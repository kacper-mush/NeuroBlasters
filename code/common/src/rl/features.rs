use crate::ai::BotContext;
use crate::game::{FIRE_RATE, PROJECTILE_SPEED};
use crate::net::protocol::{RectWall, Tank};
use burn::tensor::backend::Backend;
use burn::tensor::{Tensor, TensorData};
use glam::Vec2;

// 2 (Self) + 9 (Enemies) + 6 (Friends) + 5 (Bullet) + 8 (Walls) = 30
pub const FEATURE_COUNT: usize = 30;

const SENSOR_MAX_DIST: f32 = 500.0; // Vision range

pub fn extract_features<B: Backend>(ctx: &BotContext, device: &B::Device) -> Tensor<B, 2> {
    let mut features = Vec::with_capacity(FEATURE_COUNT);

    // --- TRANSFORMATION HELPERS ---
    // We rotate everything into the Bot's local perspective.
    // X+ is Forward, Y+ is Right.
    let rot = ctx.me.rotation;
    let (sin, cos) = rot.sin_cos();

    let to_local = |world_pos: Vec2| -> Vec2 {
        let diff = world_pos - ctx.me.position;
        Vec2::new(
            diff.x * cos + diff.y * sin,  // Forward/Back
            -diff.x * sin + diff.y * cos, // Left/Right
        )
    };

    // --- 1. SELF STATE (2 inputs) ---
    features.push((ctx.me.health / 100.0).clamp(0.0, 1.0));
    features.push((ctx.me.weapon_cooldown / FIRE_RATE).clamp(0.0, 1.0));

    // --- 2. ENEMIES (3 Nearest) (9 inputs) ---
    // We explicitly sort ALL enemies by distance
    let mut enemies: Vec<&Tank> = ctx
        .players
        .iter()
        .filter(|p| {
            p.player_info.id != ctx.me.player_info.id
                && p.player_info.team != ctx.me.player_info.team
                && p.health > 0.0
        })
        .collect();

    enemies.sort_by(|a, b| {
        ctx.me
            .position
            .distance_squared(a.position)
            .partial_cmp(&ctx.me.position.distance_squared(b.position))
            .unwrap()
    });

    for i in 0..3 {
        if let Some(enemy) = enemies.get(i) {
            let local_pos = to_local(enemy.position);
            let dist = local_pos.length();

            // Normalize distance (1.0 = very close, 0.0 = far away)
            let normalized_dist = (1.0 - dist / SENSOR_MAX_DIST).clamp(0.0, 1.0);

            features.push(normalized_dist);
            features.push(local_pos.normalize_or_zero().x); // Direction X (Front)
            features.push(local_pos.normalize_or_zero().y); // Direction Y (Side)
        } else {
            // Placeholder for missing enemy
            features.push(0.0);
            features.push(0.0);
            features.push(0.0);
        }
    }

    // --- 3. TEAMMATES (2 Nearest) (6 inputs) ---
    let mut friends: Vec<&Tank> = ctx
        .players
        .iter()
        .filter(|p| {
            p.player_info.id != ctx.me.player_info.id
                && p.player_info.team == ctx.me.player_info.team
                && p.health > 0.0
        })
        .collect();

    friends.sort_by(|a, b| {
        ctx.me
            .position
            .distance_squared(a.position)
            .partial_cmp(&ctx.me.position.distance_squared(b.position))
            .unwrap()
    });

    for i in 0..2 {
        if let Some(friend) = friends.get(i) {
            let local_pos = to_local(friend.position);
            let dist = local_pos.length();
            features.push((1.0 - dist / SENSOR_MAX_DIST).clamp(0.0, 1.0));
            features.push(local_pos.normalize_or_zero().x);
            features.push(local_pos.normalize_or_zero().y);
        } else {
            features.push(0.0);
            features.push(0.0);
            features.push(0.0);
        }
    }

    // --- 4. NEAREST DANGEROUS BULLET (5 inputs) ---
    // Only care about bullets not owned by me
    let nearest_bullet = ctx
        .projectiles
        .iter()
        .filter(|p| p.owner_info.id != ctx.me.player_info.id)
        .min_by(|a, b| {
            ctx.me
                .position
                .distance_squared(a.position)
                .partial_cmp(&ctx.me.position.distance_squared(b.position))
                .unwrap()
        });

    if let Some(bullet) = nearest_bullet {
        let local_pos = to_local(bullet.position);
        let dist = local_pos.length();

        // Rotate velocity too
        let vel = bullet.velocity;
        let local_vel = Vec2::new(vel.x * cos + vel.y * sin, -vel.x * sin + vel.y * cos);

        features.push((1.0 - dist / SENSOR_MAX_DIST).clamp(0.0, 1.0));
        features.push(local_pos.normalize_or_zero().x);
        features.push(local_pos.normalize_or_zero().y);
        features.push(local_vel.x / PROJECTILE_SPEED);
        features.push(local_vel.y / PROJECTILE_SPEED);
    } else {
        features.push(0.0);
        features.push(0.0);
        features.push(0.0);
        features.push(0.0);
        features.push(0.0);
    }

    // --- 5. WALL SENSORS (LIDAR) (8 inputs) ---
    // Cast rays in 8 directions relative to bot
    let directions = [
        Vec2::new(1.0, 0.0),   // Front
        Vec2::new(0.7, 0.7),   // Front-Right
        Vec2::new(0.0, 1.0),   // Right
        Vec2::new(-0.7, 0.7),  // Back-Right
        Vec2::new(-1.0, 0.0),  // Back
        Vec2::new(-0.7, -0.7), // Back-Left
        Vec2::new(0.0, -1.0),  // Left
        Vec2::new(0.7, -0.7),  // Front-Left
    ];

    for local_dir in directions {
        // Rotate local direction to world direction for the raycast
        let world_dir = Vec2::new(
            local_dir.x * cos - local_dir.y * sin,
            local_dir.x * sin + local_dir.y * cos,
        );
        features.push(raycast_normalized(ctx, world_dir));
    }

    let data = TensorData::new(features, [1, FEATURE_COUNT]);
    Tensor::from_data(data, device)
}

// --- HELPERS ---

fn raycast_normalized(ctx: &BotContext, direction: Vec2) -> f32 {
    let origin = ctx.me.position;
    let mut min_dist = SENSOR_MAX_DIST;

    // 1. Check Internal Obstacles
    for wall in &ctx.map.walls {
        if let Some(dist) = ray_aabb_intersect(origin, direction, wall) && dist < min_dist {
            min_dist = dist;
        }
    }

    // 2. CHECK MAP BOUNDARIES (New Code)
    // We treat the map edges as infinite walls for the sensors

    // Check Left Wall (x = 0)
    if direction.x < 0.0 {
        let dist = -origin.x / direction.x;
        if dist > 0.0 && dist < min_dist {
            min_dist = dist;
        }
    }
    // Check Right Wall (x = width)
    if direction.x > 0.0 {
        let dist = (ctx.map.width - origin.x) / direction.x;
        if dist > 0.0 && dist < min_dist {
            min_dist = dist;
        }
    }
    // Check Top Wall (y = 0)
    if direction.y < 0.0 {
        let dist = -origin.y / direction.y;
        if dist > 0.0 && dist < min_dist {
            min_dist = dist;
        }
    }
    // Check Bottom Wall (y = height)
    if direction.y > 0.0 {
        let dist = (ctx.map.height - origin.y) / direction.y;
        if dist > 0.0 && dist < min_dist {
            min_dist = dist;
        }
    }

    // Invert result: 1.0 = Wall is touching us, 0.0 = Far
    (1.0 - min_dist / SENSOR_MAX_DIST).clamp(0.0, 1.0)
}

fn ray_aabb_intersect(origin: Vec2, dir: Vec2, wall: &RectWall) -> Option<f32> {
    let inv_dir = 1.0 / dir;

    let t1 = (wall.min.x - origin.x) * inv_dir.x;
    let t2 = (wall.max.x - origin.x) * inv_dir.x;
    let t3 = (wall.min.y - origin.y) * inv_dir.y;
    let t4 = (wall.max.y - origin.y) * inv_dir.y;

    let tmin = t1.min(t2).max(t3.min(t4));
    let tmax = t1.max(t2).min(t3.max(t4));

    if tmax >= tmin && tmax >= 0.0 {
        if tmin < 0.0 { Some(0.0) } else { Some(tmin) }
    } else {
        None
    }
}
