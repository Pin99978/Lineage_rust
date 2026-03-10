use bevy::prelude::*;
use shared::{
    AggroRange, AttackCooldown, Buffs, CombatStats, Health, MapId, MoveSpeed, Npc, NpcMarker,
    NpcType, Portal, Position, SpawnType, Spawner, TargetPosition, MAP_DUNGEON_1, MAP_TOWN,
};
use std::collections::HashMap;

use crate::{
    network,
    systems::ai::{GuardAi, GuardRespawnTimer},
};

#[derive(Resource, Debug, Clone)]
pub struct CollisionGrid {
    pub width: i32,
    pub height: i32,
    pub cell_size: f32,
    pub origin: Vec2,
    obstacles: Vec<bool>,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct MapManager {
    pub grids: HashMap<String, CollisionGrid>,
}

impl MapManager {
    pub fn insert(&mut self, map_id: impl Into<String>, grid: CollisionGrid) {
        self.grids.insert(map_id.into(), grid);
    }

    pub fn grid_for(&self, map_id: &MapId) -> Option<&CollisionGrid> {
        self.grids.get(&map_id.0)
    }
}

impl CollisionGrid {
    pub fn demo() -> Self {
        let width = 100;
        let height = 100;
        let cell_size = 20.0;
        let origin = Vec2::new(
            -(width as f32) * 0.5 * cell_size,
            -(height as f32) * 0.5 * cell_size,
        );
        let mut grid = Self {
            width,
            height,
            cell_size,
            origin,
            obstacles: vec![false; (width * height) as usize],
        };

        for y in 36..=64 {
            grid.set_blocked(50, y, true);
        }
        for x in 46..=54 {
            grid.set_blocked(x, 50, true);
        }
        for x in 18..=28 {
            for y in 72..=80 {
                grid.set_blocked(x, y, true);
            }
        }
        grid
    }

    pub fn dungeon_demo() -> Self {
        let mut grid = Self::demo();
        for x in 12..=88 {
            grid.set_blocked(x, 18, true);
            grid.set_blocked(x, 82, true);
        }
        for y in 18..=82 {
            grid.set_blocked(12, y, true);
            grid.set_blocked(88, y, true);
        }
        for y in 30..=70 {
            grid.set_blocked(44, y, true);
            grid.set_blocked(56, y, true);
        }
        grid
    }

    pub fn in_bounds(&self, cell: IVec2) -> bool {
        cell.x >= 0 && cell.x < self.width && cell.y >= 0 && cell.y < self.height
    }

    pub fn is_cell_blocked(&self, cell: IVec2) -> bool {
        if !self.in_bounds(cell) {
            return true;
        }
        self.obstacles[self.index(cell)]
    }

    pub fn world_to_cell(&self, world: Vec2) -> Option<IVec2> {
        let local = world - self.origin;
        let x = (local.x / self.cell_size).floor() as i32;
        let y = (local.y / self.cell_size).floor() as i32;
        let cell = IVec2::new(x, y);
        self.in_bounds(cell).then_some(cell)
    }

    pub fn cell_to_world_center(&self, cell: IVec2) -> Vec2 {
        Vec2::new(
            self.origin.x + (cell.x as f32 + 0.5) * self.cell_size,
            self.origin.y + (cell.y as f32 + 0.5) * self.cell_size,
        )
    }

    pub fn nearest_walkable(&self, target: IVec2) -> Option<IVec2> {
        if self.in_bounds(target) && !self.is_cell_blocked(target) {
            return Some(target);
        }

        let max_radius = self.width.max(self.height);
        for radius in 1..=max_radius {
            let mut best: Option<(IVec2, i32)> = None;
            for y in (target.y - radius)..=(target.y + radius) {
                for x in (target.x - radius)..=(target.x + radius) {
                    let candidate = IVec2::new(x, y);
                    if !self.in_bounds(candidate) || self.is_cell_blocked(candidate) {
                        continue;
                    }
                    let dist2 = (candidate.x - target.x).pow(2) + (candidate.y - target.y).pow(2);
                    let should_take = best.map(|(_, best_dist)| dist2 < best_dist).unwrap_or(true);
                    if should_take {
                        best = Some((candidate, dist2));
                    }
                }
            }
            if let Some((cell, _)) = best {
                return Some(cell);
            }
        }
        None
    }

