use bevy::prelude::*;
use shared::protocol::NetworkEntityKind;

#[derive(Component)]
pub struct YSortable;

pub fn y_sorting_system(mut query: Query<&mut Transform, With<YSortable>>) {
    for mut transform in &mut query {
        transform.translation.z = -transform.translation.y * 0.0001;
    }
}

pub fn color_for_network_kind(kind: NetworkEntityKind) -> Color {
    match kind {
        NetworkEntityKind::Player => Color::srgb(0.1, 0.6, 1.0),
        NetworkEntityKind::Enemy => Color::srgb(0.85, 0.25, 0.2),
        NetworkEntityKind::NpcMerchant => Color::srgb(0.2, 0.85, 0.3),
        NetworkEntityKind::LootGold => Color::srgb(0.95, 0.82, 0.2),
        NetworkEntityKind::LootHealthPotion => Color::srgb(0.82, 0.12, 0.12),
    }
}
