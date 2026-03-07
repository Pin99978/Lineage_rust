use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Default)]
pub struct Health {
    pub current: i32,
    pub max: i32,
}

impl Default for Health {
    fn default() -> Self {
        Self {
            current: 100,
            max: 100,
        }
    }
}

#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Default)]
pub struct CombatStats {
    pub attack_power: i32,
    pub attack_range: f32,
    pub attack_speed: f32,
}

impl Default for CombatStats {
    fn default() -> Self {
        Self {
            attack_power: 10,
            attack_range: 120.0,
            attack_speed: 1.0,
        }
    }
}

#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component, Default)]
pub struct ActionState {
    pub is_attacking: bool,
    pub target: Option<Entity>,
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Default)]
pub struct Mana {
    pub current: i32,
    pub max: i32,
}

impl Default for Mana {
    fn default() -> Self {
        Self {
            current: 60,
            max: 60,
        }
    }
}

#[derive(Component, Debug, Clone, Copy, Reflect, Default)]
#[reflect(Component, Default)]
pub struct ArmorClass {
    pub value: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize, Default)]
pub enum SpellType {
    #[default]
    Fireball,
    Heal,
}

#[derive(Debug, Clone, Copy, Reflect)]
pub struct SpellDef {
    pub mana_cost: i32,
    pub range: f32,
    pub power: i32,
    pub cooldown_secs: f32,
}

pub fn spell_def(spell: SpellType) -> SpellDef {
    match spell {
        SpellType::Fireball => SpellDef {
            mana_cost: 18,
            range: 230.0,
            power: 30,
            cooldown_secs: 1.4,
        },
        SpellType::Heal => SpellDef {
            mana_cost: 16,
            range: 0.0,
            power: 26,
            cooldown_secs: 2.0,
        },
    }
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Default)]
pub struct SpellCooldowns {
    pub fireball: f32,
    pub heal: f32,
}

impl Default for SpellCooldowns {
    fn default() -> Self {
        Self {
            fireball: 0.0,
            heal: 0.0,
        }
    }
}
