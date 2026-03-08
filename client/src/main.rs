use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::view::{ColorGrading, ColorGradingGlobal};
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
        .insert_resource(systems::ui::HudState::default())
        .insert_resource(systems::ui::DialogState::default())
        .insert_resource(systems::ui::chat::ChatUiState::default())
        .insert_resource(systems::ui::inventory::LocalInventoryState::default())
        .insert_resource(systems::ui::paperdoll::LocalEquipmentState::default())
        .insert_resource(systems::ui::inventory::UiWindowsState::default())
        .add_message::<systems::combat_render::DamagePopupEvent>()
        .add_message::<systems::combat_render::DeathVisualEvent>()
        .add_message::<systems::animation::PlayAttackAnimation>()
        .add_systems(
            Startup,
            (
                setup_camera,
                network::setup_network,
                systems::animation::setup_character_visual_assets,
            ),
        )
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
                systems::ui::chat::chat_focus_and_send_system
                    .run_if(in_state(systems::ui::AppState::InGame)),
                systems::ui::chat::chat_text_input_system
                    .run_if(in_state(systems::ui::AppState::InGame)),
                systems::ui::dialog_choice_input_system
                    .run_if(in_state(systems::ui::AppState::InGame)),
                systems::ui::inventory::toggle_inventory_window_system
                    .run_if(in_state(systems::ui::AppState::InGame)),
                systems::ui::paperdoll::toggle_paperdoll_window_system
                    .run_if(in_state(systems::ui::AppState::InGame)),
                systems::interaction::capture_click_intent
                    .run_if(in_state(systems::ui::AppState::InGame)),
                systems::input::capture_movement_intent
                    .run_if(in_state(systems::ui::AppState::InGame)),
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                systems::animation::attach_animation_components,
                systems::animation::apply_character_atlas_when_ready,
                systems::animation::trigger_attack_animation,
                systems::animation::update_animation_state,
                systems::animation::animate_sprite_system,
                systems::animation::update_static_character_visual,
                systems::combat_render::attach_world_health_bars,
                systems::ui::update_player_health_hud,
                systems::ui::update_player_mana_hud,
                systems::ui::update_equipment_text_hud,
                systems::ui::update_status_effects_hud,
                systems::ui::update_dialog_hud,
                systems::ui::chat::update_chat_ui_system,
                systems::ui::inventory::apply_inventory_visibility_system,
                systems::ui::paperdoll::apply_paperdoll_visibility_system,
                systems::ui::inventory::refresh_inventory_ui_system,
                systems::ui::paperdoll::refresh_paperdoll_ui_system,
                systems::ui::inventory::inventory_click_equip_system,
                systems::ui::paperdoll::paperdoll_click_unequip_system,
                systems::ui::quest_log::update_quest_log_ui,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                systems::combat_render::update_world_health_bars,
                systems::combat_render::apply_damage_feedback,
                systems::combat_render::apply_death_feedback,
                systems::render::sync_map_background_system,
                systems::movement::sync_transform_system,
                systems::render::y_sorting_system,
                systems::combat_render::animate_damage_popups,
            )
                .chain(),
        )
        .run();
}

fn setup(mut commands: Commands, asset_server: Option<Res<AssetServer>>) {
    commands.spawn((
        Name::new("FallbackBackground"),
        Sprite::from_color(Color::srgb(0.16, 0.16, 0.18), Vec2::new(2000.0, 2000.0)),
        Transform::from_xyz(0.0, 0.0, -20.0),
    ));

    if let Some(asset_server) = asset_server {
        let town_bg: Handle<Image> = asset_server.load("textures/map_bg.png");
        let dungeon_bg: Handle<Image> = asset_server.load("textures/map_bg.png");
        commands.insert_resource(systems::render::MapBackgrounds {
            town: town_bg.clone(),
            dungeon_1: dungeon_bg.clone(),
        });
        commands.spawn((
            Name::new("MapBackground"),
            systems::render::MapBackground,
            Sprite::from_image(town_bg),
            Transform::from_xyz(0.0, 0.0, -10.0),
        ));
    }

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
    commands.spawn((
        Camera2d,
        Bloom::NATURAL,
        Tonemapping::TonyMcMapface,
        ColorGrading {
            global: ColorGradingGlobal {
                exposure: -0.2,
                ..Default::default()
            },
            ..Default::default()
        },
    ));
}
