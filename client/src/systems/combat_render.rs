use bevy::prelude::*;
use shared::Health;

use crate::network::{Attackable, NetworkEntityVisual};

#[derive(Message, Debug, Clone, Copy)]
pub struct DamagePopupEvent {
    pub target_id: u64,
    pub amount: i32,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct DeathVisualEvent {
    pub target_id: u64,
}

#[derive(Component)]
pub struct DamagePopup {
    pub timer: Timer,
}

#[derive(Component)]
pub struct WorldHealthBarAttached;

#[derive(Component)]
pub struct WorldHealthBarFill {
    pub parent: Entity,
    pub max_width: f32,
}

#[derive(Component)]
pub struct WorldHealthBarBackground {
    pub parent: Entity,
}

pub fn apply_damage_feedback(
    mut commands: Commands,
    mut events: MessageReader<DamagePopupEvent>,
    mut targets: Query<(&NetworkEntityVisual, &Transform, Option<&mut Sprite>)>,
) {
    for event in events.read() {
        for (entity_visual, transform, sprite) in &mut targets {
            if entity_visual.id != event.target_id {
                continue;
            }

            if let Some(mut sprite) = sprite {
                sprite.color = Color::srgb(1.0, 0.25, 0.25);
            }

            commands.spawn((
                Text2d::new(format!("-{}", event.amount)),
                TextFont {
                    font_size: 20.0,
                    ..Default::default()
                },
                TextColor(Color::srgb(1.0, 0.8, 0.2)),
                Transform::from_xyz(
                    transform.translation.x,
                    transform.translation.y + 24.0,
                    transform.translation.z + 0.5,
                ),
                DamagePopup {
                    timer: Timer::from_seconds(0.6, TimerMode::Once),
                },
            ));
            break;
        }
    }
}

pub fn apply_death_feedback(
    mut events: MessageReader<DeathVisualEvent>,
    mut targets: Query<(&NetworkEntityVisual, &mut Sprite)>,
) {
    for event in events.read() {
        for (entity_visual, mut sprite) in &mut targets {
            if entity_visual.id == event.target_id {
                sprite.color = Color::srgb(0.35, 0.35, 0.35);
            }
        }
    }
}

pub fn animate_damage_popups(
    mut commands: Commands,
    time: Res<Time>,
    mut popups: Query<(Entity, &mut Transform, &mut DamagePopup)>,
) {
    for (entity, mut transform, mut popup) in &mut popups {
        popup.timer.tick(time.delta());
        transform.translation.y += 45.0 * time.delta_secs();

        if popup.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn attach_world_health_bars(
    mut commands: Commands,
    enemies: Query<(Entity, &Health), (With<Attackable>, Without<WorldHealthBarAttached>)>,
) {
    for (enemy_entity, health) in &enemies {
        if health.max <= 0 {
            continue;
        }

        let bg_bar = commands
            .spawn((
                Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.8), Vec2::new(36.0, 5.0)),
                Transform::from_xyz(0.0, 28.0, 0.25),
                Visibility::Hidden,
                WorldHealthBarBackground {
                    parent: enemy_entity,
                },
            ))
            .id();

        let fill_bar = commands
            .spawn((
                Sprite::from_color(Color::srgb(0.88, 0.15, 0.15), Vec2::new(34.0, 3.0)),
                Transform::from_xyz(0.0, 28.0, 0.3),
                Visibility::Hidden,
                WorldHealthBarFill {
                    parent: enemy_entity,
                    max_width: 34.0,
                },
            ))
            .id();

        commands.entity(enemy_entity).add_child(bg_bar);
        commands.entity(enemy_entity).add_child(fill_bar);
        commands.entity(enemy_entity).insert(WorldHealthBarAttached);
    }
}

pub fn update_world_health_bars(
    health_query: Query<&Health>,
    mut fill_bars: Query<
        (
            &WorldHealthBarFill,
            &mut Sprite,
            &mut Transform,
            &mut Visibility,
        ),
        Without<WorldHealthBarBackground>,
    >,
    mut bg_bars: Query<(&WorldHealthBarBackground, &mut Visibility), Without<WorldHealthBarFill>>,
) {
    for (fill, mut sprite, mut transform, mut visibility) in &mut fill_bars {
        let Ok(health) = health_query.get(fill.parent) else {
            continue;
        };
        if health.max <= 0 || health.current <= 0 {
            *visibility = Visibility::Hidden;
            continue;
        }

        let ratio = (health.current as f32 / health.max as f32).clamp(0.0, 1.0);
        if ratio >= 0.999 {
            *visibility = Visibility::Hidden;
            continue;
        }

        *visibility = Visibility::Visible;
        let width = (fill.max_width * ratio).max(1.0);
        sprite.custom_size = Some(Vec2::new(width, 3.0));
        transform.translation.x = -(fill.max_width - width) * 0.5;
    }

    for (bg, mut visibility) in &mut bg_bars {
        let Ok(health) = health_query.get(bg.parent) else {
            continue;
        };
        if health.max > 0 && health.current > 0 && health.current < health.max {
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}
