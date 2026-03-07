use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Default)]
pub struct AggroRange(pub f32);

impl Default for AggroRange {
    fn default() -> Self {
        Self(260.0)
    }
}

#[derive(Component, Debug, Clone, Reflect, Default)]
#[reflect(Component, Default)]
pub enum AiState {
    #[default]
    Idle,
    Chasing(Entity),
    Attacking(Entity),
}

#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Default)]
pub struct AttackCooldown {
    pub remaining_secs: f32,
}

impl Default for AttackCooldown {
    fn default() -> Self {
        Self {
            remaining_secs: 0.0,
        }
    }
}
