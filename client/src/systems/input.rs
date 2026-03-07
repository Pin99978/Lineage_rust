use bevy::prelude::*;
use shared::TargetPosition;

use crate::Player;

pub fn capture_movement_intent(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut player_query: Query<&mut TargetPosition, With<Player>>,
) {
    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = window_query.single() else {
        return;
    };
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };
    let Ok(world_position) = camera.viewport_to_world_2d(camera_transform, cursor_position) else {
        return;
    };

    for mut target_position in &mut player_query {
        target_position.x = world_position.x;
        target_position.y = world_position.y;
    }
}
