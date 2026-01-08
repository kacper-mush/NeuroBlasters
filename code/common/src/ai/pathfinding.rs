use crate::net::protocol::MapDefinition;
use glam::Vec2;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

const GRID_SIZE: f32 = 40.0; // Discretize map into 40x40 chunks

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
struct GridPos {
    x: i32,
    y: i32,
}

impl GridPos {
    fn from_vec2(v: Vec2) -> Self {
        Self {
            x: (v.x / GRID_SIZE).floor() as i32,
            y: (v.y / GRID_SIZE).floor() as i32,
        }
    }

    fn to_vec2(self) -> Vec2 {
        Vec2::new(
            self.x as f32 * GRID_SIZE + GRID_SIZE / 2.0,
            self.y as f32 * GRID_SIZE + GRID_SIZE / 2.0,
        )
    }

    fn distance(self, other: GridPos) -> f32 {
        let dx = (self.x - other.x).abs();
        let dy = (self.y - other.y).abs();
        ((dx * dx + dy * dy) as f32).sqrt()
    }
}

#[derive(Clone, Copy, PartialEq)]
struct AStarNode {
    cost: f32,
    pos: GridPos,
}

impl Eq for AStarNode {}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for Min-Heap
        other
            .cost
            .partial_cmp(&self.cost)
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn is_cell_blocked(pos: GridPos, map: &MapDefinition) -> bool {
    let min = Vec2::new(pos.x as f32 * GRID_SIZE, pos.y as f32 * GRID_SIZE);
    let max = min + Vec2::splat(GRID_SIZE);

    // Check bounds
    if min.x < 0.0 || min.y < 0.0 || max.x > map.width || max.y > map.height {
        return true;
    }

    // Check walls (AABB intersection)
    for wall in &map.walls {
        if min.x < wall.max.x && max.x > wall.min.x && min.y < wall.max.y && max.y > wall.min.y {
            return true;
        }
    }
    false
}

pub fn find_path_a_star(start: Vec2, end: Vec2, map: &MapDefinition) -> Vec<Vec2> {
    let start_grid = GridPos::from_vec2(start);
    let end_grid = GridPos::from_vec2(end);

    let mut open_set = BinaryHeap::new();
    open_set.push(AStarNode {
        cost: 0.0,
        pos: start_grid,
    });

    let mut came_from: HashMap<GridPos, GridPos> = HashMap::new();
    let mut cost_so_far: HashMap<GridPos, f32> = HashMap::new();
    cost_so_far.insert(start_grid, 0.0);

    let mut found = false;

    // Safety limit to prevent freezing
    let mut iterations = 0;
    while let Some(current) = open_set.pop() {
        iterations += 1;
        if iterations > 500 {
            break;
        } // Path too complex or unreachable

        if current.pos == end_grid {
            found = true;
            break;
        }

        // Neighbors
        let neighbors = [
            GridPos {
                x: current.pos.x + 1,
                y: current.pos.y,
            },
            GridPos {
                x: current.pos.x - 1,
                y: current.pos.y,
            },
            GridPos {
                x: current.pos.x,
                y: current.pos.y + 1,
            },
            GridPos {
                x: current.pos.x,
                y: current.pos.y - 1,
            },
        ];

        for next in neighbors {
            if is_cell_blocked(next, map) {
                continue;
            }

            let new_cost = cost_so_far[&current.pos] + 1.0; // Uniform grid cost

            if !cost_so_far.contains_key(&next) || new_cost < cost_so_far[&next] {
                cost_so_far.insert(next, new_cost);
                let priority = new_cost + next.distance(end_grid); // Heuristic
                open_set.push(AStarNode {
                    cost: priority,
                    pos: next,
                });
                came_from.insert(next, current.pos);
            }
        }
    }

    if found {
        // Reconstruct path
        let mut path = Vec::new();
        let mut current = end_grid;
        while current != start_grid {
            path.push(current.to_vec2());
            current = came_from[&current];
        }
        path.reverse();
        return path;
    }

    // If no path found, return direct line as fallback
    vec![end]
}
