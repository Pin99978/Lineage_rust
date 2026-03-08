use bevy::prelude::*;
use shared::{QuestId, QuestStatus, QuestTracker};

use crate::{network, systems::combat};

pub const KILL_SLIMES_TARGET: u32 = 3;

#[derive(Message, Debug, Clone)]
pub struct QuestUpdatedMessage {
    pub player_id: u64,
    pub quest_id: QuestId,
    pub status: QuestStatus,
}

pub fn track_enemy_kill_quest_system(
    mut death_events: MessageReader<combat::CombatDeathEvent>,
    entities: Query<&network::NetworkEntity>,
    mut players: Query<
        (&network::NetworkEntity, &mut QuestTracker),
        With<network::PlayerCharacter>,
    >,
    mut updates: MessageWriter<QuestUpdatedMessage>,
) {
    for death in death_events.read() {
        let Some(killer_player_id) = death.killer_player_id else {
            continue;
        };
        let Ok(dead_entity) = entities.get(death.target_entity) else {
            continue;
        };
        if dead_entity.kind != shared::protocol::NetworkEntityKind::Enemy {
            continue;
        }

        for (player_network, mut tracker) in &mut players {
            if player_network.id != killer_player_id {
                continue;
            }

            let next_status = match tracker.status_of(QuestId::KillSlimes) {
                QuestStatus::InProgress { count, target } => {
                    let next = count.saturating_add(1).min(target);
                    if next >= target {
                        QuestStatus::ReadyToTurnIn
                    } else {
                        QuestStatus::InProgress {
                            count: next,
                            target,
                        }
                    }
                }
                _ => continue,
            };

            tracker.set_status(QuestId::KillSlimes, next_status.clone());
            updates.write(QuestUpdatedMessage {
                player_id: player_network.id,
                quest_id: QuestId::KillSlimes,
                status: next_status,
            });
        }
    }
}
