use bevy::prelude::*;
use shared::protocol::MoveIntent;

use crate::network;

pub fn capture_movement_intent(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    network: Option<Res<network::ClientNetwork>>,
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

    let Some(network) = network else {
        return;
    };

    network::send_move_intent(
        &network,
        MoveIntent {
            target_x: world_position.x,
            target_y: world_position.y,
        },
    );
}
