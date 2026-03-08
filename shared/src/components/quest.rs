use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
pub enum QuestId {
    KillSlimes,
}

#[derive(Debug, Clone, PartialEq, Eq, Reflect, Serialize, Deserialize)]
pub enum QuestStatus {
    NotStarted,
    InProgress { count: u32, target: u32 },
    ReadyToTurnIn,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Reflect, Serialize, Deserialize)]
pub struct QuestEntry {
    pub id: QuestId,
    pub status: QuestStatus,
}

#[derive(Component, Debug, Clone, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component, Default)]
pub struct QuestTracker {
    pub active_quests: Vec<QuestEntry>,
}

impl QuestTracker {
    pub fn status_of(&self, quest_id: QuestId) -> QuestStatus {
        self.active_quests
            .iter()
            .find(|entry| entry.id == quest_id)
            .map(|entry| entry.status.clone())
            .unwrap_or(QuestStatus::NotStarted)
    }

    pub fn set_status(&mut self, quest_id: QuestId, status: QuestStatus) {
        if let Some(entry) = self
            .active_quests
            .iter_mut()
            .find(|entry| entry.id == quest_id)
        {
            entry.status = status;
            return;
        }
        self.active_quests.push(QuestEntry {
            id: quest_id,
            status,
        });
    }
}
