use bevy::prelude::*;
use shared::Position;

pub fn sync_transform_system(mut query: Query<(&Position, &mut Transform)>) {
    for (position, mut transform) in &mut query {
        transform.translation.x = position.x;
        transform.translation.y = position.y;
    }
}
