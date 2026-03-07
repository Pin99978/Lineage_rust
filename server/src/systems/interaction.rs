use bevy::prelude::*;

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

pub fn interaction_system(
    mut requests: MessageReader<InteractRequest>,
    players: Query<(&network::NetworkEntity, &shared::Position), With<network::PlayerCharacter>>,
    npcs: Query<
        (&network::NetworkEntity, &shared::Position, &shared::Npc),
        With<shared::NpcMarker>,
    >,
    mut dialogs: MessageWriter<DialogMessage>,
) {
    for request in requests.read() {
        let Ok((player_network, player_position)) = players.get(request.player_entity) else {
            continue;
        };
        let Some((_, npc_position, npc)) = npcs
            .iter()
            .find(|(network_entity, _, _)| network_entity.id == request.target_id)
        else {
            continue;
        };

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

pub fn to_dialog_event(message: &DialogMessage) -> shared::protocol::DialogEvent {
    shared::protocol::DialogEvent {
        player_id: message.player_id,
        text: message.text.clone(),
    }
}
