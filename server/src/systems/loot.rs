use bevy::prelude::*;
use shared::{GroundItem, Inventory, MapId, Position};

use crate::network;

#[derive(Message, Debug, Clone, Copy)]
pub struct LootRequest {
    pub looter_entity: Entity,
    pub item_id: u64,
}

#[derive(Message, Debug, Clone)]
pub struct ItemDespawnedMessage {
    pub item_id: u64,
    pub map_id: String,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct InventoryUpdateMessage {
    pub player_id: u64,
    pub item_type: shared::ItemType,
    pub amount: u32,
}

pub fn loot_system(
    mut commands: Commands,
    mut requests: MessageReader<LootRequest>,
    mut players: Query<
        (&Position, &MapId, &mut Inventory, &network::NetworkEntity),
        With<network::PlayerCharacter>,
    >,
    items: Query<(
        Entity,
        &MapId,
        &network::NetworkEntity,
        &GroundItem,
        &Position,
    )>,
    mut despawn_messages: MessageWriter<ItemDespawnedMessage>,
    mut inventory_messages: MessageWriter<InventoryUpdateMessage>,
) {
    for request in requests.read() {
        let Ok((player_position, player_map, mut inventory, player_network)) =
            players.get_mut(request.looter_entity)
        else {
            continue;
        };

        let Some((item_entity, item_map, item_network, ground_item, item_position)) = items
            .iter()
            .find(|(_, _, network_entity, _, _)| network_entity.id == request.item_id)
        else {
            continue;
        };
        if item_map.0 != player_map.0 {
            continue;
        }

        let distance = Vec2::new(
            item_position.x - player_position.x,
            item_position.y - player_position.y,
        )
        .length();
        if distance > 90.0 {
            continue;
        }

        let total = inventory
            .items
            .entry(ground_item.item_type)
            .and_modify(|value| *value += ground_item.amount)
            .or_insert(ground_item.amount);

        inventory_messages.write(InventoryUpdateMessage {
            player_id: player_network.id,
            item_type: ground_item.item_type,
            amount: *total,
        });
        despawn_messages.write(ItemDespawnedMessage {
            item_id: item_network.id,
            map_id: item_map.0.clone(),
        });
        commands.entity(item_entity).despawn();
    }
}
