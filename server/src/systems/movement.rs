use bevy::prelude::*;
use pathfinding::directed::astar::astar;
use shared::{MoveSpeed, PathQueue, Position, TargetPosition};
use std::collections::VecDeque;

use crate::map_data::CollisionGrid;

#[derive(Message, Debug, Clone, Copy)]
pub struct MoveRequest {
    pub mover_entity: Entity,
    pub target_x: f32,
    pub target_y: f32,
}

pub fn process_move_requests(
    grid: Option<Res<CollisionGrid>>,
    mut requests: MessageReader<MoveRequest>,
    mut movers: Query<(&Position, &mut TargetPosition, &mut PathQueue)>,
) {
    let Some(grid) = grid else {
        return;
    };

    for request in requests.read() {
        let Ok((position, mut target, mut queue)) = movers.get_mut(request.mover_entity) else {
            continue;
        };

        let start = Vec2::new(position.x, position.y);
        let requested_target = Vec2::new(request.target_x, request.target_y);

        if let Some(path) = compute_path_world(&grid, start, requested_target) {
            queue.waypoints = path;
            if let Some(first) = queue.waypoints.front() {
                target.x = first.x;
                target.y = first.y;
            } else {
                target.x = position.x;
                target.y = position.y;
            }
        } else {
            queue.waypoints.clear();
            target.x = position.x;
            target.y = position.y;
        }
    }
}

pub fn movement_system(
    time: Res<Time>,
    mut query: Query<(
        &mut Position,
        &mut TargetPosition,
        &MoveSpeed,
        &mut PathQueue,
    )>,
) {
    for (mut position, mut target, speed, mut queue) in &mut query {
        if queue.waypoints.is_empty() {
            continue;
        }

        let Some(next_waypoint) = queue.waypoints.front().copied() else {
            continue;
        };
        target.x = next_waypoint.x;
        target.y = next_waypoint.y;

        let to_target = Vec2::new(target.x - position.x, target.y - position.y);
        let distance = to_target.length();
        if distance <= 0.75 {
            queue.waypoints.pop_front();
            if let Some(next) = queue.waypoints.front() {
                target.x = next.x;
                target.y = next.y;
            } else {
                target.x = position.x;
                target.y = position.y;
            }
            continue;
        }

        let max_step = speed.value * time.delta_secs();
        let step = distance.min(max_step);
        let direction = to_target / distance;

        position.x += direction.x * step;
        position.y += direction.y * step;
    }
}

pub fn compute_path_world(
    grid: &CollisionGrid,
    from: Vec2,
    to: Vec2,
) -> Option<VecDeque<Position>> {
    let start_cell = grid.world_to_cell(from)?;
    let end_cell = grid.world_to_cell(to)?;
    let walkable_start = if grid.is_cell_blocked(start_cell) {
        grid.nearest_walkable(start_cell)?
    } else {
        start_cell
    };
    let walkable_end = grid.nearest_walkable(end_cell)?;

    let path_cells = astar(
        &walkable_start,
        |cell| successors(grid, *cell),
        |cell| heuristic(*cell, walkable_end),
        |cell| *cell == walkable_end,
    )
    .map(|(path, _)| path)?;

    let mut waypoints = VecDeque::new();
    for cell in path_cells.into_iter().skip(1) {
        let waypoint = grid.cell_to_world_center(cell);
        waypoints.push_back(Position {
            x: waypoint.x,
            y: waypoint.y,
        });
    }
    Some(waypoints)
}

fn successors(grid: &CollisionGrid, node: IVec2) -> Vec<(IVec2, u32)> {
    const DIRS: &[(i32, i32, u32)] = &[
        (-1, 0, 10),
        (1, 0, 10),
        (0, -1, 10),
        (0, 1, 10),
        (-1, -1, 14),
        (-1, 1, 14),
        (1, -1, 14),
        (1, 1, 14),
    ];

    let mut out = Vec::with_capacity(8);
    for (dx, dy, cost) in DIRS {
        let next = IVec2::new(node.x + dx, node.y + dy);
        if !grid.in_bounds(next) || grid.is_cell_blocked(next) {
            continue;
        }
        // Prevent diagonal corner cutting through two blocked cardinal cells.
        if *dx != 0 && *dy != 0 {
            let side_a = IVec2::new(node.x + dx, node.y);
            let side_b = IVec2::new(node.x, node.y + dy);
            if grid.is_cell_blocked(side_a) && grid.is_cell_blocked(side_b) {
                continue;
            }
        }
        out.push((next, *cost));
    }
    out
}

fn heuristic(from: IVec2, to: IVec2) -> u32 {
    let dx = (from.x - to.x).unsigned_abs();
    let dy = (from.y - to.y).unsigned_abs();
    let diagonal = dx.min(dy);
    let straight = dx.max(dy) - diagonal;
    diagonal * 14 + straight * 10
}
