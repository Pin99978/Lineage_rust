use bevy::prelude::*;

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
