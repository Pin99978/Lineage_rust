use bevy::prelude::*;
use shared::protocol::LoginRequest;
use shared::Health;

use crate::{network, Player};

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    LoginMenu,
    InGame,
}

#[derive(Resource)]
pub struct LoginName {
    pub username: String,
    pub submitted: bool,
}

#[derive(Component)]
pub struct LoginMenuRoot;

#[derive(Component)]
pub struct PlayerHealthBarUi;

pub fn setup_login_menu(mut commands: Commands, login_name: Option<Res<LoginName>>) {
    let username = login_name
        .map(|value| value.username.clone())
        .unwrap_or_else(|| "adventurer".to_string());
    commands.spawn((
        LoginMenuRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..Default::default()
        },
        BackgroundColor(Color::srgb(0.08, 0.08, 0.09)),
        children![(
            Node {
                width: Val::Px(560.0),
                height: Val::Px(160.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceEvenly,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            BackgroundColor(Color::srgba(0.12, 0.12, 0.14, 0.9)),
            children![
                (Text::new("Login Menu"), TextFont::from_font_size(32.0)),
                (
                    Text::new(format!("Username: {}", username)),
                    TextFont::from_font_size(22.0)
                ),
                (
                    Text::new("Press Enter to Login"),
                    TextFont::from_font_size(18.0)
                )
            ]
        )],
    ));
}

pub fn login_submit_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    login_name: Option<ResMut<LoginName>>,
    network: Option<Res<network::ClientNetwork>>,
) {
    if !keyboard.just_pressed(KeyCode::Enter) {
        return;
    }
    let Some(mut login_name) = login_name else {
        return;
    };
    if login_name.submitted {
        return;
    }
    let Some(network) = network else {
        return;
    };

    login_name.submitted = true;
    network::send_login_request(
        &network,
        LoginRequest {
            username: login_name.username.clone(),
        },
    );
}

pub fn cleanup_login_menu(mut commands: Commands, roots: Query<Entity, With<LoginMenuRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}

pub fn setup_ui(mut commands: Commands) {
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                bottom: Val::Px(20.0),
                width: Val::Px(260.0),
                height: Val::Px(54.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..Default::default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.06, 0.85)),
        ))
        .id();

    let health_bg = commands
        .spawn((
            Node {
                width: Val::Px(220.0),
                height: Val::Px(16.0),
                ..Default::default()
            },
            BackgroundColor(Color::srgb(0.12, 0.02, 0.02)),
        ))
        .id();

    let health_fill = commands
        .spawn((
            PlayerHealthBarUi,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..Default::default()
            },
            BackgroundColor(Color::srgb(0.86, 0.12, 0.12)),
        ))
        .id();

    commands.entity(root).add_child(health_bg);
    commands.entity(health_bg).add_child(health_fill);
}

pub fn update_player_health_hud(
    player_health: Query<&Health, (With<Player>, Changed<Health>)>,
    mut bar_query: Query<&mut Node, With<PlayerHealthBarUi>>,
) {
    let Ok(health) = player_health.single() else {
        return;
    };
    let Ok(mut bar_node) = bar_query.single_mut() else {
        return;
    };

    let ratio = if health.max > 0 {
        (health.current as f32 / health.max as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };

    bar_node.width = Val::Percent(ratio * 100.0);
}
