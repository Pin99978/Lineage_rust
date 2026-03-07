use bevy::prelude::*;
use shared::{ActionState, Buffs, CombatStats, EffectType, Health, Position, StatusEffect};

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

#[derive(Message, Debug, Clone)]
pub struct StatusEffectsChangedMessage {
    pub player_id: u64,
    pub effects: Vec<StatusEffect>,
}

pub fn combat_system(
    mut attack_requests: MessageReader<AttackRequest>,
    mut damage_events: MessageWriter<CombatDamageEvent>,
    mut death_events: MessageWriter<CombatDeathEvent>,
    mut attackers: Query<
        (&Position, &CombatStats, &mut ActionState, Option<&Buffs>),
        With<network::PlayerCharacter>,
    >,
    target_lookup: Query<(Entity, &network::NetworkEntity, &Position, Option<&Buffs>)>,
    mut target_health: Query<&mut Health>,
) {
    for request in attack_requests.read() {
        let Ok((attacker_position, combat_stats, mut action_state, attacker_buffs)) =
            attackers.get_mut(request.attacker_entity)
        else {
            continue;
        };

        let Some((target_entity, target_position, target_buffs)) = target_lookup
            .iter()
            .find(|(_, network_entity, _, _)| network_entity.id == request.target_id)
            .map(|(entity, _, position, buffs)| (entity, *position, buffs))
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

        let damage = compute_damage(combat_stats.attack_power, attacker_buffs, target_buffs);
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

pub fn update_status_effects_system(
    time: Res<Time>,
    mut entities: Query<(
        Entity,
        &network::NetworkEntity,
        &mut Health,
        &mut Buffs,
        Option<&network::PlayerCharacter>,
    )>,
    mut damage_events: MessageWriter<CombatDamageEvent>,
    mut death_events: MessageWriter<CombatDeathEvent>,
    mut status_updates: MessageWriter<StatusEffectsChangedMessage>,
) {
    let delta = time.delta_secs();

    for (_entity, network_entity, mut health, mut buffs, player_marker) in &mut entities {
        let mut changed = false;
        let mut poison_damage = 0_i32;

        for effect in &mut buffs.effects {
            effect.duration_remaining -= delta;
            if effect.effect_type == EffectType::Poison {
                effect.tick_timer -= delta;
                while effect.tick_timer <= 0.0 && effect.duration_remaining > 0.0 {
                    poison_damage += effect.strength.max(1.0) as i32;
                    effect.tick_timer += 1.0;
                    changed = true;
                }
            }
        }

        if poison_damage > 0 && health.current > 0 {
            health.current = (health.current - poison_damage).max(0);
            damage_events.write(CombatDamageEvent {
                target_id: network_entity.id,
                amount: poison_damage,
                remaining_hp: health.current,
            });
            if health.current == 0 {
                death_events.write(CombatDeathEvent {
                    target_id: network_entity.id,
                });
            }
        }

        let before = buffs.effects.len();
        buffs
            .effects
            .retain(|effect| effect.duration_remaining > 0.0);
        if buffs.effects.len() != before {
            changed = true;
        }

        if player_marker.is_some() && changed {
            status_updates.write(StatusEffectsChangedMessage {
                player_id: network_entity.id,
                effects: buffs.effects.clone(),
            });
        }
    }
}

pub fn add_or_refresh_poison(buffs: &mut Buffs, duration_secs: f32, strength: f32) -> bool {
    if let Some(existing) = buffs
        .effects
        .iter_mut()
        .find(|effect| effect.effect_type == EffectType::Poison)
    {
        existing.duration_remaining = duration_secs.max(existing.duration_remaining);
        existing.strength = existing.strength.max(strength);
        existing.tick_timer = existing.tick_timer.clamp(0.1, 1.0);
        return true;
    }

    buffs.effects.push(StatusEffect {
        effect_type: EffectType::Poison,
        duration_remaining: duration_secs,
        tick_timer: 1.0,
        strength,
    });
    true
}

fn compute_damage(
    base_attack: i32,
    attacker_buffs: Option<&Buffs>,
    target_buffs: Option<&Buffs>,
) -> i32 {
    let attack_bonus = attacker_buffs
        .map(|buffs| {
            buffs
                .effects
                .iter()
                .filter(|effect| effect.effect_type == EffectType::AttackUp)
                .map(|effect| effect.strength.max(0.0) as i32)
                .sum::<i32>()
        })
        .unwrap_or(0);
    let defense_down_bonus = target_buffs
        .map(|buffs| {
            buffs
                .effects
                .iter()
                .filter(|effect| effect.effect_type == EffectType::DefenseDown)
                .map(|effect| effect.strength.max(0.0) as i32)
                .sum::<i32>()
        })
        .unwrap_or(0);
    (base_attack + attack_bonus + defense_down_bonus).max(1)
}

pub fn log_player_death_system(
    mut death_events: MessageReader<CombatDeathEvent>,
    players: Query<&network::NetworkEntity, With<network::PlayerCharacter>>,
) {
    for death in death_events.read() {
        if players.iter().any(|player| player.id == death.target_id) {
            info!("player {} died", death.target_id);
        }
    }
}
