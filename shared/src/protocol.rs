use serde::{Deserialize, Serialize};

use crate::{EquipmentMap, EquipmentSlot, ItemType, SpellType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    LoginRequest(LoginRequest),
    MoveIntent(MoveIntent),
    AttackIntent(AttackIntent),
    LootIntent(LootIntent),
    CastSpellIntent(CastSpellIntent),
    EquipIntent(EquipIntent),
    UnequipIntent(UnequipIntent),
    InteractIntent(InteractIntent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CastSpellIntent {
    pub spell: SpellType,
    pub target_id: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EquipIntent {
    pub item_type: ItemType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct UnequipIntent {
    pub slot: EquipmentSlot,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct InteractIntent {
    pub target_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    LoginResponse(LoginResponse),
    AssignedPlayer { player_id: u64 },
    EntityState(EntityState),
    DamageEvent(DamageEvent),
    DeathEvent(DeathEvent),
    ItemSpawnEvent(ItemSpawnEvent),
    ItemDespawnEvent(ItemDespawnEvent),
    InventoryUpdate(InventoryUpdate),
    ManaUpdate(ManaUpdate),
    EquipmentUpdate(EquipmentUpdate),
    HealEvent(HealEvent),
    DialogEvent(DialogEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkEntityKind {
    Player,
    Enemy,
    NpcMerchant,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ManaUpdate {
    pub player_id: u64,
    pub current: i32,
    pub max: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentUpdate {
    pub player_id: u64,
    pub equipment: EquipmentMap,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HealEvent {
    pub target_id: u64,
    pub amount: i32,
    pub resulting_hp: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogEvent {
    pub player_id: u64,
    pub text: String,
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
