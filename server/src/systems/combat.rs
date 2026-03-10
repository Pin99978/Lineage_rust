use bevy::prelude::*;
use shared::{
    class_def, experience_required_for_level, ActionState, ArmorClass, BaseStats, Buffs,
    CharacterClass, CombatStats, EffectType, Experience, Health, Level, Mana, Position,
    StatusEffect,
};

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
    pub target_entity: Entity,
    pub target_id: u64,
    pub killer_player_id: Option<u64>,
    pub exp_lost: Option<u32>,
}

#[derive(Message, Debug, Clone)]
pub struct StatusEffectsChangedMessage {
    pub player_id: u64,
    pub effects: Vec<StatusEffect>,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct ExpChangedMessage {
    pub player_id: u64,
    pub level: u32,
    pub exp_current: u32,
    pub exp_next: u32,
    pub str_stat: u32,
    pub dex: u32,
    pub int_stat: u32,
    pub con: u32,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct LevelUpMessage {
    pub player_id: u64,
    pub new_level: u32,
    pub health_max: i32,
    pub mana_max: i32,
}

#[derive(Message, Debug, Clone)]
pub struct SystemNoticeMessage {
    pub player_id: u64,
    pub text: String,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct PlayerDeathPenaltyMessage {
    pub player_id: u64,
    pub exp_lost: u32,
}

pub fn combat_system(
    mut attack_requests: MessageReader<AttackRequest>,
    mut damage_events: MessageWriter<CombatDamageEvent>,
    mut death_events: MessageWriter<CombatDeathEvent>,
    mut attackers: Query<
        (
            &Position,
            &CombatStats,
            &mut ActionState,
            Option<&Buffs>,
            &network::NetworkEntity,
        ),
        With<network::PlayerCharacter>,
    >,
    target_lookup: Query<(Entity, &network::NetworkEntity, &Position, Option<&Buffs>)>,
    mut target_health: Query<&mut Health>,
    target_armor: Query<&ArmorClass>,
) {
    for request in attack_requests.read() {
        let Ok((attacker_position, combat_stats, mut action_state, attacker_buffs, attacker_net)) =
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

        let target_ac = target_armor
            .get(target_entity)
            .map(|armor| armor.value)
            .unwrap_or(0);
        let damage = compute_damage(
            combat_stats.attack_power,
            target_ac,
            attacker_buffs,
            target_buffs,
        );
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
                target_entity,
                target_id: request.target_id,
                killer_player_id: Some(attacker_net.id),
                exp_lost: None,
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

    for (entity, network_entity, mut health, mut buffs, player_marker) in &mut entities {
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
                    target_entity: entity,
                    target_id: network_entity.id,
                    killer_player_id: None,
                    exp_lost: None,
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
    target_ac: i32,
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
    (base_attack + attack_bonus + defense_down_bonus - target_ac).max(1)
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

pub fn experience_and_level_system(
    mut death_events: MessageReader<CombatDeathEvent>,
    entities: Query<&network::NetworkEntity>,
    mut players: Query<
        (
            &network::NetworkEntity,
            &mut Level,
            &mut Experience,
            &BaseStats,
            &CharacterClass,
            &mut Health,
            &mut Mana,
        ),
        With<network::PlayerCharacter>,
    >,
    mut exp_events: MessageWriter<ExpChangedMessage>,
    mut level_up_events: MessageWriter<LevelUpMessage>,
) {
    for death in death_events.read() {
        let Some(killer_player_id) = death.killer_player_id else {
            continue;
        };
        let Ok(dead_net) = entities.get(death.target_entity) else {
            continue;
        };
        if dead_net.kind != shared::protocol::NetworkEntityKind::Enemy {
            continue;
        }

        let mut awarded = false;
        for (player_net, mut level, mut exp, base_stats, class, mut health, mut mana) in
            &mut players
        {
            if player_net.id != killer_player_id {
                continue;
            }

            let reward = 70_u32;
            exp.current = exp.current.saturating_add(reward);
            awarded = true;

            while exp.current >= exp.next_level_req.max(1) {
                exp.current -= exp.next_level_req.max(1);
                level.current = level.current.saturating_add(1);
                exp.next_level_req = experience_required_for_level(level.current);

                let class_growth = class_def(*class);
                let hp_gain = ((roll_growth(base_stats.con, level.current, player_net.id, 17)
                    as f32)
                    * class_growth.hp_growth_mult)
                    .round()
                    .max(1.0) as u32;
                let mp_gain = ((roll_growth(base_stats.int_stat, level.current, player_net.id, 31)
                    as f32)
                    * class_growth.mp_growth_mult)
                    .round()
                    .max(1.0) as u32;
                health.max = health.max.saturating_add(hp_gain as i32);
                mana.max = mana.max.saturating_add(mp_gain as i32);
                health.current = health.max;
                mana.current = mana.max;

                level_up_events.write(LevelUpMessage {
                    player_id: player_net.id,
                    new_level: level.current,
                    health_max: health.max,
                    mana_max: mana.max,
                });
            }

            exp_events.write(ExpChangedMessage {
                player_id: player_net.id,
                level: level.current,
                exp_current: exp.current,
                exp_next: exp.next_level_req,
                str_stat: base_stats.str_stat,
                dex: base_stats.dex,
                int_stat: base_stats.int_stat,
                con: base_stats.con,
            });
            break;
        }

        if !awarded {
            continue;
        }
    }
}

pub fn death_penalty_system(
    mut death_events: MessageReader<CombatDeathEvent>,
    mut players: Query<
        (
            &network::NetworkEntity,
            &mut Level,
            &mut Experience,
            &BaseStats,
            &mut Health,
        ),
        With<network::PlayerCharacter>,
    >,
    mut exp_events: MessageWriter<ExpChangedMessage>,
    mut notice_events: MessageWriter<SystemNoticeMessage>,
    mut penalty_events: MessageWriter<PlayerDeathPenaltyMessage>,
) {
    for death in death_events.read() {
        let Ok((player_net, mut level, mut exp, base_stats, mut health)) =
            players.get_mut(death.target_entity)
        else {
            continue;
        };

        let penalty = experience_required_for_level(level.current) / 10;
        let mut leveled_down = false;

        if exp.current >= penalty {
            exp.current -= penalty;
        } else if level.current > 1 {
            let missing = penalty - exp.current;
            level.current -= 1;
            let prev_req = experience_required_for_level(level.current);
            exp.next_level_req = prev_req.max(1);
            exp.current = prev_req.saturating_sub(missing);
            leveled_down = true;
        } else {
            exp.current = 0;
            exp.next_level_req = experience_required_for_level(1);
        }

        health.current = (health.max / 2).max(1);

        exp_events.write(ExpChangedMessage {
            player_id: player_net.id,
            level: level.current,
            exp_current: exp.current,
            exp_next: exp.next_level_req.max(1),
            str_stat: base_stats.str_stat,
            dex: base_stats.dex,
            int_stat: base_stats.int_stat,
            con: base_stats.con,
        });

        penalty_events.write(PlayerDeathPenaltyMessage {
            player_id: player_net.id,
            exp_lost: penalty,
        });
        notice_events.write(SystemNoticeMessage {
            player_id: player_net.id,
            text: format!("你死了！失去了 {} 點經驗值。", penalty),
        });
        if leveled_down {
            notice_events.write(SystemNoticeMessage {
                player_id: player_net.id,
                text: format!("等級降低至 Lv.{}！", level.current),
            });
        }
    }
}

fn roll_growth(primary_stat: u32, level: u32, player_id: u64, salt: u64) -> u32 {
    let min_gain = (primary_stat / 2).max(1);
    let max_gain = primary_stat.max(min_gain + 1);
    let span = max_gain - min_gain + 1;
    let seed = player_id
        .wrapping_mul(1103515245)
        .wrapping_add((level as u64).wrapping_mul(12345))
        .wrapping_add(salt);
    min_gain + (seed as u32 % span)
}
