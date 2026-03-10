use bevy::prelude::*;

pub mod components;
pub mod protocol;

pub use components::ai::{AggroRange, AiState, AttackCooldown};
pub use components::combat::{
    class_def, experience_required_for_level, spell_def, ActionState, Alignment, AlignmentStatus,
    ArmorClass, BaseStats, Buffs, CharacterClass, CombatStats, EffectType, Experience, Health,
    KnownSpells, Level, Mana, SpellCooldowns, SpellType, StatusEffect,
};
pub use components::guild::{GuildMembership, GuildRole};
pub use components::item::{
    item_modifier, item_slot, scroll_spell, EquipmentMap, EquipmentSlot, GroundItem, Inventory,
    ItemType, LootDropEntry, LootTable, StatModifier,
};
pub use components::movement::{MoveSpeed, PathQueue, Position, TargetPosition};
pub use components::npc::{Npc, NpcMarker, NpcType};
pub use components::quest::{QuestEntry, QuestId, QuestStatus, QuestTracker};
pub use components::world::{MapId, Portal, SpawnType, Spawner, MAP_DUNGEON_1, MAP_TOWN};

pub struct MovementComponentsPlugin;

impl Plugin for MovementComponentsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Position>()
            .register_type::<TargetPosition>()
            .register_type::<MoveSpeed>()
            .register_type::<AggroRange>()
            .register_type::<AiState>()
            .register_type::<AttackCooldown>()
            .register_type::<Health>()
            .register_type::<Mana>()
            .register_type::<BaseStats>()
            .register_type::<Level>()
            .register_type::<Experience>()
            .register_type::<ArmorClass>()
            .register_type::<CombatStats>()
            .register_type::<ActionState>()
            .register_type::<SpellType>()
            .register_type::<CharacterClass>()
            .register_type::<SpellCooldowns>()
            .register_type::<KnownSpells>()
            .register_type::<AlignmentStatus>()
            .register_type::<Alignment>()
            .register_type::<EffectType>()
            .register_type::<StatusEffect>()
            .register_type::<Buffs>()
            .register_type::<GuildRole>()
            .register_type::<GuildMembership>()
            .register_type::<GroundItem>()
            .register_type::<Inventory>()
            .register_type::<ItemType>()
            .register_type::<EquipmentSlot>()
            .register_type::<StatModifier>()
            .register_type::<EquipmentMap>()
            .register_type::<LootDropEntry>()
            .register_type::<LootTable>()
            .register_type::<NpcType>()
            .register_type::<Npc>()
            .register_type::<NpcMarker>()
            .register_type::<QuestId>()
            .register_type::<QuestStatus>()
            .register_type::<QuestEntry>()
            .register_type::<QuestTracker>()
            .register_type::<MapId>()
            .register_type::<Portal>()
            .register_type::<SpawnType>()
            .register_type::<Spawner>();
    }
}
