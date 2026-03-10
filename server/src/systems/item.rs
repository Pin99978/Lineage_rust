use bevy::prelude::*;
use shared::{class_def, scroll_spell, CharacterClass, Health, Inventory, ItemType, KnownSpells};

use crate::{
    network,
    systems::{loot, spell},
};

#[derive(Message, Debug, Clone, Copy)]
pub struct UseItemRequest {
    pub player_entity: Entity,
    pub item_type: ItemType,
}

const HEALTH_POTION_HEAL: i32 = 20;

pub fn use_item_system(
    mut requests: MessageReader<UseItemRequest>,
    mut players: Query<
        (
            &network::NetworkEntity,
            &mut Inventory,
            &mut Health,
            &CharacterClass,
            &mut KnownSpells,
        ),
        With<network::PlayerCharacter>,
    >,
    mut inventory_updates: MessageWriter<loot::InventoryUpdateMessage>,
    mut heal_events: MessageWriter<spell::HealEventMessage>,
    mut spell_learned_events: MessageWriter<spell::SpellLearnedMessage>,
) {
    for request in requests.read() {
        let Ok((player_network, mut inventory, mut health, player_class, mut known_spells)) =
            players.get_mut(request.player_entity)
        else {
            continue;
        };

        if request.item_type == ItemType::HealthPotion {
            let available = inventory
                .items
                .get(&ItemType::HealthPotion)
                .copied()
                .unwrap_or(0);
            if available == 0 {
                continue;
            }

            let before = health.current;
            health.current = (health.current + HEALTH_POTION_HEAL).clamp(0, health.max);
            if health.current == before {
                continue;
            }

            let remaining = available.saturating_sub(1);
            if remaining == 0 {
                inventory.items.remove(&ItemType::HealthPotion);
            } else {
                inventory.items.insert(ItemType::HealthPotion, remaining);
            }

            inventory_updates.write(loot::InventoryUpdateMessage {
                player_id: player_network.id,
                item_type: ItemType::HealthPotion,
                amount: remaining,
            });
            heal_events.write(spell::HealEventMessage {
                target_id: player_network.id,
                amount: health.current - before,
                resulting_hp: health.current,
            });
            continue;
        }

        let Some(spell_to_learn) = scroll_spell(request.item_type) else {
            continue;
        };
        if !class_def(*player_class).can_cast.contains(&spell_to_learn) {
            continue;
        }
        if known_spells.knows(spell_to_learn) {
            continue;
        }

        let available = inventory.items.get(&request.item_type).copied().unwrap_or(0);
        if available == 0 {
            continue;
        }

        let remaining = available.saturating_sub(1);
        if remaining == 0 {
            inventory.items.remove(&request.item_type);
        } else {
            inventory.items.insert(request.item_type, remaining);
        }
        if !known_spells.learn(spell_to_learn) {
            continue;
        }

        inventory_updates.write(loot::InventoryUpdateMessage {
            player_id: player_network.id,
            item_type: request.item_type,
            amount: remaining,
        });
        spell_learned_events.write(spell::SpellLearnedMessage {
            player_id: player_network.id,
            spell: spell_to_learn,
        });
    }
}
