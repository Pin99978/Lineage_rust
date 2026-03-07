use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize, Default)]
pub enum ItemType {
    #[default]
    Gold,
    HealthPotion,
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
            ],
        }
    }
}
