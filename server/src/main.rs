use bevy::{app::ScheduleRunnerPlugin, prelude::*};
use shared::MovementComponentsPlugin;
use std::time::Duration;

mod db;
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
        .add_systems(
            Startup,
            (
                db::setup_db,
                network::setup_network,
                systems::ai::spawn_enemies,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                network::receive_client_messages,
                network::apply_db_results,
                systems::ai::ai_aggro_system,
                systems::ai::ai_chase_and_attack_system,
                systems::movement::movement_system,
                systems::combat::combat_system,
                systems::drop::item_drop_system,
                systems::loot::loot_system,
                systems::combat::log_player_death_system,
                network::broadcast_world_state,
                network::broadcast_combat_events,
                network::broadcast_item_events,
                db::periodic_save_players,
            )
                .chain(),
        )
        .run();
}
