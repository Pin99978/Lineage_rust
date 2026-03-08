use bevy::prelude::*;
use shared::{MapId, PathQueue, Portal, Position, TargetPosition};

use crate::network;

#[derive(Message, Debug, Clone, Copy)]
pub struct InteractRequest {
    pub player_entity: Entity,
    pub target_id: u64,
}

#[derive(Message, Debug, Clone)]
pub struct DialogMessage {
    pub player_id: u64,
    pub text: String,
}

#[derive(Message, Debug, Clone)]
pub struct MapChangedMessage {
    pub player_id: u64,
    pub map_id: String,
    pub x: f32,
    pub y: f32,
}

pub fn interaction_system(
    mut requests: MessageReader<InteractRequest>,
    players: Query<(&network::NetworkEntity, &Position, &MapId), With<network::PlayerCharacter>>,
    npcs: Query<
        (&network::NetworkEntity, &Position, &MapId, &shared::Npc),
        With<shared::NpcMarker>,
    >,
    mut dialogs: MessageWriter<DialogMessage>,
) {
    for request in requests.read() {
        let Ok((player_network, player_position, player_map)) = players.get(request.player_entity)
        else {
            continue;
        };
        let Some((_, npc_position, npc_map, npc)) = npcs
            .iter()
            .find(|(network_entity, _, _, _)| network_entity.id == request.target_id)
        else {
            continue;
        };
        if npc_map.0 != player_map.0 {
            continue;
        }

        let distance = Vec2::new(
            npc_position.x - player_position.x,
            npc_position.y - player_position.y,
        )
        .length();
        if distance > 120.0 {
            continue;
        }

        dialogs.write(DialogMessage {
            player_id: player_network.id,
            text: npc.dialog.clone(),
        });
    }
}

pub fn portal_system(
    mut players: Query<
        (
            &network::NetworkEntity,
            &mut Position,
            &mut TargetPosition,
            &mut MapId,
            &mut PathQueue,
        ),
        (With<network::PlayerCharacter>, Without<Portal>),
    >,
    portals: Query<(&Position, &MapId, &Portal), Without<network::PlayerCharacter>>,
    mut map_changed: MessageWriter<MapChangedMessage>,
) {
    for (player_network, mut position, mut target, mut player_map, mut path_queue) in &mut players {
        let Some((_, _, portal)) = portals
            .iter()
            .find(|(portal_position, portal_map, portal)| {
                if portal_map.0 != player_map.0 {
                    return false;
                }
                let distance = Vec2::new(
                    portal_position.x - position.x,
                    portal_position.y - position.y,
                )
                .length();
                distance <= portal.trigger_radius.max(2.0)
            })
        else {
            continue;
        };

        player_map.0 = portal.target_map.clone();
        position.x = portal.target_x;
        position.y = portal.target_y;
        target.x = portal.target_x;
        target.y = portal.target_y;
        path_queue.waypoints.clear();

        map_changed.write(MapChangedMessage {
            player_id: player_network.id,
            map_id: portal.target_map.clone(),
            x: portal.target_x,
            y: portal.target_y,
        });
    }
}
