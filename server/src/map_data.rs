use bevy::prelude::*;
use shared::{Npc, NpcMarker, NpcType, Position, SpawnType, Spawner, TargetPosition};

use crate::network;

#[derive(Resource, Debug, Clone)]
pub struct CollisionGrid {
    pub width: i32,
    pub height: i32,
    pub cell_size: f32,
    pub origin: Vec2,
    obstacles: Vec<bool>,
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
    commands.insert_resource(CollisionGrid::demo());

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
        Position { x: 0.0, y: 0.0 },
        TargetPosition { x: 0.0, y: 0.0 },
    ));

    // World spawners.
    let configs = [
        (Position { x: 180.0, y: 120.0 }, 130.0, 3, 2.0),
        (Position { x: 260.0, y: -70.0 }, 140.0, 3, 2.5),
    ];
    for (position, radius, max_count, cooldown_secs) in configs {
        commands.spawn((
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
}
