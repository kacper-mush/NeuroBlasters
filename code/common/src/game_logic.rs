use crate::protocol::{InputPayload, MapDefinition, PlayerState, Projectile, RectWall};
use glam::Vec2;

// --- Helper Functions ---

/// Resolves collision between a circular entity (player/projectile) and a rectangular wall.
///
/// Uses the "closest point on AABB" method:
/// 1. Find the point on the rectangle closest to the circle's center.
/// 2. Calculate the vector from that closest point to the center.
/// 3. If the length of that vector is less than the radius, there is a collision.
fn resolve_wall_collision(position: &mut Vec2, radius: f32, wall: &RectWall) {
    // 1. Find the closest point on the AABB to the circle center.
    // We clamp the circle's center coordinates to the wall's min/max bounds.
    let closest_point = position.clamp(wall.min, wall.max);

    // 2. Calculate the vector from the closest point to the circle center.
    let diff = *position - closest_point;

    // 3. Check distance squared to avoid expensive sqrt() if not needed.
    let dist_sq = diff.length_squared();

    // Standard Collision: The circle center is outside the wall, but the radius overlaps.
    if dist_sq < radius.powi(2) && dist_sq > 0.0001 {
        // Calculate actual distance and overlap depth.
        let dist = dist_sq.sqrt();
        let overlap = radius - dist;

        // Normal vector points from wall to player.
        let normal = diff / dist;

        // Push the player out of the wall along the normal.
        *position += normal * overlap;
    }
    // Deep Overlap: The circle center is INSIDE the wall (dist_sq is near zero).
    // This happens if the player spawns inside or moves too fast.
    else if dist_sq <= 0.0001 {
        // Calculate distances to all 4 edges to find the shortest path out.
        let d_min_x = (position.x - wall.min.x).abs();
        let d_max_x = (position.x - wall.max.x).abs();
        let d_min_y = (position.y - wall.min.y).abs();
        let d_max_y = (position.y - wall.max.y).abs();

        let min_dist = d_min_x.min(d_max_x).min(d_min_y).min(d_max_y);

        // Push to the nearest edge.
        if min_dist == d_min_x {
            position.x = wall.min.x - radius;
        } else if min_dist == d_max_x {
            position.x = wall.max.x + radius;
        } else if min_dist == d_min_y {
            position.y = wall.min.y - radius;
        } else {
            position.y = wall.max.y + radius;
        }
    }
}

/// Constrains a position to be within the map boundaries.
fn constrain_to_map(position: &mut Vec2, radius: f32, map: &MapDefinition) {
    // Simple AABB clamping. Ensure the circle stays strictly inside the map dimensions.
    position.x = position.x.clamp(radius, map.width - radius);
    position.y = position.y.clamp(radius, map.height - radius);
}

// --- Main Physics Logic ---

pub fn apply_player_physics(
    player: &mut PlayerState,
    input: &InputPayload,
    map: &MapDefinition,
    dt: f32,
) {
    // 1. Movement
    // Normalize the input vector to ensure diagonal movement isn't faster (length 1.0).
    if input.move_axis.length_squared() > 0.0 {
        player.velocity = input.move_axis.normalize() * player.speed;
    } else {
        player.velocity = Vec2::ZERO;
    }

    // Euler integration: pos = pos + vel * dt
    player.position += player.velocity * dt;

    // 2. Rotation
    // Calculate the direction vector from player to mouse cursor.
    let look_dir = input.aim_pos - player.position;
    if look_dir.length_squared() > 0.0 {
        // atan2(y, x) gives the angle in radians.
        player.rotation = look_dir.y.atan2(look_dir.x);
    }

    // 3. Boundaries & Collisions
    constrain_to_map(&mut player.position, player.radius, map);
    for wall in &map.walls {
        resolve_wall_collision(&mut player.position, player.radius, wall);
    }
}

pub fn update_projectiles(projectiles: &mut Vec<Projectile>, map: &MapDefinition, dt: f32) {
    projectiles.retain_mut(|proj| {
        proj.position += proj.velocity * dt;

        // Bounds Check
        if proj.position.x < 0.0
            || proj.position.x > map.width
            || proj.position.y < 0.0
            || proj.position.y > map.height
        {
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