    fn set_blocked(&mut self, x: i32, y: i32, blocked: bool) {
        let cell = IVec2::new(x, y);
        if self.in_bounds(cell) {
            let index = self.index(cell);
            self.obstacles[index] = blocked;
        }
    }

    fn index(&self, cell: IVec2) -> usize {
        (cell.y * self.width + cell.x) as usize
    }
}

pub fn setup_world_map(mut commands: Commands, network: Option<ResMut<network::ServerNetwork>>) {
    let Some(mut network) = network else {
        return;
    };
    let mut maps = MapManager::default();
    maps.insert(MAP_TOWN, CollisionGrid::demo());
    maps.insert(MAP_DUNGEON_1, CollisionGrid::dungeon_demo());
    commands.insert_resource(maps);

    // Merchant NPC at town center.
    let npc_id = network.allocate_entity_id();
    commands.spawn((
        NpcMarker,
        Npc {
            npc_type: NpcType::Merchant,
            dialog: "Welcome to Talking Island!".to_string(),
        },
        network::NetworkEntity {
            id: npc_id,
            kind: shared::protocol::NetworkEntityKind::NpcMerchant,
        },
        MapId(MAP_TOWN.to_string()),
        Position { x: 0.0, y: 0.0 },
        TargetPosition { x: 0.0, y: 0.0 },
    ));

    // Town guards that hunt chaotic players.
    let guard_positions = [(100.0, 150.0), (-100.0, 150.0), (0.0, -200.0)];
    for (x, y) in guard_positions {
        let guard_id = network.allocate_entity_id();
        commands.spawn((
            GuardAi,
            GuardRespawnTimer::new(Vec2::new(x, y)),
            network::NetworkEntity {
                id: guard_id,
                kind: shared::protocol::NetworkEntityKind::Enemy,
            },
            MapId(MAP_TOWN.to_string()),
            Position { x, y },
            TargetPosition { x, y },
            MoveSpeed { value: 260.0 },
            Health {
                current: 300,
                max: 300,
            },
            Buffs::default(),
            CombatStats {
                attack_power: 40,
                attack_range: 80.0,
                attack_speed: 1.5,
            },
            AggroRange(600.0),
            AttackCooldown::default(),
        ));
    }

    // World spawners (Town + Dungeon1).
    let configs = [
        (MAP_TOWN, Position { x: 180.0, y: 120.0 }, 130.0, 3, 2.0),
        (MAP_TOWN, Position { x: 260.0, y: -70.0 }, 140.0, 3, 2.5),
        (
            MAP_DUNGEON_1,
            Position { x: -120.0, y: 80.0 },
            110.0,
            4,
            1.8,
        ),
    ];
    for (map_id, position, radius, max_count, cooldown_secs) in configs {
        commands.spawn((
            MapId(map_id.to_string()),
            position,
            Spawner {
                spawn_type: SpawnType::Enemy,
                max_count,
                radius,
                active_entities: Vec::new(),
                cooldown_secs,
                cooldown_remaining: 0.0,
            },
        ));
    }

    // Two-way portals between Town and Dungeon1.
    let town_portal_id = network.allocate_entity_id();
    commands.spawn((
        network::NetworkEntity {
            id: town_portal_id,
            kind: shared::protocol::NetworkEntityKind::Portal,
        },
        MapId(MAP_TOWN.to_string()),
        Position { x: -250.0, y: 0.0 },
        Portal {
            target_map: MAP_DUNGEON_1.to_string(),
            target_x: -300.0,
            target_y: 0.0,
            trigger_radius: 28.0,
        },
    ));
    let dungeon_portal_id = network.allocate_entity_id();
    commands.spawn((
        network::NetworkEntity {
            id: dungeon_portal_id,
            kind: shared::protocol::NetworkEntityKind::Portal,
        },
        MapId(MAP_DUNGEON_1.to_string()),
        Position { x: -350.0, y: 0.0 },
        Portal {
            target_map: MAP_TOWN.to_string(),
            target_x: -300.0,
            target_y: 0.0,
            trigger_radius: 28.0,
        },
    ));
}
