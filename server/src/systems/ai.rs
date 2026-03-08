use bevy::prelude::*;
use shared::{
    AggroRange, AiState, AttackCooldown, Buffs, CombatStats, Health, LootTable, PathQueue,
    Position, TargetPosition,
};

use crate::{
    map_data::MapManager,
    network,
    systems::{combat, movement},
};

#[derive(Component)]
pub struct EnemyAi;

#[derive(Component, Debug, Clone)]
pub struct EnemyPathRepathTimer {
    pub timer: Timer,
}

impl Default for EnemyPathRepathTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.8, TimerMode::Repeating),
        }
    }
}

pub fn spawn_enemy_at(
    commands: &mut Commands,
    network: &mut ResMut<network::ServerNetwork>,
    map_id: &shared::MapId,
    position: Vec2,
) -> Entity {
    let enemy_id = network.allocate_entity_id();
    let entity = commands.spawn((
        EnemyAi,
        network::NetworkEntity {
            id: enemy_id,
            kind: shared::protocol::NetworkEntityKind::Enemy,
        },
        shared::MapId(map_id.0.clone()),
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
        Buffs::default(),
        LootTable::default(),
        CombatStats {
            attack_power: 12,
            attack_range: 52.0,
            attack_speed: 1.2,
        },
        shared::MoveSpeed { value: 220.0 },
        AggroRange(300.0),
        AiState::Idle,
        AttackCooldown::default(),
        PathQueue::default(),
        EnemyPathRepathTimer::default(),
    ));
    entity.id()
}

pub fn ai_aggro_system(
    mut enemies: Query<
        (
            &Position,
            &shared::MapId,
            &AggroRange,
            &Health,
            &mut AiState,
        ),
        With<EnemyAi>,
    >,
    players: Query<(Entity, &Position, &shared::MapId, &Health), With<network::PlayerCharacter>>,
) {
    for (enemy_position, enemy_map, aggro_range, enemy_health, mut ai_state) in &mut enemies {
        if enemy_health.current <= 0 {
            *ai_state = AiState::Idle;
            continue;
        }
        if !matches!(*ai_state, AiState::Idle) {
            continue;
        }

        let mut nearest: Option<(Entity, f32)> = None;
        for (player_entity, player_position, player_map, player_health) in &players {
            if player_map.0 != enemy_map.0 {
                continue;
            }
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
            &shared::MapId,
            &AggroRange,
            &CombatStats,
            &Health,
            &mut AiState,
            &mut TargetPosition,
            &mut AttackCooldown,
            &mut PathQueue,
            &mut EnemyPathRepathTimer,
        ),
        (With<EnemyAi>, Without<network::PlayerCharacter>),
    >,
    mut players: Query<
        (
            Entity,
            &Position,
            &shared::MapId,
            &network::NetworkEntity,
            &mut Health,
            &mut Buffs,
        ),
        (With<network::PlayerCharacter>, Without<EnemyAi>),
    >,
    mut damage_events: MessageWriter<combat::CombatDamageEvent>,
    mut death_events: MessageWriter<combat::CombatDeathEvent>,
    mut status_updates: MessageWriter<combat::StatusEffectsChangedMessage>,
    maps: Option<Res<MapManager>>,
) {
    let Some(maps) = maps else {
        return;
    };

    for (
        enemy_position,
        enemy_map,
        aggro_range,
        combat_stats,
        enemy_health,
        mut ai_state,
        mut target_position,
        mut cooldown,
        mut path_queue,
        mut repath_timer,
    ) in &mut enemies
    {
        if enemy_health.current <= 0 {
            *ai_state = AiState::Idle;
            path_queue.waypoints.clear();
            continue;
        }
        cooldown.remaining_secs = (cooldown.remaining_secs - time.delta_secs()).max(0.0);
        repath_timer.timer.tick(time.delta());

        let target_entity = match *ai_state {
            AiState::Chasing(target) | AiState::Attacking(target) => target,
            AiState::Idle => continue,
        };

        let Ok((
            _player_entity,
            player_position,
            player_map,
            player_network,
            mut player_health,
            mut player_buffs,
        )) = players.get_mut(target_entity)
        else {
            *ai_state = AiState::Idle;
            path_queue.waypoints.clear();
            continue;
        };

        if player_health.current <= 0 {
            *ai_state = AiState::Idle;
            path_queue.waypoints.clear();
            continue;
        }
        if player_map.0 != enemy_map.0 {
            *ai_state = AiState::Idle;
            path_queue.waypoints.clear();
            continue;
        }

        let to_player = Vec2::new(
            player_position.x - enemy_position.x,
            player_position.y - enemy_position.y,
        );
        let distance = to_player.length();
        if distance > aggro_range.0 * 1.5 {
            *ai_state = AiState::Idle;
            path_queue.waypoints.clear();
            continue;
        }

        if distance <= combat_stats.attack_range {
            *ai_state = AiState::Attacking(target_entity);
            target_position.x = enemy_position.x;
            target_position.y = enemy_position.y;
            path_queue.waypoints.clear();

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

            let poisoned = combat::add_or_refresh_poison(&mut player_buffs, 4.0, 3.0);
            if poisoned {
                status_updates.write(combat::StatusEffectsChangedMessage {
                    player_id: player_network.id,
                    effects: player_buffs.effects.clone(),
                });
            }

            if player_health.current == 0 {
                death_events.write(combat::CombatDeathEvent {
                    target_entity: target_entity,
                    target_id: player_network.id,
                    killer_player_id: None,
                });
            }
        } else {
            *ai_state = AiState::Chasing(target_entity);
            if !repath_timer.timer.just_finished() && !path_queue.waypoints.is_empty() {
                if let Some(next) = path_queue.waypoints.front() {
                    target_position.x = next.x;
                    target_position.y = next.y;
                }
                continue;
            }

            let from = Vec2::new(enemy_position.x, enemy_position.y);
            let to = Vec2::new(player_position.x, player_position.y);
            let Some(grid) = maps.grids.get(&enemy_map.0) else {
                *ai_state = AiState::Idle;
                path_queue.waypoints.clear();
                continue;
            };
            let Some(new_path) = movement::compute_path_world(grid, from, to) else {
                *ai_state = AiState::Idle;
                path_queue.waypoints.clear();
                continue;
            };
            if new_path.len() > 64 {
                *ai_state = AiState::Idle;
                path_queue.waypoints.clear();
                continue;
            }

            path_queue.waypoints = new_path;
            if let Some(next) = path_queue.waypoints.front() {
                target_position.x = next.x;
                target_position.y = next.y;
            }
        }
    }
}
