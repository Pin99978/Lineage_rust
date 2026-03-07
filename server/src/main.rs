use bevy::{app::ScheduleRunnerPlugin, prelude::*};
use shared::MovementComponentsPlugin;
use std::time::Duration;

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
        .add_systems(
            Startup,
            (network::setup_network, systems::combat::spawn_target_dummy).chain(),
        )
        .add_systems(
            Update,
            (
                network::receive_client_messages,
                systems::movement::movement_system,
                systems::combat::combat_system,
                network::broadcast_world_state,
                network::broadcast_combat_events,
            )
                .chain(),
        )
        .run();
}
