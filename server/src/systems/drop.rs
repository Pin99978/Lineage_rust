use bevy::prelude::*;
use shared::protocol::NetworkEntityKind;
use shared::{GroundItem, ItemType, LootTable, MapId, Position};

use crate::{network, systems};

#[derive(Message, Debug, Clone)]
pub struct ItemSpawnedMessage {
    pub item_id: u64,
    pub map_id: String,
    pub item_type: ItemType,
    pub amount: u32,
    pub x: f32,
    pub y: f32,
}

pub fn item_drop_system(
    mut commands: Commands,
    mut death_events: MessageReader<systems::combat::CombatDeathEvent>,
    network: Option<ResMut<network::ServerNetwork>>,
    dead_targets: Query<(
        &network::NetworkEntity,
        &MapId,
        &Position,
        Option<&LootTable>,
    )>,
    mut spawned_messages: MessageWriter<ItemSpawnedMessage>,
) {
    let Some(mut network) = network else {
        return;
    };

    for death in death_events.read() {
        let Some((_, map_id, position, loot_table)) = dead_targets
            .iter()
            .find(|(entity, _, _, _)| entity.id == death.target_id)
        else {
            continue;
        };
        let Some(loot_table) = loot_table else {
            continue;
        };

        let mut slot_index = 0_u32;
        for entry in &loot_table.entries {
            let roll = deterministic_roll(death.target_id, slot_index);
            slot_index += 1;
            if roll >= entry.chance_permille as u32 {
                continue;
            }

            let item_id = network.allocate_entity_id();
            let offset = slot_index as f32 * 12.0;
            let item_x = position.x + offset;
            let item_y = position.y;
            let kind = match entry.item_type {
                ItemType::Gold => NetworkEntityKind::LootGold,
                ItemType::HealthPotion => NetworkEntityKind::LootHealthPotion,
                ItemType::BronzeSword => NetworkEntityKind::LootGold,
                ItemType::LeatherArmor => NetworkEntityKind::LootHealthPotion,
                ItemType::ScrollLightning => NetworkEntityKind::LootHealthPotion,
                ItemType::ScrollPoisonArrow => NetworkEntityKind::LootHealthPotion,
                ItemType::ScrollBless => NetworkEntityKind::LootHealthPotion,
            };

            commands.spawn((
                network::NetworkEntity { id: item_id, kind },
                MapId(map_id.0.clone()),
                GroundItem {
                    item_type: entry.item_type,
                    amount: entry.amount,
                },
                Position {
                    x: item_x,
                    y: item_y,
                },
            ));

            spawned_messages.write(ItemSpawnedMessage {
                item_id,
                map_id: map_id.0.clone(),
                item_type: entry.item_type,
                amount: entry.amount,
                x: item_x,
                y: item_y,
            });
        }
    }
}

fn deterministic_roll(seed: u64, slot_index: u32) -> u32 {
    let mixed = seed ^ ((slot_index as u64 + 1) * 0x9E37_79B1_u64);
    (mixed % 1000) as u32
}
