use bevy::prelude::*;
use shared::protocol::{
    decode_server_message, encode_client_message, AttackIntent, ClientMessage, EntityState,
    NetworkEntityKind, ServerMessage,
};
use shared::{Health, Position};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};

use crate::{systems, Player};

const SERVER_ADDR: &str = "127.0.0.1:5000";
const MAX_PACKET_SIZE: usize = 1024;

#[derive(Resource)]
pub struct ClientNetwork {
    socket: UdpSocket,
    server_addr: SocketAddr,
}

#[derive(Resource, Default)]
pub struct LocalPlayer {
    pub id: Option<u64>,
}

#[derive(Resource, Default)]
pub struct NetworkEntityMap {
    pub entity_by_id: HashMap<u64, Entity>,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct NetworkEntityVisual {
    pub id: u64,
}

#[derive(Component)]
pub struct Attackable;

pub fn setup_network(mut commands: Commands) {
    let Ok(server_addr) = SERVER_ADDR.parse() else {
        return;
    };
    let Ok(socket) = UdpSocket::bind("127.0.0.1:0") else {
        return;
    };
    if socket.set_nonblocking(true).is_err() {
        return;
    }

    commands.insert_resource(ClientNetwork {
        socket,
        server_addr,
    });
    commands.insert_resource(LocalPlayer::default());
    commands.insert_resource(NetworkEntityMap::default());
}

pub fn send_move_intent(network: &ClientNetwork, intent: shared::protocol::MoveIntent) {
    let message = ClientMessage::MoveIntent(intent);
    send_to_server(network, &message);
}

pub fn send_attack_intent(network: &ClientNetwork, intent: AttackIntent) {
    let message = ClientMessage::AttackIntent(intent);
    send_to_server(network, &message);
}

fn send_to_server(network: &ClientNetwork, message: &ClientMessage) {
    let Ok(payload) = encode_client_message(message) else {
        return;
    };
    let _ = network.socket.send_to(&payload, network.server_addr);
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn receive_server_state(
    mut commands: Commands,
    network: Option<Res<ClientNetwork>>,
    local_player: Option<ResMut<LocalPlayer>>,
    entity_map: Option<ResMut<NetworkEntityMap>>,
    mut player_query: Query<
        (Entity, &mut Position, &mut Health, Option<&mut Sprite>),
        With<Player>,
    >,
    mut visuals_query: Query<
        (
            Entity,
            &NetworkEntityVisual,
            &mut Position,
            &mut Health,
            Option<&mut Sprite>,
        ),
        Without<Player>,
    >,
    mut damage_feedback: MessageWriter<systems::combat_render::DamagePopupEvent>,
    mut death_feedback: MessageWriter<systems::combat_render::DeathVisualEvent>,
) {
    let Some(network) = network else {
        return;
    };
    let Some(mut local_player) = local_player else {
        return;
    };
    let Some(mut entity_map) = entity_map else {
        return;
    };

    let mut buffer = [0_u8; MAX_PACKET_SIZE];
    loop {
        let Ok((size, _)) = network.socket.recv_from(&mut buffer) else {
            break;
        };
        let Ok(message) = decode_server_message(&buffer[..size]) else {
            continue;
        };

        match message {
            ServerMessage::AssignedPlayer { player_id } => {
                local_player.id = Some(player_id);
                let Ok((player_entity, _, _, _)) = player_query.single_mut() else {
                    continue;
                };
                commands
                    .entity(player_entity)
                    .insert(NetworkEntityVisual { id: player_id });
                entity_map.entity_by_id.insert(player_id, player_entity);
            }
            ServerMessage::EntityState(state) => {
                apply_entity_state(
                    &mut commands,
                    &mut local_player,
                    &mut entity_map,
                    &mut player_query,
                    &mut visuals_query,
                    state,
                );
            }
            ServerMessage::DamageEvent(event) => {
                damage_feedback.write(systems::combat_render::DamagePopupEvent {
                    target_id: event.target_id,
                    amount: event.amount,
                });
            }
            ServerMessage::DeathEvent(event) => {
                death_feedback.write(systems::combat_render::DeathVisualEvent {
                    target_id: event.target_id,
                });
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn apply_entity_state(
    commands: &mut Commands,
    local_player: &mut LocalPlayer,
    entity_map: &mut NetworkEntityMap,
    player_query: &mut Query<
        (Entity, &mut Position, &mut Health, Option<&mut Sprite>),
        With<Player>,
    >,
    visuals_query: &mut Query<
        (
            Entity,
            &NetworkEntityVisual,
            &mut Position,
            &mut Health,
            Option<&mut Sprite>,
        ),
        Without<Player>,
    >,
    state: EntityState,
) {
    if local_player.id == Some(state.entity_id) {
        let Ok((player_entity, mut position, mut health, sprite)) = player_query.single_mut()
        else {
            return;
        };
        position.x = state.x;
        position.y = state.y;
        health.current = state.health_current;
        health.max = state.health_max;
        if let Some(mut sprite) = sprite {
            sprite.color = if state.alive {
                Color::srgb(0.1, 0.4, 1.0)
            } else {
                Color::srgb(0.35, 0.35, 0.35)
            };
        }
        entity_map
            .entity_by_id
            .insert(state.entity_id, player_entity);
        return;
    }

    if let Some(existing_entity) = entity_map.entity_by_id.get(&state.entity_id).copied() {
        if let Ok((_, _, mut position, mut health, sprite)) = visuals_query.get_mut(existing_entity)
        {
            position.x = state.x;
            position.y = state.y;
            health.current = state.health_current;
            health.max = state.health_max;
            if let Some(mut sprite) = sprite {
                sprite.color = if state.alive {
                    color_for_kind(state.kind)
                } else {
                    Color::srgb(0.35, 0.35, 0.35)
                };
            }
            return;
        }
    }

    let mut entity_commands = commands.spawn((
        NetworkEntityVisual {
            id: state.entity_id,
        },
        systems::render::YSortable,
        Position {
            x: state.x,
            y: state.y,
        },
        Health {
            current: state.health_current,
            max: state.health_max,
        },
        Sprite::from_color(color_for_kind(state.kind), Vec2::splat(32.0)),
        Transform::from_xyz(state.x, state.y, 0.0),
    ));
    if state.kind == NetworkEntityKind::Enemy {
        entity_commands.insert(Attackable);
    }
    let spawned = entity_commands.id();
    entity_map.entity_by_id.insert(state.entity_id, spawned);
}

fn color_for_kind(kind: NetworkEntityKind) -> Color {
    match kind {
        NetworkEntityKind::Player => Color::srgb(0.1, 0.6, 1.0),
        NetworkEntityKind::Enemy => Color::srgb(0.85, 0.25, 0.2),
    }
}
