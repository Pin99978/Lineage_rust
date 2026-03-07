use bevy::prelude::*;
use shared::{ActionState, CombatStats, Health, Position};

use crate::network;

#[derive(Message, Debug, Clone, Copy)]
pub struct AttackRequest {
    pub attacker_entity: Entity,
    pub target_id: u64,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct CombatDamageEvent {
    pub target_id: u64,
    pub amount: i32,
    pub remaining_hp: i32,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct CombatDeathEvent {
    pub target_id: u64,
}

pub fn spawn_target_dummy(mut commands: Commands, network: Option<ResMut<network::ServerNetwork>>) {
    let Some(mut network) = network else {
        return;
    };

    let dummy_id = network.allocate_entity_id();
    commands.spawn((
        network::NetworkEntity {
            id: dummy_id,
            kind: shared::protocol::NetworkEntityKind::Dummy,
        },
        Position { x: 220.0, y: 0.0 },
        TargetDummy,
        Health {
            current: 150,
            max: 150,
        },
    ));
}

#[derive(Component)]
pub struct TargetDummy;

pub fn combat_system(
    mut attack_requests: MessageReader<AttackRequest>,
    mut damage_events: MessageWriter<CombatDamageEvent>,
    mut death_events: MessageWriter<CombatDeathEvent>,
    mut attackers: Query<
        (&Position, &CombatStats, &mut ActionState),
        With<network::PlayerCharacter>,
    >,
    target_lookup: Query<(Entity, &network::NetworkEntity, &Position)>,
    mut target_health: Query<&mut Health>,
) {
    for request in attack_requests.read() {
        let Ok((attacker_position, combat_stats, mut action_state)) =
            attackers.get_mut(request.attacker_entity)
        else {
            continue;
        };

        let Some((target_entity, target_position)) = target_lookup
            .iter()
            .find(|(_, network_entity, _)| network_entity.id == request.target_id)
            .map(|(entity, _, position)| (entity, *position))
        else {
            continue;
        };

        let distance = Vec2::new(
            target_position.x - attacker_position.x,
            target_position.y - attacker_position.y,
        )
        .length();

        if distance > combat_stats.attack_range {
            action_state.is_attacking = false;
            action_state.target = None;
            continue;
        }

        let Ok(mut health) = target_health.get_mut(target_entity) else {
            continue;
        };
        if health.current <= 0 {
            continue;
        }

        let damage = combat_stats.attack_power.max(1);
        health.current = (health.current - damage).max(0);

        action_state.is_attacking = true;
        action_state.target = Some(target_entity);

        damage_events.write(CombatDamageEvent {
            target_id: request.target_id,
            amount: damage,
            remaining_hp: health.current,
        });

        if health.current == 0 {
            death_events.write(CombatDeathEvent {
                target_id: request.target_id,
            });
        }
    }
}
