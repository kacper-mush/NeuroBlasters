// common/src/game_logic.rs
use crate::protocol::{Map, PlayerInput, PlayerState, Projectile, RectWall};
use glam::Vec2;

// --- Helper Functions ---

fn resolve_wall_collision(position: &mut Vec2, radius: f32, wall: &RectWall) {
    let closest_point = position.clamp(wall.min, wall.max);
    let diff = *position - closest_point;
    let dist_sq = diff.length_squared();

    // Standard Collision
    if dist_sq < radius.powi(2) && dist_sq > 0.0001 {
        let dist = dist_sq.sqrt();
        let overlap = radius - dist;
        let normal = diff / dist;
        *position += normal * overlap;
    } 
    // Deep Overlap (Center inside wall)
    else if dist_sq <= 0.0001 {
        let d_min_x = (position.x - wall.min.x).abs();
        let d_max_x = (position.x - wall.max.x).abs();
        let d_min_y = (position.y - wall.min.y).abs();
        let d_max_y = (position.y - wall.max.y).abs();

        let min_dist = d_min_x.min(d_max_x).min(d_min_y).min(d_max_y);

        if min_dist == d_min_x { position.x = wall.min.x - radius; }
        else if min_dist == d_max_x { position.x = wall.max.x + radius; }
        else if min_dist == d_min_y { position.y = wall.min.y - radius; }
        else { position.y = wall.max.y + radius; }
    }
}

fn constrain_to_map(position: &mut Vec2, radius: f32, map: &Map) {
    position.x = position.x.clamp(radius, map.width - radius);
    position.y = position.y.clamp(radius, map.height - radius);
}

// --- Physics Logic ---

pub fn apply_player_physics(player: &mut PlayerState, input: &PlayerInput, map: &Map, dt: f32) {
    // 1. Movement
    if input.move_axis.length_squared() > 0.0 {
        player.velocity = input.move_axis.normalize() * player.speed;
    } else {
        player.velocity = Vec2::ZERO;
    }
    player.position += player.velocity * dt;

    // 2. Rotation
    let look_dir = input.aim_pos - player.position;
    if look_dir.length_squared() > 0.0 {
        player.rotation = look_dir.y.atan2(look_dir.x);
    }

    // 3. Collisions
    constrain_to_map(&mut player.position, player.radius, map);
    for wall in &map.walls {
        resolve_wall_collision(&mut player.position, player.radius, wall);
    }
}

pub fn update_projectiles(projectiles: &mut Vec<Projectile>, map: &Map, dt: f32) {
    projectiles.retain_mut(|proj| {
        proj.position += proj.velocity * dt;

        // Bounds Check
        if proj.position.x < 0.0 || proj.position.x > map.width ||
           proj.position.y < 0.0 || proj.position.y > map.height {
            return false;
        }

        // Wall Check
        for wall in &map.walls {
            let closest = proj.position.clamp(wall.min, wall.max);
            if (proj.position - closest).length_squared() < proj.radius.powi(2) {
                return false;
            }
        }
        true
    });
}