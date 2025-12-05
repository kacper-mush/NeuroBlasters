// Game logic module
use crate::{PlayerState, PlayerInput, Map, RectWall};
use glam::Vec2;

fn resolve_wall_collision(player: &mut PlayerState, wall: &RectWall) {
    let closest_point = player.position.clamp(wall.min, wall.max);
    let diff = player.position - closest_point;
    let dist_sq = diff.length_squared();

    // Case 1: Player is intersecting the edge (Standard case)
    if dist_sq < player.radius.powi(2) && dist_sq > 0.0001 { // Check > epsilon, not just 0.0
        let dist = dist_sq.sqrt();
        let overlap = player.radius - dist;
        let normal = diff / dist;
        player.position += normal * overlap;
    } 
    // Case 2: Player center is INSIDE the rectangle (Deep overlap)
    else if dist_sq <= 0.0001 {
        // We are deep inside. We need to push out to the nearest edge.
        // Calculate distances to all 4 edges
        let d_min_x = (player.position.x - wall.min.x).abs();
        let d_max_x = (player.position.x - wall.max.x).abs();
        let d_min_y = (player.position.y - wall.min.y).abs();
        let d_max_y = (player.position.y - wall.max.y).abs();

        // Find smallest distance
        let min_dist = d_min_x.min(d_max_x).min(d_min_y).min(d_max_y);

        // Push in the direction of that edge
        if min_dist == d_min_x { player.position.x = wall.min.x - player.radius; }
        else if min_dist == d_max_x { player.position.x = wall.max.x + player.radius; }
        else if min_dist == d_min_y { player.position.y = wall.min.y - player.radius; }
        else { player.position.y = wall.max.y + player.radius; }
    }
}

fn constrain_to_map(player: &mut PlayerState, map: &Map) {
    player.position.x = player.position.x.clamp(player.radius, map.width - player.radius);
    player.position.y = player.position.y.clamp(player.radius, map.height - player.radius);
}

pub fn apply_player_physics(player: &mut PlayerState, input: &PlayerInput, map: &Map, dt: f32) {
    // Velocity
    if input.move_axis.length_squared() > 0.0 {
        player.velocity = input.move_axis.normalize() * player.speed;
    } else {
        player.velocity = Vec2::ZERO;
    }

    // Movement
    player.position += player.velocity * dt;

    // Boundaries
    constrain_to_map(player, map);

    // Collisions
    for wall in &map.walls {
        resolve_wall_collision(player, wall);
    }
}

