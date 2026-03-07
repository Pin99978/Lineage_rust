use bevy::prelude::*;
use shared::protocol::{AttackIntent, MoveIntent};
use shared::Health;

use crate::network;

pub fn capture_movement_intent(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    network: Option<Res<network::ClientNetwork>>,
    attackables: Query<
        (&Transform, &network::NetworkEntityVisual, &Health),
        With<network::Attackable>,
    >,
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

    let clicked_target = attackables
        .iter()
        .find(|(transform, _, health)| {
            health.current > 0 && transform.translation.truncate().distance(world_position) <= 24.0
        })
        .map(|(_, visual, _)| visual.id);

    if let Some(target_id) = clicked_target {
        network::send_attack_intent(&network, AttackIntent { target_id });
    } else {
        network::send_move_intent(
            &network,
            MoveIntent {
                target_x: world_position.x,
                target_y: world_position.y,
            },
        );
    }
}
