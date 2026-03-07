use bevy::prelude::*;
use shared::{MoveSpeed, MovementComponentsPlugin, Position, TargetPosition};

mod systems;

#[derive(Component)]
pub struct Player;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MovementComponentsPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                systems::input::capture_movement_intent,
                systems::movement::movement_system,
                systems::movement::sync_transform_system,
            ),
        )
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Player,
        Position::default(),
        TargetPosition::default(),
        MoveSpeed { value: 320.0 },
        Sprite::from_color(Color::srgb(0.1, 0.4, 1.0), Vec2::splat(32.0)),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
}
