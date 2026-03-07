use bevy::prelude::*;
use shared::{Position, SpawnType, Spawner};

use crate::{network, systems::ai};

pub fn spawner_system(
    time: Res<Time>,
    mut commands: Commands,
    network: Option<ResMut<network::ServerNetwork>>,
    mut spawners: Query<(Entity, &Position, &mut Spawner)>,
    existing: Query<(
        Entity,
        &network::NetworkEntity,
        &Position,
        &shared::Health,
        &ai::EnemyAi,
    )>,
) {
    let Some(mut network) = network else {
        return;
    };

    for (_spawner_entity, spawner_position, mut spawner) in &mut spawners {
        spawner.cooldown_remaining = (spawner.cooldown_remaining - time.delta_secs()).max(0.0);

        spawner.active_entities.retain(|entity| {
            existing
                .get(*entity)
                .map(|(_, network_entity, _, health, _)| {
                    network_entity.kind == shared::protocol::NetworkEntityKind::Enemy
                        && health.current > 0
                })
                .unwrap_or(false)
        });

        let living_count = existing
            .iter()
            .filter(|(_, _, position, health, _)| {
                if health.current <= 0 {
                    return false;
                }
                Vec2::new(
                    position.x - spawner_position.x,
                    position.y - spawner_position.y,
                )
                .length()
                    <= spawner.radius + 8.0
            })
            .count() as u32;

        if living_count >= spawner.max_count {
            continue;
        }
        if spawner.cooldown_remaining > 0.0 {
            continue;
        }

        if matches!(spawner.spawn_type, SpawnType::Enemy) {
            let sequence = living_count + spawner.active_entities.len() as u32 + 1;
            let spawned = ai::spawn_enemy_at(
                &mut commands,
                &mut network,
                random_point(spawner_position, spawner.radius, sequence),
            );
            spawner.active_entities.push(spawned);
            spawner.cooldown_remaining = spawner.cooldown_secs.max(0.2);
        }
    }
}

fn random_point(center: &Position, radius: f32, sequence: u32) -> Vec2 {
    let seed = sequence as f32 * 0.618_033_95;
    let angle = seed.fract() * std::f32::consts::TAU;
    let length = (seed * 1.73).fract() * radius.max(8.0);
    Vec2::new(
        center.x + angle.cos() * length,
        center.y + angle.sin() * length,
    )
}
