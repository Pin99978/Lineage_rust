use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect, Serialize, Deserialize)]
pub enum SpawnType {
    Enemy,
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct Spawner {
    pub spawn_type: SpawnType,
    pub max_count: u32,
    pub radius: f32,
    pub active_entities: Vec<Entity>,
    pub cooldown_secs: f32,
    pub cooldown_remaining: f32,
}

impl Default for Spawner {
    fn default() -> Self {
        Self {
            spawn_type: SpawnType::Enemy,
            max_count: 3,
            radius: 120.0,
            active_entities: Vec::new(),
            cooldown_secs: 2.0,
            cooldown_remaining: 0.0,
        }
    }
}
