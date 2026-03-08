use bevy::prelude::*;
use shared::{Npc, NpcMarker, NpcType, Position, QuestId, QuestStatus, QuestTracker};

use crate::{
    network,
    systems::{interaction, quest},
};

#[derive(Message, Debug, Clone, Copy)]
pub struct NpcInteractRequest {
    pub player_entity: Entity,
    pub target_id: u64,
    pub choice_index: Option<u8>,
}

#[derive(Message, Debug, Clone)]
pub struct DialogueMessage {
    pub player_id: u64,
    pub npc_id: u64,
    pub text: String,
    pub choices: Vec<String>,
}

pub fn npc_dialogue_system(
    mut requests: MessageReader<NpcInteractRequest>,
    mut players: Query<
        (
            &network::NetworkEntity,
            &Position,
            &shared::MapId,
            &mut QuestTracker,
        ),
        With<network::PlayerCharacter>,
    >,
    npcs: Query<(&network::NetworkEntity, &Position, &shared::MapId, &Npc), With<NpcMarker>>,
    mut dialogues: MessageWriter<DialogueMessage>,
    mut quest_updates: MessageWriter<quest::QuestUpdatedMessage>,
) {
    for request in requests.read() {
        let Ok((player_network, player_position, player_map, mut tracker)) =
            players.get_mut(request.player_entity)
        else {
            continue;
        };
        let Some((npc_network, npc_position, npc_map, npc)) = npcs
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

        if !matches!(npc.npc_type, NpcType::Merchant) {
            continue;
        }

        if let Some(choice) = request.choice_index {
            handle_choice(
                choice,
                player_network.id,
                npc_network.id,
                &mut tracker,
                &mut dialogues,
                &mut quest_updates,
            );
            continue;
        }

        let status = tracker.status_of(QuestId::KillSlimes);
        let (text, choices) = match status {
            QuestStatus::NotStarted => (
                "Need help? Slay 3 slimes for me.".to_string(),
                vec![
                    "Accept Quest: Kill Slimes".to_string(),
                    "Maybe later".to_string(),
                ],
            ),
            QuestStatus::InProgress { count, target } => (
                format!("Progress: [{}/{}] Kill Slimes", count, target),
                vec!["Keep going".to_string()],
            ),
            QuestStatus::ReadyToTurnIn => (
                "Great work. Ready to turn in?".to_string(),
                vec!["Turn in quest".to_string(), "Not yet".to_string()],
            ),
            QuestStatus::Completed => (
                "Thanks again. Come back later for more work.".to_string(),
                vec!["Bye".to_string()],
            ),
        };

        dialogues.write(DialogueMessage {
            player_id: player_network.id,
            npc_id: npc_network.id,
            text,
            choices,
        });
    }
}

fn handle_choice(
    choice: u8,
    player_id: u64,
    npc_id: u64,
    tracker: &mut QuestTracker,
    dialogues: &mut MessageWriter<DialogueMessage>,
    quest_updates: &mut MessageWriter<quest::QuestUpdatedMessage>,
) {
    let status = tracker.status_of(QuestId::KillSlimes);
    let (text, maybe_status) = match (status, choice) {
        (QuestStatus::NotStarted, 1) => (
            "Quest accepted: Kill 3 slimes.".to_string(),
            Some(QuestStatus::InProgress {
                count: 0,
                target: quest::KILL_SLIMES_TARGET,
            }),
        ),
        (QuestStatus::ReadyToTurnIn, 1) => (
            "Quest completed. Reward will be available in next task.".to_string(),
            Some(QuestStatus::Completed),
        ),
        _ => ("Understood.".to_string(), None),
    };

    if let Some(next) = maybe_status {
        tracker.set_status(QuestId::KillSlimes, next.clone());
        quest_updates.write(quest::QuestUpdatedMessage {
            player_id,
            quest_id: QuestId::KillSlimes,
            status: next,
        });
    }

    dialogues.write(DialogueMessage {
        player_id,
        npc_id,
        text,
        choices: Vec::new(),
    });
}

pub fn to_dialogue_response(message: &DialogueMessage) -> shared::protocol::DialogueResponse {
    shared::protocol::DialogueResponse {
        player_id: message.player_id,
        npc_id: message.npc_id,
        text: message.text.clone(),
        choices: message.choices.clone(),
    }
}

pub fn convert_legacy_dialog_to_npc(
    mut old_dialogs: MessageReader<interaction::DialogMessage>,
    mut dialogues: MessageWriter<DialogueMessage>,
) {
    for dialog in old_dialogs.read() {
        dialogues.write(DialogueMessage {
            player_id: dialog.player_id,
            npc_id: 0,
            text: dialog.text.clone(),
            choices: Vec::new(),
        });
    }
}
