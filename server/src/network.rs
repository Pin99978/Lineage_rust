use bevy::prelude::*;
use shared::protocol::{
    decode_client_message, encode_server_message, AttackIntent, ClientMessage, DamageEvent,
    EntityState, InventoryUpdate, ItemDespawnEvent, ItemSpawnEvent, LootIntent, NetworkEntityKind,
    ServerMessage,
};
use shared::{ActionState, CombatStats, Health, Inventory, MoveSpeed, Position, TargetPosition};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};

use crate::systems::{combat, drop, loot};

const SERVER_BIND_ADDR: &str = "127.0.0.1:5000";
const MAX_PACKET_SIZE: usize = 1024;

#[derive(Component, Debug, Clone, Copy)]
pub struct NetworkEntity {
    pub id: u64,
    pub kind: NetworkEntityKind,
}

#[derive(Component)]
pub struct PlayerCharacter;

#[derive(Resource)]
pub struct ServerNetwork {
    socket: UdpSocket,
    clients: HashMap<SocketAddr, Entity>,
    next_entity_id: u64,
}

impl ServerNetwork {
    pub fn allocate_entity_id(&mut self) -> u64 {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        id
    }
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
        next_entity_id: 1,
    });
}

pub fn receive_client_messages(
    mut commands: Commands,
    network: Option<ResMut<ServerNetwork>>,
    mut attack_requests: MessageWriter<combat::AttackRequest>,
    mut loot_requests: MessageWriter<loot::LootRequest>,
) {
    let Some(mut network) = network else {
        return;
    };

    let mut buffer = [0_u8; MAX_PACKET_SIZE];
    loop {
        let Ok((size, address)) = network.socket.recv_from(&mut buffer) else {
            break;
        };
        let Ok(message) = decode_client_message(&buffer[..size]) else {
            continue;
        };

        let player_entity = ensure_client_player(&mut commands, &mut network, address);
        match message {
            ClientMessage::MoveIntent(intent) => {
                commands.entity(player_entity).insert(TargetPosition {
                    x: intent.target_x,
                    y: intent.target_y,
                });
            }
            ClientMessage::AttackIntent(AttackIntent { target_id }) => {
                attack_requests.write(combat::AttackRequest {
                    attacker_entity: player_entity,
                    target_id,
                });
            }
            ClientMessage::LootIntent(LootIntent { item_id }) => {
                loot_requests.write(loot::LootRequest {
                    looter_entity: player_entity,
                    item_id,
                });
            }
        }
    }
}

fn ensure_client_player(
    commands: &mut Commands,
    network: &mut ServerNetwork,
    address: SocketAddr,
) -> Entity {
    if let Some(entity) = network.clients.get(&address).copied() {
        return entity;
    }

    let player_id = network.allocate_entity_id();
    let player_entity = commands
        .spawn((
            PlayerCharacter,
            NetworkEntity {
                id: player_id,
                kind: NetworkEntityKind::Player,
            },
            Position { x: -300.0, y: 0.0 },
            TargetPosition { x: -300.0, y: 0.0 },
            MoveSpeed { value: 320.0 },
            Health::default(),
            CombatStats {
                attack_power: 25,
                attack_range: 90.0,
                attack_speed: 1.0,
            },
            ActionState::default(),
            Inventory::default(),
        ))
        .id();
    network.clients.insert(address, player_entity);

    let assigned = ServerMessage::AssignedPlayer { player_id };
    if let Ok(payload) = encode_server_message(&assigned) {
        let _ = network.socket.send_to(&payload, address);
    }

    player_entity
}

pub fn broadcast_world_state(
    network: Option<Res<ServerNetwork>>,
    entities: Query<(&NetworkEntity, &Position, Option<&Health>)>,
) {
    let Some(network) = network else {
        return;
    };

    for &address in network.clients.keys() {
        for (network_entity, position, health) in &entities {
            let (health_current, health_max, alive) = if let Some(health) = health {
                (health.current, health.max, health.current > 0)
            } else {
                (0, 0, true)
            };
            let message = ServerMessage::EntityState(EntityState {
                entity_id: network_entity.id,
                kind: network_entity.kind,
                x: position.x,
                y: position.y,
                health_current,
                health_max,
                alive,
            });
            let Ok(payload) = encode_server_message(&message) else {
                continue;
            };
            let _ = network.socket.send_to(&payload, address);
        }
    }
}

pub fn broadcast_combat_events(
    network: Option<Res<ServerNetwork>>,
    mut damage_events: MessageReader<combat::CombatDamageEvent>,
    mut death_events: MessageReader<combat::CombatDeathEvent>,
) {
    let Some(network) = network else {
        return;
    };

    for damage in damage_events.read() {
        let message = ServerMessage::DamageEvent(DamageEvent {
            target_id: damage.target_id,
            amount: damage.amount,
            remaining_hp: damage.remaining_hp,
        });
        let Ok(payload) = encode_server_message(&message) else {
            continue;
        };
        for &address in network.clients.keys() {
            let _ = network.socket.send_to(&payload, address);
        }
    }

    for death in death_events.read() {
        let message = ServerMessage::DeathEvent(shared::protocol::DeathEvent {
            target_id: death.target_id,
        });
        let Ok(payload) = encode_server_message(&message) else {
            continue;
        };
        for &address in network.clients.keys() {
            let _ = network.socket.send_to(&payload, address);
        }
    }
}

pub fn broadcast_item_events(
    network: Option<Res<ServerNetwork>>,
    mut spawned_items: MessageReader<drop::ItemSpawnedMessage>,
    mut despawned_items: MessageReader<loot::ItemDespawnedMessage>,
    mut inventory_updates: MessageReader<loot::InventoryUpdateMessage>,
    players: Query<(Entity, &NetworkEntity), With<PlayerCharacter>>,
) {
    let Some(network) = network else {
        return;
    };

    for spawned in spawned_items.read() {
        let message = ServerMessage::ItemSpawnEvent(ItemSpawnEvent {
            item_id: spawned.item_id,
            item_type: spawned.item_type,
            amount: spawned.amount,
            x: spawned.x,
            y: spawned.y,
        });
        let Ok(payload) = encode_server_message(&message) else {
            continue;
        };
        for &address in network.clients.keys() {
            let _ = network.socket.send_to(&payload, address);
        }
    }

    for despawned in despawned_items.read() {
        let message = ServerMessage::ItemDespawnEvent(ItemDespawnEvent {
            item_id: despawned.item_id,
        });
        let Ok(payload) = encode_server_message(&message) else {
            continue;
        };
        for &address in network.clients.keys() {
            let _ = network.socket.send_to(&payload, address);
        }
    }

    for inventory in inventory_updates.read() {
        let message = ServerMessage::InventoryUpdate(InventoryUpdate {
            player_id: inventory.player_id,
            item_type: inventory.item_type,
            amount: inventory.amount,
        });
        let Ok(payload) = encode_server_message(&message) else {
            continue;
        };

        for (&address, &entity) in &network.clients {
            let Ok((_, player_network)) = players.get(entity) else {
                continue;
            };
            if player_network.id == inventory.player_id {
                let _ = network.socket.send_to(&payload, address);
            }
        }
    }
}
