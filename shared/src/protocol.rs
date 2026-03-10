use serde::{Deserialize, Serialize};

use crate::{
    CharacterClass, EquipmentMap, EquipmentSlot, ItemType, QuestId, QuestStatus, SpellType,
    StatusEffect,
};

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
    InteractNpcIntent(InteractNpcIntent),
    ChatIntent(ChatIntent),
    UseItemIntent(UseItemIntent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub class: CharacterClass,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct InteractNpcIntent {
    pub target_id: u64,
    pub choice_index: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatChannel {
    Say,
    Shout,
    Whisper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatIntent {
    pub channel: ChatChannel,
    pub target: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct UseItemIntent {
    pub item_type: ItemType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    LoginResponse(LoginResponse),
    AssignedPlayer { player_id: u64 },
    EntityState(EntityState),
    MapChangeEvent(MapChangeEvent),
    DamageEvent(DamageEvent),
    DeathEvent(DeathEvent),
    ItemSpawnEvent(ItemSpawnEvent),
    ItemDespawnEvent(ItemDespawnEvent),
    InventoryUpdate(InventoryUpdate),
    ManaUpdate(ManaUpdate),
    ExpUpdateEvent(ExpUpdateEvent),
    LevelUpEvent(LevelUpEvent),
    EquipmentUpdate(EquipmentUpdate),
    HealEvent(HealEvent),
    DialogEvent(DialogEvent),
    DialogueResponse(DialogueResponse),
    ChatEvent(ChatEvent),
    QuestUpdateEvent(QuestUpdateEvent),
    StatusEffectUpdate(StatusEffectUpdate),
    SystemNotice(SystemNotice),
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
    Portal,
    LootGold,
    LootHealthPotion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityState {
    pub entity_id: u64,
    pub kind: NetworkEntityKind,
    pub class: CharacterClass,
    pub map_id: String,
    pub x: f32,
    pub y: f32,
    pub health_current: i32,
    pub health_max: i32,
    pub alive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapChangeEvent {
    pub map_id: String,
    pub x: f32,
    pub y: f32,
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
    pub exp_lost: Option<u32>,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ExpUpdateEvent {
    pub player_id: u64,
    pub level: u32,
    pub exp_current: u32,
    pub exp_next: u32,
    pub str_stat: u32,
    pub dex: u32,
    pub int_stat: u32,
    pub con: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LevelUpEvent {
    pub player_id: u64,
    pub new_level: u32,
    pub health_max: i32,
    pub mana_max: i32,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueResponse {
    pub player_id: u64,
    pub npc_id: u64,
    pub text: String,
    pub choices: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatEvent {
    pub sender: String,
    pub channel: ChatChannel,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestUpdateEvent {
    pub player_id: u64,
    pub quest_id: QuestId,
    pub status: QuestStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusEffectUpdate {
    pub player_id: u64,
    pub effects: Vec<StatusEffect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemNotice {
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
