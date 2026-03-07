use bevy::{app::ScheduleRunnerPlugin, prelude::*};
use shared::MovementComponentsPlugin;
use std::time::Duration;

mod systems;

fn main() {
    App::new()
        .add_plugins(
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                1.0 / 60.0,
            ))),
        )
        .add_plugins(MovementComponentsPlugin)
        .add_systems(Update, systems::movement::movement_system)
        .run();
}
