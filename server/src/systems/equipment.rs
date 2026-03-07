use bevy::prelude::*;
use shared::protocol::EquipmentUpdate;
use shared::{
    item_modifier, item_slot, ArmorClass, CombatStats, EquipmentMap, EquipmentSlot, Inventory,
    ItemType,
};

use crate::{network, systems::loot};

#[derive(Message, Debug, Clone, Copy)]
pub struct EquipRequest {
    pub player_entity: Entity,
    pub item_type: ItemType,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct UnequipRequest {
    pub player_entity: Entity,
    pub slot: EquipmentSlot,
}

#[derive(Message, Debug, Clone)]
pub struct EquipmentChangedMessage {
    pub player_id: u64,
    pub equipment: EquipmentMap,
}

pub fn equip_system(
    mut requests: MessageReader<EquipRequest>,
    mut players: Query<
        (
            &network::NetworkEntity,
            &mut Inventory,
            &mut EquipmentMap,
            &mut CombatStats,
            &mut ArmorClass,
        ),
        With<network::PlayerCharacter>,
    >,
    mut changed: MessageWriter<EquipmentChangedMessage>,
    mut inventory_updates: MessageWriter<loot::InventoryUpdateMessage>,
) {
    for request in requests.read() {
        let Ok((network_entity, mut inventory, mut equipment, mut stats, mut armor)) =
            players.get_mut(request.player_entity)
        else {
            continue;
        };

        let Some(slot) = item_slot(request.item_type) else {
            continue;
        };

        let available = inventory
            .items
            .get(&request.item_type)
            .copied()
            .unwrap_or(0);
        if available == 0 {
            continue;
        }

        let equipped_item = match slot {
            EquipmentSlot::Weapon => &mut equipment.weapon,
            EquipmentSlot::Armor => &mut equipment.armor,
        };
        if equipped_item.is_some() {
            continue;
        }

        *equipped_item = Some(request.item_type);
        decrement_item(&mut inventory, request.item_type, 1);
        let total = inventory
            .items
            .get(&request.item_type)
            .copied()
            .unwrap_or(0);
        recalculate_stats_from_equipment(&equipment, &mut stats, &mut armor);

        changed.write(EquipmentChangedMessage {
            player_id: network_entity.id,
            equipment: equipment.clone(),
        });
        inventory_updates.write(loot::InventoryUpdateMessage {
            player_id: network_entity.id,
            item_type: request.item_type,
            amount: total,
        });
    }
}

pub fn unequip_system(
    mut requests: MessageReader<UnequipRequest>,
    mut players: Query<
        (
            &network::NetworkEntity,
            &mut Inventory,
            &mut EquipmentMap,
            &mut CombatStats,
            &mut ArmorClass,
        ),
        With<network::PlayerCharacter>,
    >,
    mut changed: MessageWriter<EquipmentChangedMessage>,
    mut inventory_updates: MessageWriter<loot::InventoryUpdateMessage>,
) {
    for request in requests.read() {
        let Ok((network_entity, mut inventory, mut equipment, mut stats, mut armor)) =
            players.get_mut(request.player_entity)
        else {
            continue;
        };

        let removed = match request.slot {
            EquipmentSlot::Weapon => equipment.weapon.take(),
            EquipmentSlot::Armor => equipment.armor.take(),
        };
        let Some(item_type) = removed else {
            continue;
        };

        *inventory.items.entry(item_type).or_insert(0) += 1;
        let total = inventory.items.get(&item_type).copied().unwrap_or(0);
        recalculate_stats_from_equipment(&equipment, &mut stats, &mut armor);
        changed.write(EquipmentChangedMessage {
            player_id: network_entity.id,
            equipment: equipment.clone(),
        });
        inventory_updates.write(loot::InventoryUpdateMessage {
            player_id: network_entity.id,
            item_type,
            amount: total,
        });
    }
}

fn decrement_item(inventory: &mut Inventory, item_type: ItemType, amount: u32) {
    if let Some(current) = inventory.items.get_mut(&item_type) {
        *current = current.saturating_sub(amount);
        if *current == 0 {
            inventory.items.remove(&item_type);
        }
    }
}

pub fn recalculate_stats_from_equipment(
    equipment: &EquipmentMap,
    stats: &mut CombatStats,
    armor: &mut ArmorClass,
) {
    let mut attack_bonus = 0;
    let mut armor_bonus = 0;

    if let Some(item) = equipment.weapon {
        let modifier = item_modifier(item);
        attack_bonus += modifier.attack_power_bonus;
        armor_bonus += modifier.armor_class_bonus;
    }
    if let Some(item) = equipment.armor {
        let modifier = item_modifier(item);
        attack_bonus += modifier.attack_power_bonus;
        armor_bonus += modifier.armor_class_bonus;
    }

    stats.attack_power = (25 + attack_bonus).clamp(1, 999);
    armor.value = armor_bonus.clamp(-200, 200);
}

pub fn to_equipment_update(message: &EquipmentChangedMessage) -> EquipmentUpdate {
    EquipmentUpdate {
        player_id: message.player_id,
        equipment: message.equipment.clone(),
    }
}
