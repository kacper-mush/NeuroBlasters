use crate::protocol::Team;
use crate::protocol::{InputPayload, KillEvent, MapDefinition, PlayerState, Projectile, RectWall};
use glam::Vec2;
use rand::Rng;

const PROJECTILE_SPEED: f32 = 500.0;
const PROJECTILE_RADIUS: f32 = 5.0;
const FIRE_RATE: f32 = 0.2; // Seconds between shots
const PROJECTILE_DAMAGE: f32 = 10.0;

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

/// Returns true if the position is safe (not inside any wall).
fn is_position_safe(pos: Vec2, radius: f32, map: &MapDefinition) -> bool {
    // 1. Check Map Boundaries
    if pos.x < radius || pos.x > map.width - radius || pos.y < radius || pos.y > map.height - radius
    {
        return false;
    }

    // 2. Check Walls
    for wall in &map.walls {
        // AABB expansion check.
        // We expand the wall by the player's radius. If the center of the player
        // is inside this expanded box, they are colliding.
        let min_safe = wall.min - Vec2::splat(radius);
        let max_safe = wall.max + Vec2::splat(radius);

        if pos.x >= min_safe.x && pos.x <= max_safe.x && pos.y >= min_safe.y && pos.y <= max_safe.y
        {
            return false;
        }
    }
    true
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

/// Handles weapon cooldown and bullet spawning.
/// Returns Some(Projectile) if a bullet was fired this frame.
pub fn handle_shooting(
    player: &mut PlayerState,
    input: &InputPayload,
    dt: f32,
    new_projectile_id: u64,
) -> Option<Projectile> {
    // 1. Tick down the cooldown
    if player.weapon_cooldown > 0.0 {
        player.weapon_cooldown -= dt;
    }

    // 2. Check if trying to shoot and cooldown is ready
    if input.shoot && player.weapon_cooldown <= 0.0 {
        // Reset cooldown
        player.weapon_cooldown = FIRE_RATE;

        // Calculate direction
        let aim_dir = (input.aim_pos - player.position).normalize_or_zero();

        if aim_dir == Vec2::ZERO {
            return None; // Don't shoot if aim is invalid (e.g. mouse exactly on player)
        }

        // Spawn bullet slightly in front of player so they don't hit themselves immediately
        let spawn_offset = aim_dir * (player.radius + PROJECTILE_RADIUS + 1.0);

        return Some(Projectile {
            id: new_projectile_id,
            owner_id: player.id,
            position: player.position + spawn_offset,
            velocity: aim_dir * PROJECTILE_SPEED,
            radius: PROJECTILE_RADIUS,
        });
    }

    None
}

/// Checks for collisions between projectiles and players.
///
/// 1. Removes projectiles that hit a player.
/// 2. Deals damage to the hit player.
/// 3. Returns a list of kills if any players died.
/// 4. Removes dead players from the list (so they vanish from the game).
pub fn resolve_combat(
    players: &mut Vec<PlayerState>,
    projectiles: &mut Vec<Projectile>,
) -> Vec<KillEvent> {
    let mut kills = Vec::new();

    // 1. Process Projectile Collisions
    // We use retain() to filter out bullets that hit something.
    projectiles.retain(|proj| {
        let mut hit_someone = false;

        for player in players.iter_mut() {
            // Don't hit yourself
            if player.id == proj.owner_id {
                continue;
            }

            // Simple Circle-Circle Collision
            let dist_sq = player.position.distance_squared(proj.position);
            let sum_radii = player.radius + proj.radius;

            if dist_sq < sum_radii * sum_radii {
                // COLLISION DETECTED
                player.health -= PROJECTILE_DAMAGE;

                // Check for death immediately (so we know who killed them)
                // We mark them as "dead" here, but remove them in step 2.
                if player.health <= 0.0 {
                    kills.push(KillEvent {
                        killer_id: proj.owner_id,
                        victim_id: player.id,
                    });
                }

                hit_someone = true;
                break; // Bullet hits the first player it touches, then disappears
            }
        }

        !hit_someone // Keep the bullet if it DID NOT hit anyone
    });

    // 2. Remove Dead Players
    // We only keep players who are still alive (health > 0).
    players.retain(|p| p.health > 0.0);

    kills
}

/// Finds a spawn position using a hybrid Random + Grid Scan approach.
pub fn find_spawn_position(
    map: &MapDefinition,
    player_radius: f32,
    rng: &mut impl Rng,
) -> Option<Vec2> {
    // --- Phase 1: Fast Random Guessing ---
    // Try random spots first.
    let max_attempts = 50000;
    for _ in 0..max_attempts {
        let x = rng.random_range(player_radius..map.width - player_radius);
        let y = rng.random_range(player_radius..map.height - player_radius);
        let candidate = Vec2::new(x, y);

        if is_position_safe(candidate, player_radius, map) {
            return Some(candidate);
        }
    }

    // --- Phase 2: Deterministic Grid Scan ---
    // If we are very unlucky which is possible if the map is almost full of "walls", scan systematically.
    // We step by the half of player radius to ensure we don't miss any "player-sized" gaps.
    // This should basically never happen on well design maps.
    let step = player_radius / 2.0;

    let mut y = player_radius;
    while y <= map.height - player_radius {
        let mut x = player_radius;
        while x <= map.width - player_radius {
            let candidate = Vec2::new(x, y);

            if is_position_safe(candidate, player_radius, map) {
                return Some(candidate);
            }

            x += step;
        }
        y += step;
    }

    None // there is no spawnable place on a map
}

/// Checks if one team has been eliminated.
/// Returns Some(Team) if a team has won (opponent wiped out), or None if the battle continues.
pub fn check_round_winner(players: &[PlayerState]) -> Option<Team> {
    let mut blue_alive = 0;
    let mut red_alive = 0;

    for p in players {
        // We assume players with health <= 0 are already removed by resolve_combat,
        // but checking > 0 doesn't hurt.
        if p.health > 0.0 {
            match p.team {
                Team::Blue => blue_alive += 1,
                Team::Red => red_alive += 1,
            }
        }
    }

    // If Blue is wiped out and Red is standing, Red wins.
    if blue_alive == 0 && red_alive > 0 {
        return Some(Team::Red);
    }

    // If Red is wiped out and Blue is standing, Blue wins.
    if red_alive == 0 && blue_alive > 0 {
        return Some(Team::Blue);
    }

    // If both are alive (or both died same tick), round continues.
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{PlayerId, RectWall, Team};
    use glam::Vec2;
    #[allow(deprecated)]
    use rand::rngs::mock::StepRng; // Or use a seeded StdRng

    // --- Helper to create dummy players ---
    fn make_player(id: u64, team: Team, pos: Vec2) -> PlayerState {
        PlayerState {
            id: PlayerId(id),
            team,
            position: pos,
            velocity: Vec2::ZERO,
            rotation: 0.0,
            radius: 10.0,
            speed: 100.0,
            health: 100.0,
            weapon_cooldown: 0.0,
        }
    }

    // --- Helper to create dummy map ---
    fn make_map() -> MapDefinition {
        MapDefinition {
            width: 1000.0,
            height: 1000.0,
            walls: vec![RectWall {
                min: Vec2::new(400.0, 400.0),
                max: Vec2::new(600.0, 600.0),
            }],
        }
    }

    #[test]
    fn test_shooting_cooldown() {
        let mut p = make_player(1, Team::Blue, Vec2::new(100.0, 100.0));
        let dt = 0.1;

        let input_shoot = InputPayload {
            move_axis: Vec2::ZERO,
            aim_pos: Vec2::new(200.0, 100.0),
            shoot: true,
        };

        // 1. First shot should succeed
        let proj = handle_shooting(&mut p, &input_shoot, dt, 101);
        assert!(proj.is_some(), "Should fire when cooldown is 0");
        assert!(p.weapon_cooldown > 0.0, "Cooldown should be set");

        // 2. Immediate second shot should fail
        let proj_fail = handle_shooting(&mut p, &input_shoot, dt, 102);
        assert!(proj_fail.is_none(), "Should not fire during cooldown");

        // 3. Wait for cooldown to expire
        p.weapon_cooldown = 0.0;
        let proj_again = handle_shooting(&mut p, &input_shoot, dt, 103);
        assert!(
            proj_again.is_some(),
            "Should fire again after cooldown reset"
        );
    }

    #[test]
    fn test_combat_damage_and_kills() {
        // Setup: Player 2 is at (200, 200).
        // We spawn a bullet exactly at (200, 200) owned by Player 1.
        let mut players = vec![
            make_player(1, Team::Blue, Vec2::new(0.0, 0.0)),
            make_player(2, Team::Red, Vec2::new(200.0, 200.0)),
        ];

        // Give Player 2 low health so they die in one hit
        players[1].health = 5.0;

        let mut projectiles = vec![Projectile {
            id: 99,
            owner_id: PlayerId(1),             // Owned by P1
            position: Vec2::new(200.0, 200.0), // Hits P2 immediately
            velocity: Vec2::ZERO,
            radius: 5.0,
        }];

        // Run Logic
        let kills = resolve_combat(&mut players, &mut projectiles);

        // Assertions
        assert_eq!(kills.len(), 1, "Should generate 1 kill event");
        assert_eq!(kills[0].victim_id, PlayerId(2));
        assert_eq!(kills[0].killer_id, PlayerId(1));

        assert_eq!(players.len(), 1, "Dead player should be removed from list");
        assert_eq!(players[0].id, PlayerId(1), "Survivor should be Player 1");

        assert!(
            projectiles.is_empty(),
            "Bullet should be destroyed on impact"
        );
    }

    #[test]
    fn test_no_friendly_fire_logic_check() {
        // NOTE: Currently your code allows friendly fire.
        // This test ensures the code behaves as currently written (FF is ON).

        let mut players = vec![
            make_player(1, Team::Blue, Vec2::new(0.0, 0.0)),
            make_player(2, Team::Blue, Vec2::new(50.0, 50.0)), // Teammate
        ];

        let mut projectiles = vec![Projectile {
            id: 88,
            owner_id: PlayerId(1),
            position: Vec2::new(50.0, 50.0), // Hits teammate
            velocity: Vec2::ZERO,
            radius: 5.0,
        }];

        resolve_combat(&mut players, &mut projectiles);

        // If you decide to add FF protection later, flip this assertion.
        assert!(
            players[1].health < 100.0,
            "Teammate took damage (Friendly Fire is enabled)"
        );
    }

    #[test]
    fn test_win_condition() {
        // Scenario 1: Both teams alive
        let p1 = vec![
            make_player(1, Team::Blue, Vec2::ZERO),
            make_player(2, Team::Red, Vec2::ZERO),
        ];
        assert_eq!(check_round_winner(&p1), None);

        // Scenario 2: Blue Eliminated
        let p2 = vec![make_player(2, Team::Red, Vec2::ZERO)];
        assert_eq!(check_round_winner(&p2), Some(Team::Red));

        // Scenario 3: Red Eliminated
        let p3 = vec![make_player(1, Team::Blue, Vec2::ZERO)];
        assert_eq!(check_round_winner(&p3), Some(Team::Blue));
    }

    #[test]
    #[allow(deprecated)]
    fn test_spawn_finding() {
        let map = make_map();
        let mut rng = StepRng::new(0, 1);
        let radius = 10.0;

        // 1. Find a valid spot
        let spawn = find_spawn_position(&map, radius, &mut rng);
        assert!(
            spawn.is_some(),
            "Should find a spawn on a largely empty map"
        );
        let pos = spawn.unwrap();

        // 2. Check it's not inside the wall (400..600)
        let in_wall_x = pos.x >= 390.0 && pos.x <= 610.0;
        let in_wall_y = pos.y >= 390.0 && pos.y <= 610.0;
        assert!(
            !(in_wall_x && in_wall_y),
            "Spawn position {:?} is inside the wall!",
            pos
        );
    }

    #[test]
    fn test_wall_collision_resolution() {
        let wall = RectWall {
            min: Vec2::new(100.0, 100.0),
            max: Vec2::new(200.0, 200.0),
        };
        let radius = 10.0;

        // Case 1: Player inside wall (deep overlap)
        let mut deep_pos = Vec2::new(150.0, 150.0);
        resolve_wall_collision(&mut deep_pos, radius, &wall);
        // Should be pushed to nearest edge (100 or 200)
        // 150 is equidistant, logic usually picks min_x or similar.
        // Let's just check it is OUTSIDE bounds now.
        let inside =
            deep_pos.x > 100.0 && deep_pos.x < 200.0 && deep_pos.y > 100.0 && deep_pos.y < 200.0;
        assert!(!inside, "Player should be pushed out of wall");

        // Case 2: Shallow collision (just touching)
        let mut shallow_pos = Vec2::new(95.0, 150.0); // radius 10 touches wall at x=100
        resolve_wall_collision(&mut shallow_pos, radius, &wall);
        assert!(
            shallow_pos.x <= 90.0 + 0.001,
            "Player should be pushed left to x=90 (radius distance)"
        );
    }
}
