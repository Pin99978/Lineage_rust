use bevy::prelude::*;
use shared::{MoveSpeed, Position, TargetPosition};

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
        let next_position = Vec2::new(position.x, position.y) + direction * step;

        position.x = next_position.x;
        position.y = next_position.y;
    }
}

pub fn sync_transform_system(mut query: Query<(&Position, &mut Transform)>) {
    for (position, mut transform) in &mut query {
        transform.translation.x = position.x;
        transform.translation.y = position.y;
    }
}
