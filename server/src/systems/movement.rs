use bevy::prelude::*;
use shared::{MoveSpeed, Position, TargetPosition};

const WORLD_HALF_EXTENT_X: f32 = 950.0;
const WORLD_HALF_EXTENT_Y: f32 = 950.0;
const OBSTACLE_MIN_X: f32 = -140.0;
const OBSTACLE_MAX_X: f32 = 140.0;
const OBSTACLE_MIN_Y: f32 = -100.0;
const OBSTACLE_MAX_Y: f32 = 100.0;

pub fn movement_system(
    time: Res<Time>,
    mut query: Query<(&mut Position, &TargetPosition, &MoveSpeed)>,
) {
    for (mut position, target, speed) in &mut query {
        let to_target = Vec2::new(target.x - position.x, target.y - position.y);
        let distance = to_target.length();

        if distance <= f32::EPSILON {
            continue;
        }

        let max_step = speed.value * time.delta_secs();
        let step = distance.min(max_step);
        let direction = to_target / distance;
        let current_position = Vec2::new(position.x, position.y);
        let desired_position = current_position + direction * step;
        let next_position = resolve_walkable_position(current_position, desired_position);

        position.x = next_position.x;
        position.y = next_position.y;
    }
}

fn resolve_walkable_position(current: Vec2, desired: Vec2) -> Vec2 {
    let clamped = clamp_to_world_bounds(desired);
    if !is_inside_obstacle(clamped) {
        return clamped;
    }

    let slide_x = clamp_to_world_bounds(Vec2::new(clamped.x, current.y));
    let slide_y = clamp_to_world_bounds(Vec2::new(current.x, clamped.y));

    let x_walkable = !is_inside_obstacle(slide_x);
    let y_walkable = !is_inside_obstacle(slide_y);

    match (x_walkable, y_walkable) {
        (true, true) => {
            let x_distance = (clamped - slide_x).length_squared();
            let y_distance = (clamped - slide_y).length_squared();
            if x_distance <= y_distance {
                slide_x
            } else {
                slide_y
            }
        }
        (true, false) => slide_x,
        (false, true) => slide_y,
        (false, false) => current,
    }
}

fn clamp_to_world_bounds(position: Vec2) -> Vec2 {
    Vec2::new(
        position.x.clamp(-WORLD_HALF_EXTENT_X, WORLD_HALF_EXTENT_X),
        position.y.clamp(-WORLD_HALF_EXTENT_Y, WORLD_HALF_EXTENT_Y),
    )
}

fn is_inside_obstacle(position: Vec2) -> bool {
    position.x >= OBSTACLE_MIN_X
        && position.x <= OBSTACLE_MAX_X
        && position.y >= OBSTACLE_MIN_Y
        && position.y <= OBSTACLE_MAX_Y
}
