use bevy::prelude::*;
use shared::{EquipmentSlot, Health, ItemType, SpellType};

use crate::{network, systems::animation, Player};

#[allow(clippy::too_many_arguments)]
pub fn capture_movement_intent(
    keyboard: Res<ButtonInput<KeyCode>>,
    network: Option<Res<network::ClientNetwork>>,
    chat_state: Option<Res<crate::systems::ui::chat::ChatUiState>>,
    windows_state: Option<Res<crate::systems::ui::inventory::UiWindowsState>>,
    player_query: Query<&Transform, With<Player>>,
    mut attack_animation: MessageWriter<animation::PlayAttackAnimation>,
    attackables: Query<
        (&Transform, &network::NetworkEntityVisual, &Health),
        With<network::Attackable>,
    >,
) {
    let Some(network) = network else {
        return;
    };
    if chat_state
        .as_ref()
        .map(|state| state.focused)
        .unwrap_or(false)
    {
        return;
    }
    if windows_state
        .as_ref()
        .map(|state| state.blocks_world_input())
        .unwrap_or(false)
    {
        return;
    }

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
            attack_animation.write(animation::PlayAttackAnimation {
                target_id: target,
                local_player: true,
            });
        }
    }

    if keyboard.just_pressed(KeyCode::Digit2) {
        network::cast_spell_by_hotkey(&network, SpellType::Heal, None);
        attack_animation.write(animation::PlayAttackAnimation {
            target_id: None,
            local_player: true,
        });
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
}
