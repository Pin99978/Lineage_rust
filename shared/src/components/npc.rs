use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect, Serialize, Deserialize)]
pub enum NpcType {
    Merchant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect, Serialize, Deserialize)]
pub enum DialogueChoiceKind {
    AcceptQuestKillSlimes,
    TurnInQuestKillSlimes,
    Leave,
}

#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub struct Npc {
    pub npc_type: NpcType,
    pub dialog: String,
}

#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component, Default)]
pub struct NpcMarker;
