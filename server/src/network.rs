use bevy::prelude::*;
use shared::protocol::{
    decode_client_message, encode_server_message, AttackIntent, ClientMessage, DamageEvent,
    EntityState, EquipmentUpdate, InventoryUpdate, ItemDespawnEvent, ItemSpawnEvent, LoginRequest,
    LoginResponse, LootIntent, ManaUpdate, NetworkEntityKind, ServerMessage,
};
use shared::{
    ActionState, ArmorClass, CombatStats, Health, Mana, MoveSpeed, Position, SpellCooldowns,
    TargetPosition,
};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::Instant;

use crate::{
    db,
    systems::{combat, drop, equipment, loot, spell},
};

const SERVER_BIND_ADDR: &str = "127.0.0.1:5000";
const MAX_PACKET_SIZE: usize = 1024;
const SESSION_TIMEOUT_SECS: f32 = 15.0;

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
    pub last_seen: Instant,
}

impl SessionState {
    fn new() -> Self {
        Self {
            username: None,
            entity: None,
            player_id: None,
            logged_in: false,
            login_pending: false,
            last_seen: Instant::now(),
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

#[allow(clippy::too_many_arguments)]
pub fn receive_client_messages(
    mut commands: Commands,
    network: Option<ResMut<ServerNetwork>>,
    db_bridge: Option<Res<db::DbBridge>>,
    player_snapshot: Query<(
        &Position,
        &Health,
        &Mana,
        &shared::Inventory,
        &shared::EquipmentMap,
    )>,
    mut attack_requests: MessageWriter<combat::AttackRequest>,
    mut loot_requests: MessageWriter<loot::LootRequest>,
    mut cast_spell_requests: MessageWriter<spell::CastSpellRequest>,
    mut equip_requests: MessageWriter<equipment::EquipRequest>,
    mut unequip_requests: MessageWriter<equipment::UnequipRequest>,
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
        network
            .sessions
            .entry(address)
            .or_insert_with(SessionState::new)
            .last_seen = Instant::now();

        match message {
            ClientMessage::LoginRequest(LoginRequest { username }) => {
                let requested = username.trim().to_string();
                if let Some(existing) = network.sessions.get(&address).cloned() {
                    if existing.logged_in
                        && existing.username.as_deref() != Some(requested.as_str())
                    {
                        if let (Some(username), Some(entity)) =
                            (existing.username.clone(), existing.entity)
                        {
                            if let Ok((position, health, mana, inventory, equipment)) =
                                player_snapshot.get(entity)
                            {
                                let _ = db_bridge.command_tx.send(db::DbCommand::SavePlayer {
                                    data: db::PersistedPlayer {
                                        username,
                                        x: position.x,
                                        y: position.y,
                                        health_current: health.current,
                                        health_max: health.max,
                                        mana_current: mana.current,
                                        mana_max: mana.max,
                                        inventory: inventory.clone(),
                                        equipment: equipment.clone(),
                                    },
                                });
                            }
                        }
                        if let Some(entity) = existing.entity {
                            commands.entity(entity).despawn();
                        }
                        network.sessions.remove(&address);
                    }
                }
                handle_login_request(
                    &mut network,
                    &socket_clone,
                    &db_bridge.command_tx,
                    address,
                    requested,
                );
            }
            ClientMessage::MoveIntent(intent) => {
                let session = network
                    .sessions
                    .entry(address)
                    .or_insert_with(SessionState::new);
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
                let session = network
                    .sessions
                    .entry(address)
                    .or_insert_with(SessionState::new);
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
                let session = network
                    .sessions
                    .entry(address)
                    .or_insert_with(SessionState::new);
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
            ClientMessage::CastSpellIntent(intent) => {
                let session = network
                    .sessions
                    .entry(address)
                    .or_insert_with(SessionState::new);
                if !session.logged_in {
                    continue;
                }
                if let Some(player_entity) = session.entity {
                    cast_spell_requests.write(spell::CastSpellRequest {
                        caster_entity: player_entity,
                        spell: intent.spell,
                        target_id: intent.target_id,
                    });
                }
            }
            ClientMessage::EquipIntent(intent) => {
                let session = network
                    .sessions
                    .entry(address)
                    .or_insert_with(SessionState::new);
                if !session.logged_in {
                    continue;
                }
                if let Some(player_entity) = session.entity {
                    equip_requests.write(equipment::EquipRequest {
                        player_entity,
                        item_type: intent.item_type,
                    });
                }
            }
            ClientMessage::UnequipIntent(intent) => {
                let session = network
                    .sessions
                    .entry(address)
                    .or_insert_with(SessionState::new);
                if !session.logged_in {
                    continue;
                }
                if let Some(player_entity) = session.entity {
                    unequip_requests.write(equipment::UnequipRequest {
                        player_entity,
                        slot: intent.slot,
                    });
                }
            }
        }
    }
}

fn handle_login_request(
    network: &mut ServerNetwork,
    socket: &UdpSocket,
    command_tx: &crossbeam_channel::Sender<db::DbCommand>,
    address: SocketAddr,
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

    // UDP has no disconnect event. If the same username reconnects from a new
    // source port/address, resume the existing session instead of spawning a
    // duplicate player entity.
    if let Some((old_addr, old_session)) = network
        .sessions
        .iter()
        .find(|(session_addr, session)| {
            **session_addr != address
                && session.logged_in
                && session.username.as_deref() == Some(username.as_str())
        })
        .map(|(session_addr, session)| (*session_addr, session.clone()))
    {
        network.sessions.remove(&old_addr);
        let mut resumed = old_session.clone();
        resumed.last_seen = Instant::now();
        network.sessions.insert(address, resumed.clone());

        let response = ServerMessage::LoginResponse(LoginResponse {
            success: true,
            message: format!("welcome back {}", username),
        });
        if let Ok(payload) = encode_server_message(&response) {
            let _ = socket.send_to(&payload, address);
        }

        if let Some(player_id) = resumed.player_id {
            let assigned = ServerMessage::AssignedPlayer { player_id };
            if let Ok(payload) = encode_server_message(&assigned) {
                let _ = socket.send_to(&payload, address);
            }
        }
        return;
    }

    let session = network
        .sessions
        .entry(address)
        .or_insert_with(SessionState::new);

    if session.logged_in {
        session.last_seen = Instant::now();
        let message = ServerMessage::LoginResponse(LoginResponse {
            success: true,
            message: "already logged in".to_string(),
        });
        if let Ok(payload) = encode_server_message(&message) {
            let _ = socket.send_to(&payload, address);
        }
        if let Some(player_id) = session.player_id {
            let assigned = ServerMessage::AssignedPlayer { player_id };
            if let Ok(payload) = encode_server_message(&assigned) {
                let _ = socket.send_to(&payload, address);
            }
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

pub fn cleanup_stale_sessions(
    mut commands: Commands,
    network: Option<ResMut<ServerNetwork>>,
    db_bridge: Option<Res<db::DbBridge>>,
    players: Query<(
        &Position,
        &Health,
        &Mana,
        &shared::Inventory,
        &shared::EquipmentMap,
    )>,
) {
    let Some(mut network) = network else {
        return;
    };

    let now = Instant::now();
    let stale_addresses: Vec<SocketAddr> = network
        .sessions
        .iter()
        .filter_map(|(address, session)| {
            let idle = now.duration_since(session.last_seen).as_secs_f32();
            (idle > SESSION_TIMEOUT_SECS).then_some(*address)
        })
        .collect();

    for address in stale_addresses {
        if let Some(session) = network.sessions.remove(&address) {
            if let (Some(username), Some(entity), Some(db_bridge)) =
                (session.username.clone(), session.entity, db_bridge.as_ref())
            {
                if let Ok((position, health, mana, inventory, equipment)) = players.get(entity) {
                    let _ = db_bridge.command_tx.send(db::DbCommand::SavePlayer {
                        data: db::PersistedPlayer {
                            username,
                            x: position.x,
                            y: position.y,
                            health_current: health.current,
                            health_max: health.max,
                            mana_current: mana.current,
                            mana_max: mana.max,
                            inventory: inventory.clone(),
                            equipment: equipment.clone(),
                        },
                    });
                }
            }
            if let Some(entity) = session.entity {
                commands.entity(entity).despawn();
            }
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
                let mut combat_stats = CombatStats {
                    attack_power: 25,
                    attack_range: 90.0,
                    attack_speed: 1.0,
                };
                let mut armor_class = ArmorClass::default();
                equipment::recalculate_stats_from_equipment(
                    &data.equipment,
                    &mut combat_stats,
                    &mut armor_class,
                );
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
                        Mana {
                            current: data.mana_current,
                            max: data.mana_max.max(1),
                        },
                        armor_class,
                        combat_stats,
                        SpellCooldowns::default(),
                        ActionState::default(),
                        data.inventory,
                        data.equipment.clone(),
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

                let mana_message = ServerMessage::ManaUpdate(ManaUpdate {
                    player_id,
                    current: data.mana_current,
                    max: data.mana_max.max(1),
                });
                if let Ok(payload) = encode_server_message(&mana_message) {
                    let _ = network.socket.send_to(&payload, address);
                }

                let equipment_message = ServerMessage::EquipmentUpdate(EquipmentUpdate {
                    player_id,
                    equipment: data.equipment,
                });
                if let Ok(payload) = encode_server_message(&equipment_message) {
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

pub fn broadcast_spell_events(
    network: Option<Res<ServerNetwork>>,
    mut mana_events: MessageReader<spell::ManaChangedMessage>,
    mut heal_events: MessageReader<spell::HealEventMessage>,
) {
    let Some(network) = network else {
        return;
    };

    for mana in mana_events.read() {
        let message = ServerMessage::ManaUpdate(spell::to_mana_update(*mana));
        let Ok(payload) = encode_server_message(&message) else {
            continue;
        };
        for (&address, session) in &network.sessions {
            if session.logged_in && session.player_id == Some(mana.player_id) {
                let _ = network.socket.send_to(&payload, address);
            }
        }
    }

    for heal in heal_events.read() {
        let message = ServerMessage::HealEvent(spell::to_heal_event(*heal));
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

pub fn broadcast_equipment_events(
    network: Option<Res<ServerNetwork>>,
    mut equipment_events: MessageReader<equipment::EquipmentChangedMessage>,
) {
    let Some(network) = network else {
        return;
    };

    for changed in equipment_events.read() {
        let message = ServerMessage::EquipmentUpdate(equipment::to_equipment_update(changed));
        let Ok(payload) = encode_server_message(&message) else {
            continue;
        };
        for (&address, session) in &network.sessions {
            if session.logged_in && session.player_id == Some(changed.player_id) {
                let _ = network.socket.send_to(&payload, address);
            }
        }
    }
}
