use bevy::prelude::*;
use shared::protocol::{ChatChannel, ChatEvent};
use shared::{GuildMembership, Position};

use crate::network;

const SAY_RADIUS: f32 = 30.0;
const MAX_CHAT_MESSAGE_LEN: usize = 160;

#[derive(Message, Debug, Clone)]
pub struct ChatRequest {
    pub player_entity: Entity,
    pub channel: ChatChannel,
    pub target: Option<String>,
    pub message: String,
}

#[derive(Message, Debug, Clone)]
pub struct ChatDelivery {
    pub recipient_player_id: u64,
    pub event: ChatEvent,
}

#[derive(Clone)]
struct OnlinePlayer {
    player_entity: Entity,
    player_id: u64,
    username: String,
    position: Vec2,
    guild_name: Option<String>,
}

pub fn chat_system(
    network: Option<Res<network::ServerNetwork>>,
    players: Query<
        (&Position, &network::NetworkEntity, Option<&GuildMembership>),
        With<network::PlayerCharacter>,
    >,
    mut requests: MessageReader<ChatRequest>,
    mut deliveries: MessageWriter<ChatDelivery>,
) {
    let Some(network) = network else {
        return;
    };

    let online_players = collect_online_players(&network, &players);
    for request in requests.read() {
        let sender = online_players
            .iter()
            .find(|player| player.player_entity == request.player_entity);
        let Some(sender) = sender else {
            continue;
        };

        let message = request.message.trim();
        if message.is_empty() {
            continue;
        }
        let message = truncate_to_len(message, MAX_CHAT_MESSAGE_LEN);

        match request.channel {
            ChatChannel::Say => {
                for recipient in online_players
                    .iter()
                    .filter(|candidate| candidate.position.distance(sender.position) <= SAY_RADIUS)
                {
                    deliveries.write(ChatDelivery {
                        recipient_player_id: recipient.player_id,
                        event: ChatEvent {
                            sender: sender.username.clone(),
                            channel: ChatChannel::Say,
                            message: message.clone(),
                        },
                    });
                }
            }
            ChatChannel::Shout => {
                for recipient in &online_players {
                    deliveries.write(ChatDelivery {
                        recipient_player_id: recipient.player_id,
                        event: ChatEvent {
                            sender: sender.username.clone(),
                            channel: ChatChannel::Shout,
                            message: message.clone(),
                        },
                    });
                }
            }
            ChatChannel::Whisper => {
                let target_name = request.target.as_deref().map(str::trim).unwrap_or("");
                if target_name.is_empty() {
                    continue;
                }

                let target = online_players
                    .iter()
                    .find(|candidate| candidate.username.eq_ignore_ascii_case(target_name));

                let Some(target) = target else {
                    deliveries.write(ChatDelivery {
                        recipient_player_id: sender.player_id,
                        event: ChatEvent {
                            sender: "System".to_string(),
                            channel: ChatChannel::Whisper,
                            message: format!("{} is not online.", target_name),
                        },
                    });
                    continue;
                };

                deliveries.write(ChatDelivery {
                    recipient_player_id: sender.player_id,
                    event: ChatEvent {
                        sender: format!("{} -> {}", sender.username, target.username),
                        channel: ChatChannel::Whisper,
                        message: message.clone(),
                    },
                });

                if target.player_id != sender.player_id {
                    deliveries.write(ChatDelivery {
                        recipient_player_id: target.player_id,
                        event: ChatEvent {
                            sender: format!("{} -> you", sender.username),
                            channel: ChatChannel::Whisper,
                            message,
                        },
                    });
                }
            }
            ChatChannel::Guild => {
                let Some(guild_name) = sender.guild_name.as_ref() else {
                    deliveries.write(ChatDelivery {
                        recipient_player_id: sender.player_id,
                        event: ChatEvent {
                            sender: "System".to_string(),
                            channel: ChatChannel::Guild,
                            message: "You are not in a guild.".to_string(),
                        },
                    });
                    continue;
                };

                for recipient in online_players
                    .iter()
                    .filter(|candidate| candidate.guild_name.as_ref() == Some(guild_name))
                {
                    deliveries.write(ChatDelivery {
                        recipient_player_id: recipient.player_id,
                        event: ChatEvent {
                            sender: sender.username.clone(),
                            channel: ChatChannel::Guild,
                            message: message.clone(),
                        },
                    });
                }
            }
        }
    }
}

fn collect_online_players(
    network: &network::ServerNetwork,
    players: &Query<
        (&Position, &network::NetworkEntity, Option<&GuildMembership>),
        With<network::PlayerCharacter>,
    >,
) -> Vec<OnlinePlayer> {
    network
        .sessions
        .values()
        .filter(|session| session.logged_in)
        .filter_map(|session| {
            let (Some(entity), Some(player_id), Some(username)) =
                (session.entity, session.player_id, session.username.clone())
            else {
                return None;
            };
            let Ok((position, _, guild)) = players.get(entity) else {
                return None;
            };
            Some(OnlinePlayer {
                player_entity: entity,
                player_id,
                username,
                position: Vec2::new(position.x, position.y),
                guild_name: guild.map(|value| value.guild_name.clone()),
            })
        })
        .collect()
}

fn truncate_to_len(text: &str, max_len: usize) -> String {
    text.chars().take(max_len).collect()
}
