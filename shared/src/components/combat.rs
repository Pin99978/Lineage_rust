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

#[derive(
    Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize, Default,
)]
#[reflect(Component, Default)]
pub enum CharacterClass {
    Prince,
    #[default]
    Knight,
    Elf,
    Wizard,
    DarkElf,
}

impl CharacterClass {
    pub fn as_str(self) -> &'static str {
        match self {
            CharacterClass::Prince => "Prince",
            CharacterClass::Knight => "Knight",
            CharacterClass::Elf => "Elf",
            CharacterClass::Wizard => "Wizard",
            CharacterClass::DarkElf => "DarkElf",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "Prince" => Some(CharacterClass::Prince),
            "Knight" => Some(CharacterClass::Knight),
            "Elf" => Some(CharacterClass::Elf),
            "Wizard" => Some(CharacterClass::Wizard),
            "DarkElf" => Some(CharacterClass::DarkElf),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClassDef {
    pub base_str: u32,
    pub base_dex: u32,
    pub base_int: u32,
    pub base_con: u32,
    pub base_hp: i32,
    pub base_mp: i32,
    pub hp_growth_mult: f32,
    pub mp_growth_mult: f32,
    pub can_cast: Vec<SpellType>,
}

pub fn class_def(class: CharacterClass) -> ClassDef {
    match class {
        CharacterClass::Prince => ClassDef {
            base_str: 13,
            base_dex: 10,
            base_int: 10,
            base_con: 10,
            base_hp: 95,
            base_mp: 36,
            hp_growth_mult: 1.0,
            mp_growth_mult: 0.5,
            can_cast: vec![SpellType::Heal],
        },
        CharacterClass::Knight => ClassDef {
            base_str: 16,
            base_dex: 12,
            base_int: 8,
            base_con: 14,
            base_hp: 120,
            base_mp: 22,
            hp_growth_mult: 1.4,
            mp_growth_mult: 0.2,
            can_cast: Vec::new(),
        },
        CharacterClass::Elf => ClassDef {
            base_str: 11,
            base_dex: 12,
            base_int: 12,
            base_con: 12,
            base_hp: 90,
            base_mp: 48,
            hp_growth_mult: 0.8,
            mp_growth_mult: 1.2,
            can_cast: vec![SpellType::Heal],
        },
        CharacterClass::Wizard => ClassDef {
            base_str: 8,
            base_dex: 7,
            base_int: 18,
            base_con: 12,
            base_hp: 76,
            base_mp: 70,
            hp_growth_mult: 0.6,
            mp_growth_mult: 1.5,
            can_cast: vec![SpellType::Fireball, SpellType::Heal],
        },
        CharacterClass::DarkElf => ClassDef {
            base_str: 12,
            base_dex: 15,
            base_int: 11,
            base_con: 8,
            base_hp: 88,
            base_mp: 34,
            hp_growth_mult: 0.9,
            mp_growth_mult: 0.8,
            can_cast: vec![SpellType::Fireball],
        },
    }
}

#[derive(Debug, Clone, Copy, Reflect)]
pub struct SpellDef {
    pub req_level: u32,
    pub mana_cost: i32,
    pub range: f32,
    pub power: i32,
    pub cooldown_secs: f32,
}

pub fn spell_def(spell: SpellType) -> SpellDef {
    match spell {
        SpellType::Fireball => SpellDef {
            req_level: 1,
            mana_cost: 18,
            range: 230.0,
            power: 30,
            cooldown_secs: 1.4,
        },
        SpellType::Heal => SpellDef {
            req_level: 3,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, Default)]
pub enum EffectType {
    #[default]
    Poison,
    Regen,
    SpeedUp,
    AttackUp,
    DefenseDown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct StatusEffect {
    pub effect_type: EffectType,
    pub duration_remaining: f32,
    pub tick_timer: f32,
    pub strength: f32,
}

#[derive(Component, Default, Debug, Clone, Reflect)]
#[reflect(Component, Default)]
pub struct Buffs {
    pub effects: Vec<StatusEffect>,
}

#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Default)]
pub struct BaseStats {
    pub str_stat: u32,
    pub dex: u32,
    pub int_stat: u32,
    pub con: u32,
}

impl Default for BaseStats {
    fn default() -> Self {
        Self {
            str_stat: 15,
            dex: 15,
            int_stat: 15,
            con: 15,
        }
    }
}

#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Default)]
pub struct Level {
    pub current: u32,
}

impl Default for Level {
    fn default() -> Self {
        Self { current: 1 }
    }
}

#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Default)]
pub struct Experience {
    pub current: u32,
    pub next_level_req: u32,
}

impl Default for Experience {
    fn default() -> Self {
        Self {
            current: 0,
            next_level_req: experience_required_for_level(1),
        }
    }
}

pub fn experience_required_for_level(level: u32) -> u32 {
    let lvl = level.max(1) as f32;
    (100.0 * lvl.powf(1.5)).round() as u32
}
