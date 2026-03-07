use bevy::prelude::*;
use shared::protocol::{
    decode_client_message, encode_server_message, AttackIntent, ClientMessage, DamageEvent,
    EntityState, InventoryUpdate, ItemDespawnEvent, ItemSpawnEvent, LoginRequest, LoginResponse,
    LootIntent, NetworkEntityKind, ServerMessage,
};
use shared::{ActionState, CombatStats, Health, MoveSpeed, Position, TargetPosition};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};

use crate::{
    db,
    systems::{combat, drop, loot},
};

const SERVER_BIND_ADDR: &str = "127.0.0.1:5000";
const MAX_PACKET_SIZE: usize = 1024;

#[derive(Component, Debug, Clone, Copy)]
pub struct NetworkEntity {
    pub id: u64,
    pub kind: NetworkEntityKind,
}

#[derive(Component)]
pub struct PlayerCharacter;

#[derive(Debug, Clone)]
pub struct SessionState {
    pub username: Option<String>,
    pub entity: Option<Entity>,
    pub player_id: Option<u64>,
    pub logged_in: bool,
    pub login_pending: bool,
}

impl SessionState {
    fn new() -> Self {
        Self {
            username: None,
            entity: None,
            player_id: None,
            logged_in: false,
            login_pending: false,
        }
    }
}

#[derive(Resource)]
pub struct ServerNetwork {
    socket: UdpSocket,
    pub sessions: HashMap<SocketAddr, SessionState>,
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
        sessions: HashMap::new(),
        next_entity_id: 1,
    });
}

pub fn receive_client_messages(
    mut commands: Commands,
    network: Option<ResMut<ServerNetwork>>,
    db_bridge: Option<Res<db::DbBridge>>,
    mut attack_requests: MessageWriter<combat::AttackRequest>,
    mut loot_requests: MessageWriter<loot::LootRequest>,
) {
    let Some(mut network) = network else {
        return;
    };
    let Some(db_bridge) = db_bridge else {
        return;
    };
    let Ok(socket_clone) = network.socket.try_clone() else {
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

        let session = network
            .sessions
            .entry(address)
            .or_insert_with(SessionState::new);

        match message {
            ClientMessage::LoginRequest(LoginRequest { username }) => {
                handle_login_request(
                    &socket_clone,
                    &db_bridge.command_tx,
                    address,
                    session,
                    username,
                );
            }
            ClientMessage::MoveIntent(intent) => {
                if !session.logged_in {
                    continue;
                }
                if let Some(player_entity) = session.entity {
                    commands.entity(player_entity).insert(TargetPosition {
                        x: intent.target_x,
                        y: intent.target_y,
                    });
                }
            }
            ClientMessage::AttackIntent(AttackIntent { target_id }) => {
                if !session.logged_in {
                    continue;
                }
                if let Some(player_entity) = session.entity {
                    attack_requests.write(combat::AttackRequest {
                        attacker_entity: player_entity,
                        target_id,
                    });
                }
            }
            ClientMessage::LootIntent(LootIntent { item_id }) => {
                if !session.logged_in {
                    continue;
                }
                if let Some(player_entity) = session.entity {
                    loot_requests.write(loot::LootRequest {
                        looter_entity: player_entity,
                        item_id,
                    });
                }
            }
        }
    }
}

fn handle_login_request(
    socket: &UdpSocket,
    command_tx: &crossbeam_channel::Sender<db::DbCommand>,
    address: SocketAddr,
    session: &mut SessionState,
    username: String,
) {
    let username = username.trim().to_string();
    if username.is_empty() || username.len() > 24 {
        let message = ServerMessage::LoginResponse(LoginResponse {
            success: false,
            message: "invalid username".to_string(),
        });
        if let Ok(payload) = encode_server_message(&message) {
            let _ = socket.send_to(&payload, address);
        }
        return;
    }

    if session.logged_in {
        let message = ServerMessage::LoginResponse(LoginResponse {
            success: true,
            message: "already logged in".to_string(),
        });
        if let Ok(payload) = encode_server_message(&message) {
            let _ = socket.send_to(&payload, address);
        }
        return;
    }

    if session.login_pending {
        return;
    }

    if command_tx
        .send(db::DbCommand::LoadOrCreate {
            address,
            username: username.clone(),
        })
        .is_ok()
    {
        session.login_pending = true;
    } else {
        let message = ServerMessage::LoginResponse(LoginResponse {
            success: false,
            message: "database offline".to_string(),
        });
        if let Ok(payload) = encode_server_message(&message) {
            let _ = socket.send_to(&payload, address);
        }
    }
}

