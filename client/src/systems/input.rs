use bevy::prelude::*;
use shared::protocol::{AttackIntent, LootIntent, MoveIntent};
use shared::{EquipmentSlot, Health, ItemType, SpellType};

use crate::{network, Player};

#[allow(clippy::too_many_arguments)]
pub fn capture_movement_intent(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    network: Option<Res<network::ClientNetwork>>,
    player_query: Query<&Transform, With<Player>>,
    lootables: Query<(&Transform, &network::NetworkEntityVisual), With<network::Lootable>>,
    attackables: Query<
        (&Transform, &network::NetworkEntityVisual, &Health),
        With<network::Attackable>,
    >,
) {
    let Some(network) = network else {
        return;
    };

    if keyboard.just_pressed(KeyCode::Digit1) {
        if let Ok(player_transform) = player_query.single() {
            let player_position = player_transform.translation.truncate();
            let target = attackables
                .iter()
                .filter(|(_, _, health)| health.current > 0)
                .min_by(|(left_t, _, _), (right_t, _, _)| {
                    let left = left_t.translation.truncate().distance(player_position);
                    let right = right_t.translation.truncate().distance(player_position);
                    left.total_cmp(&right)
                })
                .map(|(_, visual, _)| visual.id);

            network::cast_spell_by_hotkey(&network, SpellType::Fireball, target);
        }
    }

    if keyboard.just_pressed(KeyCode::Digit2) {
        network::cast_spell_by_hotkey(&network, SpellType::Heal, None);
    }

    if keyboard.just_pressed(KeyCode::KeyE) {
        network::equip_item_by_hotkey(&network, ItemType::BronzeSword);
    }
    if keyboard.just_pressed(KeyCode::KeyR) {
        network::equip_item_by_hotkey(&network, ItemType::LeatherArmor);
    }
    if keyboard.just_pressed(KeyCode::KeyQ) {
        network::unequip_slot_by_hotkey(&network, EquipmentSlot::Weapon);
    }
    if keyboard.just_pressed(KeyCode::KeyW) {
        network::unequip_slot_by_hotkey(&network, EquipmentSlot::Armor);
    }

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

    let clicked_loot = lootables
        .iter()
        .find(|(transform, _)| transform.translation.truncate().distance(world_position) <= 18.0)
        .map(|(_, visual)| visual.id);
    if let Some(item_id) = clicked_loot {
        network::send_loot_intent(&network, LootIntent { item_id });
        return;
    }

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
