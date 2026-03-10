use bevy::prelude::*;
use shared::{GuildMembership, GuildRole, Inventory, ItemType};
use std::collections::HashMap;

use crate::{db, network, systems::loot};

const GUILD_CREATE_GOLD_COST: u32 = 1_000;

#[derive(Resource, Default)]
pub struct PendingGuildInvites {
    by_player_id: HashMap<u64, PendingInvite>,
}

#[derive(Debug, Clone)]
struct PendingInvite {
    from_username: String,
    guild_name: String,
}

#[derive(Message, Debug, Clone)]
pub struct CreateGuildRequest {
    pub player_entity: Entity,
    pub guild_name: String,
}

#[derive(Message, Debug, Clone)]
pub struct InviteToGuildRequest {
    pub player_entity: Entity,
    pub target_username: String,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct RespondToGuildInviteRequest {
    pub player_entity: Entity,
    pub accepted: bool,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct LeaveGuildRequest {
    pub player_entity: Entity,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct DisbandGuildRequest {
    pub player_entity: Entity,
}

#[derive(Message, Debug, Clone)]
pub struct GuildUpdateMessage {
    pub player_id: u64,
    pub guild_name: Option<String>,
    pub role: Option<GuildRole>,
    pub member_usernames: Vec<String>,
}

#[derive(Message, Debug, Clone)]
pub struct GuildInviteMessage {
    pub player_id: u64,
    pub from_username: String,
    pub guild_name: String,
}

#[derive(Message, Debug, Clone)]
pub struct GuildActionErrorMessage {
    pub player_id: u64,
    pub message: String,
}

pub fn setup_guild_system(mut commands: Commands) {
    commands.insert_resource(PendingGuildInvites::default());
}

pub fn handle_create_guild(
    mut requests: MessageReader<CreateGuildRequest>,
    db_bridge: Option<Res<db::DbBridge>>,
    network: Option<Res<network::ServerNetwork>>,
    mut players: Query<(
        &network::NetworkEntity,
        &mut Inventory,
        Option<&GuildMembership>,
    )>,
    mut inventory_updates: MessageWriter<loot::InventoryUpdateMessage>,
    mut errors: MessageWriter<GuildActionErrorMessage>,
) {
    let Some(db_bridge) = db_bridge else {
        return;
    };
    let Some(network) = network else {
        return;
    };

    for request in requests.read() {
        let Some((username, player_id)) =
            username_and_id_for_entity(&network, request.player_entity)
        else {
            continue;
        };

        let Ok((player_net, mut inventory, membership)) = players.get_mut(request.player_entity)
        else {
            continue;
        };
        if membership.is_some() {
            errors.write(GuildActionErrorMessage {
                player_id,
                message: "already in a guild".to_string(),
            });
            continue;
        }

        let trimmed_name = request.guild_name.trim();
        if trimmed_name.len() < 3 || trimmed_name.len() > 24 {
            errors.write(GuildActionErrorMessage {
                player_id,
                message: "guild name must be 3-24 chars".to_string(),
            });
            continue;
        }

        let gold = inventory.items.get(&ItemType::Gold).copied().unwrap_or(0);
        if gold < GUILD_CREATE_GOLD_COST {
            errors.write(GuildActionErrorMessage {
                player_id,
                message: format!("need {} gold to create guild", GUILD_CREATE_GOLD_COST),
            });
            continue;
        }

        let remaining = gold - GUILD_CREATE_GOLD_COST;
        if remaining == 0 {
            inventory.items.remove(&ItemType::Gold);
        } else {
            inventory.items.insert(ItemType::Gold, remaining);
        }
        inventory_updates.write(loot::InventoryUpdateMessage {
            player_id: player_net.id,
            item_type: ItemType::Gold,
            amount: remaining,
        });

        let _ = db_bridge.command_tx.send(db::DbCommand::CreateGuild {
            username,
            guild_name: trimmed_name.to_string(),
        });
    }
}

pub fn handle_invite_to_guild(
    mut requests: MessageReader<InviteToGuildRequest>,
    network: Option<Res<network::ServerNetwork>>,
    players: Query<(&network::NetworkEntity, Option<&GuildMembership>)>,
    invites: Option<ResMut<PendingGuildInvites>>,
    mut invite_messages: MessageWriter<GuildInviteMessage>,
    mut errors: MessageWriter<GuildActionErrorMessage>,
) {
    let Some(network) = network else {
        return;
    };
    let Some(mut invites) = invites else {
        return;
    };

    for request in requests.read() {
        let Some((from_username, from_player_id)) =
            username_and_id_for_entity(&network, request.player_entity)
        else {
            continue;
        };
        let Ok((_, inviter_membership)) = players.get(request.player_entity) else {
            continue;
        };
        let Some(inviter_membership) = inviter_membership else {
            errors.write(GuildActionErrorMessage {
                player_id: from_player_id,
                message: "you are not in a guild".to_string(),
            });
            continue;
        };
        if inviter_membership.role != GuildRole::Leader {
            errors.write(GuildActionErrorMessage {
                player_id: from_player_id,
                message: "only leader can invite".to_string(),
            });
            continue;
        }

        let target_name = request.target_username.trim();
        let Some((target_entity, target_player_id)) =
            find_entity_and_player_id_by_username(&network, target_name)
        else {
            errors.write(GuildActionErrorMessage {
                player_id: from_player_id,
                message: "target is not online".to_string(),
            });
            continue;
        };
        let Ok((_, target_membership)) = players.get(target_entity) else {
            continue;
        };
        if target_membership.is_some() {
            errors.write(GuildActionErrorMessage {
                player_id: from_player_id,
                message: "target is already in a guild".to_string(),
            });
            continue;
        }

        invites.by_player_id.insert(
            target_player_id,
            PendingInvite {
                from_username: from_username.clone(),
                guild_name: inviter_membership.guild_name.clone(),
            },
        );
        invite_messages.write(GuildInviteMessage {
            player_id: target_player_id,
            from_username,
            guild_name: inviter_membership.guild_name.clone(),
        });
    }
}

pub fn handle_respond_to_guild_invite(
    mut requests: MessageReader<RespondToGuildInviteRequest>,
    db_bridge: Option<Res<db::DbBridge>>,
    network: Option<Res<network::ServerNetwork>>,
    players: Query<Option<&GuildMembership>, With<network::PlayerCharacter>>,
    invites: Option<ResMut<PendingGuildInvites>>,
    mut errors: MessageWriter<GuildActionErrorMessage>,
) {
    let Some(db_bridge) = db_bridge else {
        return;
    };
    let Some(network) = network else {
        return;
    };
    let Some(mut invites) = invites else {
        return;
    };

    for request in requests.read() {
        let Some((username, player_id)) =
            username_and_id_for_entity(&network, request.player_entity)
        else {
            continue;
        };
        let Ok(membership) = players.get(request.player_entity) else {
            continue;
        };
        if membership.is_some() {
            errors.write(GuildActionErrorMessage {
                player_id,
                message: "already in a guild".to_string(),
            });
            continue;
        }

        let Some(invite) = invites.by_player_id.remove(&player_id) else {
            errors.write(GuildActionErrorMessage {
                player_id,
                message: "no pending guild invite".to_string(),
            });
            continue;
        };
        if !request.accepted {
            continue;
        }

        let _ = db_bridge.command_tx.send(db::DbCommand::JoinGuild {
            username,
            guild_name: invite.guild_name,
            role: GuildRole::Member,
        });
    }
}

pub fn handle_leave_guild(
    mut requests: MessageReader<LeaveGuildRequest>,
    db_bridge: Option<Res<db::DbBridge>>,
    network: Option<Res<network::ServerNetwork>>,
    players: Query<Option<&GuildMembership>, With<network::PlayerCharacter>>,
    mut errors: MessageWriter<GuildActionErrorMessage>,
) {
    let Some(db_bridge) = db_bridge else {
        return;
    };
    let Some(network) = network else {
        return;
    };

    for request in requests.read() {
        let Some((username, player_id)) =
            username_and_id_for_entity(&network, request.player_entity)
        else {
            continue;
        };
        let Ok(membership) = players.get(request.player_entity) else {
            continue;
        };
        let Some(membership) = membership else {
            errors.write(GuildActionErrorMessage {
                player_id,
                message: "not in a guild".to_string(),
            });
            continue;
        };
        if membership.role == GuildRole::Leader {
            errors.write(GuildActionErrorMessage {
                player_id,
                message: "leader must disband guild".to_string(),
            });
            continue;
        }

        let _ = db_bridge
            .command_tx
            .send(db::DbCommand::LeaveGuild { username });
    }
}

pub fn handle_disband_guild(
    mut requests: MessageReader<DisbandGuildRequest>,
    db_bridge: Option<Res<db::DbBridge>>,
    network: Option<Res<network::ServerNetwork>>,
    players: Query<Option<&GuildMembership>, With<network::PlayerCharacter>>,
    mut errors: MessageWriter<GuildActionErrorMessage>,
) {
    let Some(db_bridge) = db_bridge else {
        return;
    };
    let Some(network) = network else {
        return;
    };

    for request in requests.read() {
        let Some((username, player_id)) =
            username_and_id_for_entity(&network, request.player_entity)
        else {
            continue;
        };
        let Ok(membership) = players.get(request.player_entity) else {
            continue;
        };
        let Some(membership) = membership else {
            errors.write(GuildActionErrorMessage {
                player_id,
                message: "not in a guild".to_string(),
            });
            continue;
        };
        if membership.role != GuildRole::Leader {
            errors.write(GuildActionErrorMessage {
                player_id,
                message: "only leader can disband".to_string(),
            });
            continue;
        }

        let _ = db_bridge
            .command_tx
            .send(db::DbCommand::DisbandGuild { username });
    }
}

fn username_and_id_for_entity(
    network: &network::ServerNetwork,
    entity: Entity,
) -> Option<(String, u64)> {
    network.sessions.values().find_map(|session| {
        match (
            session.logged_in,
            session.entity,
            session.username.as_ref(),
            session.player_id,
        ) {
            (true, Some(player_entity), Some(username), Some(player_id))
                if player_entity == entity =>
            {
                Some((username.clone(), player_id))
            }
            _ => None,
        }
    })
}

fn find_entity_and_player_id_by_username(
    network: &network::ServerNetwork,
    username: &str,
) -> Option<(Entity, u64)> {
    network.sessions.values().find_map(|session| {
        match (
            session.logged_in,
            session.entity,
            session.username.as_ref(),
            session.player_id,
        ) {
            (true, Some(entity), Some(name), Some(player_id))
                if name.eq_ignore_ascii_case(username) =>
            {
                Some((entity, player_id))
            }
            _ => None,
        }
    })
}
