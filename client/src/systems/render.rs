use bevy::prelude::*;

#[derive(Component)]
pub struct YSortable;

pub fn y_sorting_system(mut query: Query<&mut Transform, With<YSortable>>) {
    for mut transform in &mut query {
        transform.translation.z = -transform.translation.y * 0.0001;
    }
}
