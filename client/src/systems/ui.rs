use bevy::prelude::*;
use shared::Health;

use crate::Player;

#[derive(Component)]
pub struct PlayerHealthBarUi;

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
