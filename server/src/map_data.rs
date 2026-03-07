use bevy::prelude::*;
use shared::{Npc, NpcMarker, NpcType, Position, SpawnType, Spawner, TargetPosition};

use crate::network;

pub fn setup_world_map(mut commands: Commands, network: Option<ResMut<network::ServerNetwork>>) {
    let Some(mut network) = network else {
        return;
    };

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
