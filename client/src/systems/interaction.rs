use bevy::prelude::*;
use shared::protocol::{AttackIntent, InteractIntent, LootIntent, MoveIntent};
use shared::Health;

use crate::{network, systems::animation};

#[allow(clippy::too_many_arguments)]
pub fn capture_click_intent(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    network: Option<Res<network::ClientNetwork>>,
    chat_state: Option<Res<crate::systems::ui::chat::ChatUiState>>,
    mut attack_animation: MessageWriter<animation::PlayAttackAnimation>,
    lootables: Query<(&Transform, &network::NetworkEntityVisual), With<network::Lootable>>,
    npcs: Query<(&Transform, &network::NetworkEntityVisual), With<network::NpcInteractable>>,
    attackables: Query<
        (&Transform, &network::NetworkEntityVisual, &Health),
        With<network::Attackable>,
    >,
) {
    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }
    if chat_state
        .as_ref()
        .map(|state| state.focused)
        .unwrap_or(false)
    {
        return;
    }
    let Some(network) = network else {
        return;
    };
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

    if let Some(item_id) = lootables
        .iter()
        .find(|(transform, _)| transform.translation.truncate().distance(world_position) <= 18.0)
        .map(|(_, visual)| visual.id)
    {
        network::send_loot_intent(&network, LootIntent { item_id });
        return;
    }

    if let Some(target_id) = npcs
        .iter()
        .find(|(transform, _)| transform.translation.truncate().distance(world_position) <= 26.0)
        .map(|(_, visual)| visual.id)
    {
        network::send_interact_intent(&network, InteractIntent { target_id });
        return;
    }

    if let Some(target_id) = attackables
        .iter()
        .find(|(transform, _, health)| {
            health.current > 0 && transform.translation.truncate().distance(world_position) <= 24.0
        })
        .map(|(_, visual, _)| visual.id)
    {
        network::send_attack_intent(&network, AttackIntent { target_id });
        attack_animation.write(animation::PlayAttackAnimation {
            target_id: Some(target_id),
            local_player: true,
        });
        return;
    }

    network::send_move_intent(
        &network,
        MoveIntent {
            target_x: world_position.x,
            target_y: world_position.y,
        },
    );
}
