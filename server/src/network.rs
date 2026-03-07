use bevy::prelude::*;
use shared::protocol::{
    decode_client_message, encode_server_message, ClientMessage, PlayerState, ServerMessage,
};
use shared::{MoveSpeed, Position, TargetPosition};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};

const SERVER_BIND_ADDR: &str = "127.0.0.1:5000";
const MAX_PACKET_SIZE: usize = 1024;

#[derive(Component)]
pub struct NetworkPlayer {
    pub id: u64,
}

#[derive(Resource)]
pub struct ServerNetwork {
    socket: UdpSocket,
    clients: HashMap<SocketAddr, Entity>,
    next_player_id: u64,
}

pub fn setup_network(mut commands: Commands) {
    let Ok(socket) = UdpSocket::bind(SERVER_BIND_ADDR) else {
        return;
    };
    if socket.set_nonblocking(true).is_err() {
        return;
    }

    commands.insert_resource(ServerNetwork {
        socket,
        clients: HashMap::new(),
        next_player_id: 1,
    });
}

pub fn receive_client_messages(mut commands: Commands, network: Option<ResMut<ServerNetwork>>) {
    let Some(mut network) = network else {
        return;
    };

    let mut buffer = [0_u8; MAX_PACKET_SIZE];
    loop {
        let packet = network.socket.recv_from(&mut buffer);
        let Ok((size, address)) = packet else {
            break;
        };

        let Ok(ClientMessage::MoveIntent(intent)) = decode_client_message(&buffer[..size]) else {
            continue;
        };

        let entity = if let Some(entity) = network.clients.get(&address).copied() {
            entity
        } else {
            let id = network.next_player_id;
            network.next_player_id += 1;
            let spawned = commands
                .spawn((
                    NetworkPlayer { id },
                    Position::default(),
                    TargetPosition::default(),
                    MoveSpeed { value: 320.0 },
                ))
                .id();
            network.clients.insert(address, spawned);
            spawned
        };

        commands.entity(entity).insert(TargetPosition {
            x: intent.target_x,
            y: intent.target_y,
        });
    }
}

pub fn broadcast_player_state(
    network: Option<Res<ServerNetwork>>,
    players: Query<(&NetworkPlayer, &Position)>,
) {
    let Some(network) = network else {
        return;
    };

    for (&address, &entity) in &network.clients {
        let Ok((player, position)) = players.get(entity) else {
            continue;
        };

        let message = ServerMessage::PlayerState(PlayerState {
            player_id: player.id,
            x: position.x,
            y: position.y,
        });
        let Ok(payload) = encode_server_message(&message) else {
            continue;
        };

        let _ = network.socket.send_to(&payload, address);
    }
}
