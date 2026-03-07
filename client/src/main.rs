use bevy::prelude::*;
use shared::{Health, MovementComponentsPlugin, Position};

mod network;
mod systems;

#[derive(Component)]
pub struct Player;

fn main() {
    let username = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "adventurer".to_string());

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MovementComponentsPlugin)
        .init_state::<systems::ui::AppState>()
        .insert_resource(systems::ui::LoginName {
            username,
            submitted: false,
        })
        .add_message::<systems::combat_render::DamagePopupEvent>()
        .add_message::<systems::combat_render::DeathVisualEvent>()
        .add_systems(Startup, (setup_camera, network::setup_network))
        .add_systems(
            OnEnter(systems::ui::AppState::LoginMenu),
            systems::ui::setup_login_menu,
        )
        .add_systems(
            OnExit(systems::ui::AppState::LoginMenu),
            systems::ui::cleanup_login_menu,
        )
        .add_systems(
            OnEnter(systems::ui::AppState::InGame),
            (setup, systems::ui::setup_ui),
        )
        .add_systems(
            Update,
            (
                network::receive_server_state,
                systems::ui::login_submit_system.run_if(in_state(systems::ui::AppState::LoginMenu)),
                systems::input::capture_movement_intent
                    .run_if(in_state(systems::ui::AppState::InGame)),
                systems::combat_render::attach_world_health_bars,
                systems::ui::update_player_health_hud,
                systems::combat_render::update_world_health_bars,
                systems::combat_render::apply_damage_feedback,
                systems::combat_render::apply_death_feedback,
                systems::movement::sync_transform_system,
                systems::render::y_sorting_system,
                systems::combat_render::animate_damage_popups,
            )
                .chain(),
        )
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Sprite::from_color(Color::srgb(0.16, 0.16, 0.18), Vec2::new(2000.0, 2000.0)),
        Transform::from_xyz(0.0, 0.0, -100.0),
    ));

    commands.spawn((
        Player,
        systems::render::YSortable,
        Position::default(),
        Health::default(),
        Sprite::from_color(Color::srgb(0.1, 0.4, 1.0), Vec2::splat(32.0)),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
