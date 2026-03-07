use bevy::prelude::*;
use shared::{AggroRange, AiState, AttackCooldown, CombatStats, Health, Position, TargetPosition};

use crate::{network, systems::combat};

#[derive(Component)]
pub struct EnemyAi;

pub fn spawn_enemies(mut commands: Commands, network: Option<ResMut<network::ServerNetwork>>) {
    let Some(mut network) = network else {
        return;
    };

    let enemy_positions = [Vec2::new(180.0, 120.0), Vec2::new(260.0, -70.0)];
    for position in enemy_positions {
        let enemy_id = network.allocate_entity_id();
        commands.spawn((
            EnemyAi,
            network::NetworkEntity {
                id: enemy_id,
                kind: shared::protocol::NetworkEntityKind::Enemy,
            },
            Position {
                x: position.x,
                y: position.y,
            },
            TargetPosition {
                x: position.x,
                y: position.y,
            },
            Health {
                current: 120,
                max: 120,
            },
            CombatStats {
                attack_power: 12,
                attack_range: 52.0,
                attack_speed: 1.2,
            },
            shared::MoveSpeed { value: 220.0 },
            AggroRange(300.0),
            AiState::Idle,
            AttackCooldown::default(),
        ));
    }
}

pub fn ai_aggro_system(
    mut enemies: Query<(&Position, &AggroRange, &mut AiState), With<EnemyAi>>,
    players: Query<(Entity, &Position, &Health), With<network::PlayerCharacter>>,
) {
    for (enemy_position, aggro_range, mut ai_state) in &mut enemies {
        if !matches!(*ai_state, AiState::Idle) {
            continue;
        }

        let mut nearest: Option<(Entity, f32)> = None;
        for (player_entity, player_position, player_health) in &players {
            if player_health.current <= 0 {
                continue;
            }
            let distance = Vec2::new(
                player_position.x - enemy_position.x,
                player_position.y - enemy_position.y,
            )
            .length();
            if distance > aggro_range.0 {
                continue;
            }

            let should_replace = nearest
                .map(|(_, best_distance)| distance < best_distance)
                .unwrap_or(true);
            if should_replace {
                nearest = Some((player_entity, distance));
            }
        }

        if let Some((target_entity, _)) = nearest {
            *ai_state = AiState::Chasing(target_entity);
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn ai_chase_and_attack_system(
    time: Res<Time>,
    mut enemies: Query<
        (
            &Position,
            &AggroRange,
            &CombatStats,
            &mut AiState,
            &mut TargetPosition,
            &mut AttackCooldown,
        ),
        With<EnemyAi>,
    >,
    mut players: Query<
        (Entity, &Position, &network::NetworkEntity, &mut Health),
        With<network::PlayerCharacter>,
    >,
    mut damage_events: MessageWriter<combat::CombatDamageEvent>,
    mut death_events: MessageWriter<combat::CombatDeathEvent>,
) {
    for (
        enemy_position,
        aggro_range,
        combat_stats,
        mut ai_state,
        mut target_position,
        mut cooldown,
    ) in &mut enemies
    {
        cooldown.remaining_secs = (cooldown.remaining_secs - time.delta_secs()).max(0.0);

        let target_entity = match *ai_state {
            AiState::Chasing(target) | AiState::Attacking(target) => target,
            AiState::Idle => continue,
        };

        let Ok((_player_entity, player_position, player_network, mut player_health)) =
            players.get_mut(target_entity)
        else {
            *ai_state = AiState::Idle;
            continue;
        };

        if player_health.current <= 0 {
            *ai_state = AiState::Idle;
            continue;
        }

        let to_player = Vec2::new(
            player_position.x - enemy_position.x,
            player_position.y - enemy_position.y,
        );
        let distance = to_player.length();
        if distance > aggro_range.0 * 1.5 {
            *ai_state = AiState::Idle;
            continue;
        }

        if distance <= combat_stats.attack_range {
            *ai_state = AiState::Attacking(target_entity);
            target_position.x = enemy_position.x;
            target_position.y = enemy_position.y;

            if cooldown.remaining_secs > 0.0 {
                continue;
            }

            let damage = combat_stats.attack_power.max(1);
            player_health.current = (player_health.current - damage).max(0);
            cooldown.remaining_secs = (1.0 / combat_stats.attack_speed.max(0.2)).max(0.1);

            damage_events.write(combat::CombatDamageEvent {
                target_id: player_network.id,
                amount: damage,
                remaining_hp: player_health.current,
            });

            if player_health.current == 0 {
                death_events.write(combat::CombatDeathEvent {
                    target_id: player_network.id,
                });
            }
        } else {
            *ai_state = AiState::Chasing(target_entity);
            target_position.x = player_position.x;
            target_position.y = player_position.y;
        }
    }
}
