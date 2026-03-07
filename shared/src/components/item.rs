use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize, Default)]
pub enum ItemType {
    #[default]
    Gold,
    HealthPotion,
    BronzeSword,
    LeatherArmor,
}

#[derive(Component, Debug, Clone, Copy, Reflect, Serialize, Deserialize)]
#[reflect(Component, Default)]
pub struct GroundItem {
    pub item_type: ItemType,
    pub amount: u32,
}

impl Default for GroundItem {
    fn default() -> Self {
        Self {
            item_type: ItemType::Gold,
            amount: 1,
        }
    }
}

#[derive(Component, Default, Debug, Clone, Reflect)]
#[reflect(Component, Default)]
pub struct Inventory {
    pub items: HashMap<ItemType, u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
pub enum EquipmentSlot {
    Weapon,
    Armor,
}

#[derive(Debug, Clone, Copy, Reflect, Serialize, Deserialize, Default)]
pub struct StatModifier {
    pub attack_power_bonus: i32,
    pub armor_class_bonus: i32,
}

#[derive(Component, Default, Debug, Clone, Reflect, Serialize, Deserialize)]
#[reflect(Component, Default)]
pub struct EquipmentMap {
    pub weapon: Option<ItemType>,
    pub armor: Option<ItemType>,
}

pub fn item_slot(item_type: ItemType) -> Option<EquipmentSlot> {
    match item_type {
        ItemType::BronzeSword => Some(EquipmentSlot::Weapon),
        ItemType::LeatherArmor => Some(EquipmentSlot::Armor),
        _ => None,
    }
}

pub fn item_modifier(item_type: ItemType) -> StatModifier {
    match item_type {
        ItemType::BronzeSword => StatModifier {
            attack_power_bonus: 8,
            armor_class_bonus: 0,
        },
        ItemType::LeatherArmor => StatModifier {
            attack_power_bonus: 0,
            armor_class_bonus: 6,
        },
        _ => StatModifier::default(),
    }
}

#[derive(Debug, Clone, Copy, Reflect, Serialize, Deserialize)]
pub struct LootDropEntry {
    pub item_type: ItemType,
    pub amount: u32,
    pub chance_permille: u16,
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Default)]
pub struct LootTable {
    pub entries: Vec<LootDropEntry>,
}

impl Default for LootTable {
    fn default() -> Self {
        Self {
            entries: vec![
                LootDropEntry {
                    item_type: ItemType::Gold,
                    amount: 12,
                    chance_permille: 1000,
                },
                LootDropEntry {
                    item_type: ItemType::HealthPotion,
                    amount: 1,
                    chance_permille: 320,
                },
                LootDropEntry {
                    item_type: ItemType::BronzeSword,
                    amount: 1,
                    chance_permille: 140,
                },
                LootDropEntry {
                    item_type: ItemType::LeatherArmor,
                    amount: 1,
                    chance_permille: 170,
                },
            ],
        }
    }
}
