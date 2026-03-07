use bevy::prelude::*;
use shared::protocol::{
    decode_server_message, encode_client_message, ClientMessage, MoveIntent, ServerMessage,
};
use shared::Position;
use std::net::{SocketAddr, UdpSocket};

use crate::Player;

const SERVER_ADDR: &str = "127.0.0.1:5000";
const MAX_PACKET_SIZE: usize = 1024;

#[derive(Resource)]
pub struct ClientNetwork {
    socket: UdpSocket,
    server_addr: SocketAddr,
}

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
}

pub fn send_move_intent(network: &ClientNetwork, intent: MoveIntent) {
    let message = ClientMessage::MoveIntent(intent);
    let Ok(payload) = encode_client_message(&message) else {
        return;
    };
    let _ = network.socket.send_to(&payload, network.server_addr);
}

pub fn receive_server_state(
    network: Option<Res<ClientNetwork>>,
    mut player_query: Query<&mut Position, With<Player>>,
) {
    let Some(network) = network else {
        return;
    };

    let mut buffer = [0_u8; MAX_PACKET_SIZE];
    loop {
        let packet = network.socket.recv_from(&mut buffer);
        let Ok((size, _)) = packet else {
            break;
        };

        let Ok(ServerMessage::PlayerState(state)) = decode_server_message(&buffer[..size]) else {
            continue;
        };

        for mut position in &mut player_query {
            position.x = state.x;
            position.y = state.y;
        }
    }
}
