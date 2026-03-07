use bevy::prelude::*;

use crate::network::NetworkEntityVisual;

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
