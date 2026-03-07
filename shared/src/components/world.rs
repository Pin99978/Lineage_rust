use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const MAP_TOWN: &str = "Town";
pub const MAP_DUNGEON_1: &str = "Dungeon1";

#[derive(Component, Debug, Clone, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub struct MapId(pub String);

impl Default for MapId {
    fn default() -> Self {
        Self(MAP_TOWN.to_string())
    }
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct Portal {
    pub target_map: String,
    pub target_x: f32,
    pub target_y: f32,
    pub trigger_radius: f32,
}

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
