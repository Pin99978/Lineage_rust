use bevy::prelude::*;
use shared::protocol::{HealEvent, ManaUpdate};
use shared::{spell_def, CombatStats, Health, Mana, Position, SpellCooldowns, SpellType};

use crate::network;

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

pub fn tick_spell_cooldowns(time: Res<Time>, mut cooldowns: Query<&mut SpellCooldowns>) {
    for mut cooldown in &mut cooldowns {
        let delta = time.delta_secs();
        cooldown.fireball = (cooldown.fireball - delta).max(0.0);
        cooldown.heal = (cooldown.heal - delta).max(0.0);
    }
}

#[allow(clippy::type_complexity)]
pub fn cast_spell_system(
    mut requests: MessageReader<CastSpellRequest>,
    mut casters: Query<
        (
            &network::NetworkEntity,
            &Position,
            &mut Mana,
            &mut SpellCooldowns,
            &CombatStats,
            &mut Health,
        ),
        With<network::PlayerCharacter>,
    >,
    mut targets: Query<(Entity, &network::NetworkEntity, &Position, &mut Health)>,
    mut damage_events: MessageWriter<crate::systems::combat::CombatDamageEvent>,
    mut heal_events: MessageWriter<HealEventMessage>,
    mut mana_events: MessageWriter<ManaChangedMessage>,
) {
    for request in requests.read() {
        let Ok((
            caster_network,
            caster_position,
            mut mana,
            mut cooldowns,
            caster_stats,
            mut self_health,
        )) = casters.get_mut(request.caster_entity)
        else {
            continue;
        };

        let spell = spell_def(request.spell);
        if mana.current < spell.mana_cost {
            continue;
        }
        if !can_cast(request.spell, &cooldowns) {
            continue;
        }

        match request.spell {
            SpellType::Fireball => {
                let Some(target_id) = request.target_id else {
                    continue;
                };
                let Some((target_entity, target_position)) = targets
                    .iter_mut()
                    .find(|(_, network_entity, _, health)| {
                        network_entity.id == target_id && health.current > 0
                    })
                    .map(|(entity, _, position, _)| (entity, *position))
                else {
                    continue;
                };

                let distance = Vec2::new(
                    target_position.x - caster_position.x,
                    target_position.y - caster_position.y,
                )
                .length();
                if distance > spell.range {
                    continue;
                }

                let Ok((_, target_network, _, mut target_health)) = targets.get_mut(target_entity)
                else {
                    continue;
                };
                let damage = (spell.power + caster_stats.attack_power / 4).clamp(1, 999);
                target_health.current = (target_health.current - damage).max(0);

                mana.current = (mana.current - spell.mana_cost).clamp(0, mana.max);
                cooldowns.fireball = spell.cooldown_secs;
                damage_events.write(crate::systems::combat::CombatDamageEvent {
                    target_id: target_network.id,
                    amount: damage,
                    remaining_hp: target_health.current,
                });
            }
            SpellType::Heal => {
                let heal = spell.power.clamp(1, 999);
                let before = self_health.current;
                self_health.current = (self_health.current + heal).clamp(0, self_health.max);
                if self_health.current == before {
                    continue;
                }

                mana.current = (mana.current - spell.mana_cost).clamp(0, mana.max);
                cooldowns.heal = spell.cooldown_secs;
                heal_events.write(HealEventMessage {
                    target_id: caster_network.id,
                    amount: self_health.current - before,
                    resulting_hp: self_health.current,
                });
            }
        }

        mana_events.write(ManaChangedMessage {
            player_id: caster_network.id,
            current: mana.current,
            max: mana.max,
        });
    }
}

fn can_cast(spell: SpellType, cooldowns: &SpellCooldowns) -> bool {
    match spell {
        SpellType::Fireball => cooldowns.fireball <= 0.0,
        SpellType::Heal => cooldowns.heal <= 0.0,
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
