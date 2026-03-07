use serde::{Deserialize, Serialize};

use crate::ItemType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    MoveIntent(MoveIntent),
    AttackIntent(AttackIntent),
    LootIntent(LootIntent),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MoveIntent {
    pub target_x: f32,
    pub target_y: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AttackIntent {
    pub target_id: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LootIntent {
    pub item_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    AssignedPlayer { player_id: u64 },
    EntityState(EntityState),
    DamageEvent(DamageEvent),
    DeathEvent(DeathEvent),
    ItemSpawnEvent(ItemSpawnEvent),
    ItemDespawnEvent(ItemDespawnEvent),
    InventoryUpdate(InventoryUpdate),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkEntityKind {
    Player,
    Enemy,
    LootGold,
    LootHealthPotion,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EntityState {
    pub entity_id: u64,
    pub kind: NetworkEntityKind,
    pub x: f32,
    pub y: f32,
    pub health_current: i32,
    pub health_max: i32,
    pub alive: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DamageEvent {
    pub target_id: u64,
    pub amount: i32,
    pub remaining_hp: i32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DeathEvent {
    pub target_id: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ItemSpawnEvent {
    pub item_id: u64,
    pub item_type: ItemType,
    pub amount: u32,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ItemDespawnEvent {
    pub item_id: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct InventoryUpdate {
    pub player_id: u64,
    pub item_type: ItemType,
    pub amount: u32,
}

pub fn encode_client_message(message: &ClientMessage) -> Result<Vec<u8>, bincode::Error> {
    bincode::serialize(message)
}

pub fn decode_client_message(payload: &[u8]) -> Result<ClientMessage, bincode::Error> {
    bincode::deserialize(payload)
}

pub fn encode_server_message(message: &ServerMessage) -> Result<Vec<u8>, bincode::Error> {
    bincode::serialize(message)
}

pub fn decode_server_message(payload: &[u8]) -> Result<ServerMessage, bincode::Error> {
    bincode::deserialize(payload)
}
