use bevy::prelude::*;
use shared::{MovementComponentsPlugin, Position};

mod network;
mod systems;

#[derive(Component)]
pub struct Player;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MovementComponentsPlugin)
        .add_systems(Startup, (setup, network::setup_network))
        .add_systems(
            Update,
            (
                systems::input::capture_movement_intent,
                network::receive_server_state,
                systems::movement::sync_transform_system,
                systems::render::y_sorting_system,
            )
                .chain(),
        )
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        Sprite::from_color(Color::srgb(0.16, 0.16, 0.18), Vec2::new(2000.0, 2000.0)),
        Transform::from_xyz(0.0, 0.0, -100.0),
    ));

    commands.spawn((
        Player,
        systems::render::YSortable,
        Position::default(),
        Sprite::from_color(Color::srgb(0.1, 0.4, 1.0), Vec2::splat(32.0)),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
}