pub fn apply_db_results(
    mut commands: Commands,
    network: Option<ResMut<ServerNetwork>>,
    db_bridge: Option<Res<db::DbBridge>>,
) {
    let Some(mut network) = network else {
        return;
    };
    let Some(db_bridge) = db_bridge else {
        return;
    };

    while let Ok(result) = db_bridge.result_rx.try_recv() {
        match result {
            db::DbResult::PlayerLoaded { address, data } => {
                let already_logged_in = network
                    .sessions
                    .get(&address)
                    .map(|session| session.logged_in)
                    .unwrap_or(false);
                if already_logged_in {
                    continue;
                }

                let player_id = network.allocate_entity_id();
                let player_entity = commands
                    .spawn((
                        PlayerCharacter,
                        NetworkEntity {
                            id: player_id,
                            kind: NetworkEntityKind::Player,
                        },
                        Position {
                            x: data.x,
                            y: data.y,
                        },
                        TargetPosition {
                            x: data.x,
                            y: data.y,
                        },
                        MoveSpeed { value: 320.0 },
                        Health {
                            current: data.health_current,
                            max: data.health_max.max(1),
                        },
                        CombatStats {
                            attack_power: 25,
                            attack_range: 90.0,
                            attack_speed: 1.0,
                        },
                        ActionState::default(),
                        data.inventory,
                    ))
                    .id();

                let session = network
                    .sessions
                    .entry(address)
                    .or_insert_with(SessionState::new);
                session.username = Some(data.username.clone());
                session.entity = Some(player_entity);
                session.player_id = Some(player_id);
                session.logged_in = true;
                session.login_pending = false;

                let login_response = ServerMessage::LoginResponse(LoginResponse {
                    success: true,
                    message: format!("welcome {}", data.username),
                });
                if let Ok(payload) = encode_server_message(&login_response) {
                    let _ = network.socket.send_to(&payload, address);
                }

                let assigned = ServerMessage::AssignedPlayer { player_id };
                if let Ok(payload) = encode_server_message(&assigned) {
                    let _ = network.socket.send_to(&payload, address);
                }
            }
            db::DbResult::LoginFailed { address, message } => {
                if let Some(session) = network.sessions.get_mut(&address) {
                    session.login_pending = false;
                }
                let response = ServerMessage::LoginResponse(LoginResponse {
                    success: false,
                    message,
                });
                if let Ok(payload) = encode_server_message(&response) {
                    let _ = network.socket.send_to(&payload, address);
                }
            }
            db::DbResult::SaveError { username, message } => {
                warn!("save failed for {}: {}", username, message);
            }
        }
    }
}

pub fn broadcast_world_state(
    network: Option<Res<ServerNetwork>>,
    entities: Query<(&NetworkEntity, &Position, Option<&Health>)>,
) {
    let Some(network) = network else {
        return;
    };

    for (&address, session) in &network.sessions {
        if !session.logged_in {
            continue;
        }
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
        for (&address, session) in &network.sessions {
            if session.logged_in {
                let _ = network.socket.send_to(&payload, address);
            }
        }
    }

    for death in death_events.read() {
        let message = ServerMessage::DeathEvent(shared::protocol::DeathEvent {
            target_id: death.target_id,
        });
        let Ok(payload) = encode_server_message(&message) else {
            continue;
        };
        for (&address, session) in &network.sessions {
            if session.logged_in {
                let _ = network.socket.send_to(&payload, address);
            }
        }
    }
}

pub fn broadcast_item_events(
    network: Option<Res<ServerNetwork>>,
    mut spawned_items: MessageReader<drop::ItemSpawnedMessage>,
    mut despawned_items: MessageReader<loot::ItemDespawnedMessage>,
    mut inventory_updates: MessageReader<loot::InventoryUpdateMessage>,
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
        for (&address, session) in &network.sessions {
            if session.logged_in {
                let _ = network.socket.send_to(&payload, address);
            }
        }
    }

    for despawned in despawned_items.read() {
        let message = ServerMessage::ItemDespawnEvent(ItemDespawnEvent {
            item_id: despawned.item_id,
        });
        let Ok(payload) = encode_server_message(&message) else {
            continue;
        };
        for (&address, session) in &network.sessions {
            if session.logged_in {
                let _ = network.socket.send_to(&payload, address);
            }
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

        for (&address, session) in &network.sessions {
            if session.logged_in && session.player_id == Some(inventory.player_id) {
                let _ = network.socket.send_to(&payload, address);
            }
        }
    }
}
