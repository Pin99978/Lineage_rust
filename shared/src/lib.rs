use bevy::prelude::*;

pub mod components;
pub mod protocol;

pub use components::ai::{AggroRange, AiState, AttackCooldown};
pub use components::combat::{
    spell_def, ActionState, ArmorClass, CombatStats, Health, Mana, SpellCooldowns, SpellType,
};
pub use components::item::{
    item_modifier, item_slot, EquipmentMap, EquipmentSlot, GroundItem, Inventory, ItemType,
    LootDropEntry, LootTable, StatModifier,
};
pub use components::movement::{MoveSpeed, PathQueue, Position, TargetPosition};
pub use components::npc::{Npc, NpcMarker, NpcType};
pub use components::world::{SpawnType, Spawner};

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
            .register_type::<ArmorClass>()
            .register_type::<CombatStats>()
            .register_type::<ActionState>()
            .register_type::<SpellType>()
            .register_type::<SpellCooldowns>()
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
            .register_type::<SpawnType>()
            .register_type::<Spawner>();
    }
}
