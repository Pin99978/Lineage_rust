use bevy::{app::ScheduleRunnerPlugin, prelude::*};
use shared::MovementComponentsPlugin;
use std::time::Duration;

mod db;
mod map_data;
mod network;
mod systems;

fn main() {
    App::new()
        .add_plugins(
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                1.0 / 60.0,
            ))),
        )
        .add_plugins(MovementComponentsPlugin)
        .add_message::<systems::combat::AttackRequest>()
        .add_message::<systems::combat::CombatDamageEvent>()
        .add_message::<systems::combat::CombatDeathEvent>()
        .add_message::<systems::drop::ItemSpawnedMessage>()
        .add_message::<systems::loot::LootRequest>()
        .add_message::<systems::loot::ItemDespawnedMessage>()
        .add_message::<systems::loot::InventoryUpdateMessage>()
        .add_message::<systems::spell::CastSpellRequest>()
        .add_message::<systems::spell::ManaChangedMessage>()
        .add_message::<systems::spell::HealEventMessage>()
        .add_message::<systems::equipment::EquipRequest>()
        .add_message::<systems::equipment::UnequipRequest>()
        .add_message::<systems::equipment::EquipmentChangedMessage>()
        .add_message::<systems::interaction::InteractRequest>()
        .add_message::<systems::interaction::DialogMessage>()
        .add_message::<systems::interaction::MapChangedMessage>()
        .add_message::<systems::npc::NpcInteractRequest>()
        .add_message::<systems::npc::DialogueMessage>()
        .add_message::<systems::quest::QuestUpdatedMessage>()
        .add_message::<systems::chat::ChatRequest>()
        .add_message::<systems::chat::ChatDelivery>()
        .add_message::<systems::movement::MoveRequest>()
        .add_message::<systems::item::UseItemRequest>()
        .add_message::<systems::combat::StatusEffectsChangedMessage>()
        .add_systems(
            Startup,
            (
                db::setup_db,
                network::setup_network,
                map_data::setup_world_map,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                network::receive_client_messages,
                network::cleanup_stale_sessions,
                network::apply_db_results,
                systems::movement::process_move_requests,
                systems::spawner::spawner_system,
                systems::ai::ai_aggro_system,
                systems::ai::ai_chase_and_attack_system,
                systems::movement::movement_system,
                systems::combat::combat_system,
                systems::combat::update_status_effects_system,
                systems::spell::tick_spell_cooldowns,
                systems::spell::cast_spell_system,
                systems::item::use_item_system,
                systems::interaction::interaction_system,
                systems::interaction::portal_system,
                systems::npc::convert_legacy_dialog_to_npc,
                systems::npc::npc_dialogue_system,
                systems::chat::chat_system,
                systems::quest::track_enemy_kill_quest_system,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                systems::drop::item_drop_system,
                systems::loot::loot_system,
                systems::equipment::equip_system,
                systems::equipment::unequip_system,
                systems::combat::log_player_death_system,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                network::broadcast_map_change_events,
                network::broadcast_world_state,
                network::broadcast_combat_events,
                network::broadcast_item_events,
                network::broadcast_spell_events,
                network::broadcast_equipment_events,
                network::broadcast_dialog_events,
                network::broadcast_chat_events,
                network::broadcast_quest_events,
                network::broadcast_status_effect_events,
                db::periodic_save_players,
            )
                .chain(),
        )
        .add_systems(Update, db::save_player_progress_on_change)
        .run();
}
