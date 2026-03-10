use bevy::prelude::*;
use shared::protocol::{HealEvent, ManaUpdate, SpellLearnedEvent};
use shared::{
    class_def, spell_def, Buffs, CharacterClass, CombatStats, EffectType, Health, KnownSpells,
    Level, Mana, Position, SpellCooldowns, SpellType, StatusEffect,
};

use crate::{network, systems::combat};

#[derive(Message, Debug, Clone, Copy)]
pub struct CastSpellRequest {
    pub caster_entity: Entity,
    pub spell: SpellType,
    pub target_id: Option<u64>,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct ManaChangedMessage {
    pub player_id: u64,
    pub current: i32,
    pub max: i32,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct HealEventMessage {
    pub target_id: u64,
    pub amount: i32,
    pub resulting_hp: i32,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct SpellLearnedMessage {
    pub player_id: u64,
    pub spell: SpellType,
}

pub fn tick_spell_cooldowns(time: Res<Time>, mut cooldowns: Query<&mut SpellCooldowns>) {
    for mut cooldown in &mut cooldowns {
        let delta = time.delta_secs();
        cooldown.fireball = (cooldown.fireball - delta).max(0.0);
        cooldown.heal = (cooldown.heal - delta).max(0.0);
        cooldown.lightning = (cooldown.lightning - delta).max(0.0);
        cooldown.poison_arrow = (cooldown.poison_arrow - delta).max(0.0);
        cooldown.bless = (cooldown.bless - delta).max(0.0);
    }
}

#[allow(clippy::type_complexity)]
pub fn cast_spell_system(
    mut requests: MessageReader<CastSpellRequest>,
    mut actor_queries: ParamSet<(
        Query<
            (
                &network::NetworkEntity,
                &Position,
                &mut Mana,
                &Level,
                &CharacterClass,
                &KnownSpells,
                &mut SpellCooldowns,
                &CombatStats,
                &mut Health,
                &mut Buffs,
            ),
            With<network::PlayerCharacter>,
        >,
        Query<
            (
                Entity,
                &network::NetworkEntity,
                &Position,
                &mut Health,
                Option<&mut Buffs>,
            ),
            Without<network::PlayerCharacter>,
        >,
    )>,
    mut damage_events: MessageWriter<crate::systems::combat::CombatDamageEvent>,
    mut heal_events: MessageWriter<HealEventMessage>,
    mut mana_events: MessageWriter<ManaChangedMessage>,
    mut status_updates: MessageWriter<combat::StatusEffectsChangedMessage>,
) {
    for request in requests.read() {
        let spell = spell_def(request.spell);
        let (caster_id, caster_position, caster_attack_power, caster_mana_max) = {
            let mut casters = actor_queries.p0();
            let Ok((
                caster_network,
                caster_position,
                mana,
                level,
                player_class,
                known_spells,
                cooldowns,
                caster_stats,
                _self_health,
                _self_buffs,
            )) = casters.get_mut(request.caster_entity)
            else {
                continue;
            };
            if level.current < spell.req_level {
                continue;
            }
            if !class_def(*player_class).can_cast.contains(&request.spell) {
                continue;
            }
            if !known_spells.knows(request.spell) {
                continue;
            }
            if mana.current < spell.mana_cost {
                continue;
            }
            if !can_cast(request.spell, &cooldowns) {
                continue;
            }
            (
                caster_network.id,
                *caster_position,
                caster_stats.attack_power,
                mana.max,
            )
        };

        if request.spell == SpellType::Heal {
            let mut casters = actor_queries.p0();
            let Ok((
                _caster_network,
                _caster_position,
                mut mana,
                _level,
                _player_class,
                _known_spells,
                mut cooldowns,
                _caster_stats,
                mut self_health,
                _self_buffs,
            )) = casters.get_mut(request.caster_entity)
            else {
                continue;
            };
            let heal = spell.power.clamp(1, 999);
            let before = self_health.current;
            self_health.current = (self_health.current + heal).clamp(0, self_health.max);
            if self_health.current == before {
                continue;
            }

            mana.current = (mana.current - spell.mana_cost).clamp(0, mana.max);
            cooldowns.heal = spell.cooldown_secs;
            heal_events.write(HealEventMessage {
                target_id: caster_id,
                amount: self_health.current - before,
                resulting_hp: self_health.current,
            });
            mana_events.write(ManaChangedMessage {
                player_id: caster_id,
                current: mana.current,
                max: caster_mana_max,
            });
            continue;
        }

        if request.spell == SpellType::Bless {
            let mut casters = actor_queries.p0();
            let Ok((
                _caster_network,
                _caster_position,
                mut mana,
                _level,
                _player_class,
                _known_spells,
                mut cooldowns,
                _caster_stats,
                _self_health,
                mut self_buffs,
            )) = casters.get_mut(request.caster_entity)
            else {
                continue;
            };

            if let Some(existing) = self_buffs
                .effects
                .iter_mut()
                .find(|effect| effect.effect_type == EffectType::AttackUp)
            {
                existing.duration_remaining = 30.0;
                existing.strength = existing.strength.max(spell.power as f32);
                existing.tick_timer = 0.0;
            } else {
                self_buffs.effects.push(StatusEffect {
                    effect_type: EffectType::AttackUp,
                    duration_remaining: 30.0,
                    tick_timer: 0.0,
                    strength: spell.power as f32,
                });
            }

            mana.current = (mana.current - spell.mana_cost).clamp(0, mana.max);
            cooldowns.bless = spell.cooldown_secs;
            status_updates.write(combat::StatusEffectsChangedMessage {
                player_id: caster_id,
                effects: self_buffs.effects.clone(),
            });
            mana_events.write(ManaChangedMessage {
                player_id: caster_id,
                current: mana.current,
                max: caster_mana_max,
            });
            continue;
        }

        let Some(target_id) = request.target_id else {
            continue;
        };
        let (target_entity, target_position) = {
            let mut targets = actor_queries.p1();
            let Some((target_entity, target_position)) = targets
                .iter_mut()
                .find(|(_, network_entity, _, health, _)| {
                    network_entity.id == target_id && health.current > 0
                })
                .map(|(entity, _, position, _, _)| (entity, *position))
            else {
                continue;
            };
            (target_entity, target_position)
        };

        let distance = Vec2::new(
            target_position.x - caster_position.x,
            target_position.y - caster_position.y,
        )
        .length();
        if distance > spell.range {
            continue;
        }

        {
            let mut targets = actor_queries.p1();
            let Ok((_, target_network, _, mut target_health, target_buffs)) =
                targets.get_mut(target_entity)
            else {
                continue;
            };
            let damage = match request.spell {
                SpellType::Fireball => (spell.power + caster_attack_power / 4).clamp(1, 999),
                SpellType::Lightning => spell.power.clamp(1, 999),
                SpellType::PoisonArrow => spell.power.clamp(1, 999),
                SpellType::Heal | SpellType::Bless => 0,
            };
            if damage <= 0 {
                continue;
            }
            target_health.current = (target_health.current - damage).max(0);
            damage_events.write(crate::systems::combat::CombatDamageEvent {
                target_id: target_network.id,
                amount: damage,
                remaining_hp: target_health.current,
            });
            if request.spell == SpellType::PoisonArrow {
                if let Some(mut target_buffs) = target_buffs {
                    combat::add_or_refresh_poison(&mut target_buffs, 8.0, 3.0);
                }
            }
        }

        let mut casters = actor_queries.p0();
        let Ok((
            _caster_network,
            _caster_position,
            mut mana,
            _level,
            _player_class,
            _known_spells,
            mut cooldowns,
            _caster_stats,
            _self_health,
            _self_buffs,
        )) = casters.get_mut(request.caster_entity)
        else {
            continue;
        };
        mana.current = (mana.current - spell.mana_cost).clamp(0, mana.max);
        match request.spell {
            SpellType::Fireball => cooldowns.fireball = spell.cooldown_secs,
            SpellType::Lightning => cooldowns.lightning = spell.cooldown_secs,
            SpellType::PoisonArrow => cooldowns.poison_arrow = spell.cooldown_secs,
            SpellType::Heal | SpellType::Bless => {}
        }
        mana_events.write(ManaChangedMessage {
            player_id: caster_id,
            current: mana.current,
            max: caster_mana_max,
        });
    }
}

fn can_cast(spell: SpellType, cooldowns: &SpellCooldowns) -> bool {
    match spell {
        SpellType::Fireball => cooldowns.fireball <= 0.0,
        SpellType::Heal => cooldowns.heal <= 0.0,
        SpellType::Lightning => cooldowns.lightning <= 0.0,
        SpellType::PoisonArrow => cooldowns.poison_arrow <= 0.0,
        SpellType::Bless => cooldowns.bless <= 0.0,
    }
}

pub fn to_mana_update(message: ManaChangedMessage) -> ManaUpdate {
    ManaUpdate {
        player_id: message.player_id,
        current: message.current,
        max: message.max,
    }
}

pub fn to_heal_event(message: HealEventMessage) -> HealEvent {
    HealEvent {
        target_id: message.target_id,
        amount: message.amount,
        resulting_hp: message.resulting_hp,
    }
}

pub fn to_spell_learned_event(message: SpellLearnedMessage) -> SpellLearnedEvent {
    SpellLearnedEvent {
        player_id: message.player_id,
        spell: message.spell,
    }
}
