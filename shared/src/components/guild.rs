use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect, Serialize, Deserialize, Default)]
pub enum GuildRole {
    Leader,
    #[default]
    Member,
}

impl GuildRole {
    pub fn as_str(self) -> &'static str {
        match self {
            GuildRole::Leader => "Leader",
            GuildRole::Member => "Member",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "Leader" => Some(GuildRole::Leader),
            "Member" => Some(GuildRole::Member),
            _ => None,
        }
    }
}

#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub struct GuildMembership {
    pub guild_name: String,
    pub role: GuildRole,
}
