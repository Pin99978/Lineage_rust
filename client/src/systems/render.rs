use bevy::prelude::*;
use shared::protocol::NetworkEntityKind;
use shared::MAP_DUNGEON_1;

#[derive(Component)]
pub struct YSortable;

#[derive(Component)]
pub struct MapBackground;

#[derive(Resource, Clone)]
pub struct MapBackgrounds {
    pub town: Handle<Image>,
    pub dungeon_1: Handle<Image>,
}

pub fn y_sorting_system(mut query: Query<&mut Transform, With<YSortable>>) {
    for mut transform in &mut query {
        transform.translation.z = -transform.translation.y * 0.0001;
    }
}

pub fn apply_background_for_map(sprite: &mut Sprite, map_id: &str, backgrounds: &MapBackgrounds) {
    if map_id == MAP_DUNGEON_1 {
        sprite.image = backgrounds.dungeon_1.clone();
        sprite.color = Color::srgb(0.72, 0.78, 0.90);
    } else {
        sprite.image = backgrounds.town.clone();
        sprite.color = Color::WHITE;
    }
}

pub fn sync_map_background_system(
    local_player: Option<Res<crate::network::LocalPlayer>>,
    backgrounds: Option<Res<MapBackgrounds>>,
    mut map_bg: Query<&mut Sprite, With<MapBackground>>,
    mut last_map_id: Local<String>,
) {
    let Some(local_player) = local_player else {
        return;
    };
    let Some(backgrounds) = backgrounds else {
        return;
    };
    if local_player.map_id.is_empty() || *last_map_id == local_player.map_id {
        return;
    }

    for mut sprite in &mut map_bg {
        apply_background_for_map(&mut sprite, &local_player.map_id, &backgrounds);
    }
    *last_map_id = local_player.map_id.clone();
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
