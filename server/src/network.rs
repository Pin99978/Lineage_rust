use bevy::prelude::*;
use shared::protocol::{
    decode_client_message, encode_server_message, AttackIntent, ChatIntent, ClientMessage,
    DamageEvent, EntityState, EquipmentUpdate, InventoryUpdate, ItemDespawnEvent, ItemSpawnEvent,
    LoginRequest, LoginResponse, LootIntent, ManaUpdate, NetworkEntityKind, ServerMessage,
    StatusEffectUpdate, SystemNotice, UseItemIntent,
};
use shared::{
    ActionState, ArmorClass, BaseStats, Buffs, CharacterClass, CombatStats, Experience, Health,
    Level, Mana, MapId, MoveSpeed, PathQueue, Position, SpellCooldowns, TargetPosition, MAP_TOWN,
};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::Instant;

use crate::{
    db,
    systems::{
        chat, combat, drop, equipment, interaction, item, loot, movement, npc, quest, spell,
    },
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
        &Level,
        &Experience,
        &BaseStats,
        &CharacterClass,
        &shared::Inventory,
        &shared::EquipmentMap,
        &Buffs,
        &shared::QuestTracker,
    )>,
    mut attack_requests: MessageWriter<combat::AttackRequest>,
    mut loot_requests: MessageWriter<loot::LootRequest>,
    mut cast_spell_requests: MessageWriter<spell::CastSpellRequest>,
    mut equip_requests: MessageWriter<equipment::EquipRequest>,
    mut unequip_requests: MessageWriter<equipment::UnequipRequest>,
    mut interact_requests: MessageWriter<interaction::InteractRequest>,
    mut npc_interact_requests: MessageWriter<npc::NpcInteractRequest>,
    mut chat_requests: MessageWriter<chat::ChatRequest>,
    mut move_requests: MessageWriter<movement::MoveRequest>,
    mut use_item_requests: MessageWriter<item::UseItemRequest>,
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
            ClientMessage::LoginRequest(LoginRequest { username, class }) => {
                let requested = username.trim().to_string();
                if let Some(existing) = network.sessions.get(&address).cloned() {
                    if existing.logged_in
                        && existing.username.as_deref() != Some(requested.as_str())
                    {
                        if let (Some(username), Some(entity)) =
                            (existing.username.clone(), existing.entity)
                        {
                            if let Ok((
                                position,
                                health,
                                mana,
                                level,
                                exp,
                                base_stats,
                                player_class,
                                inventory,
                                equipment,
                                _buffs,
                                quests,
                            )) = player_snapshot.get(entity)
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
                                        level: level.current,
                                        exp_current: exp.current,
                                        exp_next: exp.next_level_req,
                                        base_stats: *base_stats,
                                        class: *player_class,
                                        inventory: inventory.clone(),
                                        equipment: equipment.clone(),
                                        quests: quests.clone(),
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
                    &player_snapshot,
                    address,
                    requested,
                    class,
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
                    move_requests.write(movement::MoveRequest {
                        mover_entity: player_entity,
                        target_x: intent.target_x,
                        target_y: intent.target_y,
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
            ClientMessage::InteractIntent(intent) => {
                let session = network
                    .sessions
                    .entry(address)
                    .or_insert_with(SessionState::new);
                if !session.logged_in {
                    continue;
                }
                if let Some(player_entity) = session.entity {
                    interact_requests.write(interaction::InteractRequest {
                        player_entity,
                        target_id: intent.target_id,
                    });
                    npc_interact_requests.write(npc::NpcInteractRequest {
                        player_entity,
                        target_id: intent.target_id,
                        choice_index: None,
                    });
                }
            }
            ClientMessage::InteractNpcIntent(intent) => {
                let session = network
                    .sessions
                    .entry(address)
                    .or_insert_with(SessionState::new);
                if !session.logged_in {
                    continue;
                }
                if let Some(player_entity) = session.entity {
                    npc_interact_requests.write(npc::NpcInteractRequest {
                        player_entity,
                        target_id: intent.target_id,
                        choice_index: intent.choice_index,
                    });
                }
            }
            ClientMessage::ChatIntent(ChatIntent {
                channel,
                target,
                message,
            }) => {
                let session = network
                    .sessions
                    .entry(address)
                    .or_insert_with(SessionState::new);
                if !session.logged_in {
                    continue;
                }
                if let Some(player_entity) = session.entity {
                    chat_requests.write(chat::ChatRequest {
                        player_entity,
                        channel,
                        target,
                        message,
                    });
                }
            }
            ClientMessage::UseItemIntent(UseItemIntent { item_type }) => {
                let session = network
                    .sessions
                    .entry(address)
                    .or_insert_with(SessionState::new);
                if !session.logged_in {
                    continue;
                }
                if let Some(player_entity) = session.entity {
                    use_item_requests.write(item::UseItemRequest {
                        player_entity,
                        item_type,
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
    player_snapshot: &Query<(
        &Position,
        &Health,
        &Mana,
        &Level,
        &Experience,
        &BaseStats,
        &CharacterClass,
        &shared::Inventory,
        &shared::EquipmentMap,
        &Buffs,
        &shared::QuestTracker,
    )>,
    address: SocketAddr,
    username: String,
    class: CharacterClass,
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
            if let Some(entity) = resumed.entity {
                send_player_snapshot(socket, address, player_id, entity, player_snapshot);
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
            if let Some(entity) = session.entity {
                send_player_snapshot(socket, address, player_id, entity, player_snapshot);
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
            class,
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

fn send_player_snapshot(
    socket: &UdpSocket,
    address: SocketAddr,
    player_id: u64,
    player_entity: Entity,
    player_snapshot: &Query<(
        &Position,
        &Health,
        &Mana,
        &Level,
        &Experience,
        &BaseStats,
        &CharacterClass,
        &shared::Inventory,
        &shared::EquipmentMap,
        &Buffs,
        &shared::QuestTracker,
    )>,
) {
    let Ok((_, _, mana, level, exp, base_stats, _class, inventory, equipment, buffs, quests)) =
        player_snapshot.get(player_entity)
    else {
        return;
    };

    let mana_message = ServerMessage::ManaUpdate(ManaUpdate {
        player_id,
        current: mana.current,
        max: mana.max.max(1),
    });
    if let Ok(payload) = encode_server_message(&mana_message) {
        let _ = socket.send_to(&payload, address);
    }

    let equipment_message = ServerMessage::EquipmentUpdate(EquipmentUpdate {
        player_id,
        equipment: equipment.clone(),
    });
    if let Ok(payload) = encode_server_message(&equipment_message) {
        let _ = socket.send_to(&payload, address);
    }

    for (item_type, amount) in &inventory.items {
        let inventory_message = ServerMessage::InventoryUpdate(InventoryUpdate {
            player_id,
            item_type: *item_type,
            amount: *amount,
        });
        if let Ok(payload) = encode_server_message(&inventory_message) {
            let _ = socket.send_to(&payload, address);
        }
    }

    let status_message = ServerMessage::StatusEffectUpdate(StatusEffectUpdate {
        player_id,
        effects: buffs.effects.clone(),
    });
    if let Ok(payload) = encode_server_message(&status_message) {
        let _ = socket.send_to(&payload, address);
    }

    let exp_message = ServerMessage::ExpUpdateEvent(shared::protocol::ExpUpdateEvent {
        player_id,
        level: level.current,
        exp_current: exp.current,
        exp_next: exp.next_level_req,
        str_stat: base_stats.str_stat,
        dex: base_stats.dex,
        int_stat: base_stats.int_stat,
        con: base_stats.con,
    });
    if let Ok(payload) = encode_server_message(&exp_message) {
        let _ = socket.send_to(&payload, address);
    }

    for quest in &quests.active_quests {
        let quest_message = ServerMessage::QuestUpdateEvent(shared::protocol::QuestUpdateEvent {
            player_id,
            quest_id: quest.id,
            status: quest.status.clone(),
        });
        if let Ok(payload) = encode_server_message(&quest_message) {
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
        &Level,
        &Experience,
        &BaseStats,
        &CharacterClass,
        &shared::Inventory,
        &shared::EquipmentMap,
        &shared::QuestTracker,
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
                if let Ok((
                    position,
                    health,
                    mana,
                    level,
                    exp,
                    base_stats,
                    player_class,
                    inventory,
                    equipment,
                    quests,
                )) = players.get(entity)
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
                            level: level.current,
                            exp_current: exp.current,
                            exp_next: exp.next_level_req,
                            base_stats: *base_stats,
                            class: *player_class,
                            inventory: inventory.clone(),
                            equipment: equipment.clone(),
                            quests: quests.clone(),
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
                let inventory_items = data.inventory.items.clone();
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
                let mut player_commands = commands.spawn((
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
                    PathQueue::default(),
                    Buffs::default(),
                    data.inventory,
                    data.equipment.clone(),
                ));
                player_commands.insert(MapId(MAP_TOWN.to_string()));
                player_commands.insert(data.quests.clone());
                player_commands.insert(Level {
                    current: data.level.max(1),
                });
                player_commands.insert(Experience {
                    current: data.exp_current,
                    next_level_req: data.exp_next.max(1),
                });
                player_commands.insert(data.base_stats);
                player_commands.insert(data.class);
                let player_entity = player_commands.id();

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

                for (item_type, amount) in inventory_items {
                    let inventory_message = ServerMessage::InventoryUpdate(InventoryUpdate {
                        player_id,
                        item_type,
                        amount,
                    });
                    if let Ok(payload) = encode_server_message(&inventory_message) {
                        let _ = network.socket.send_to(&payload, address);
                    }
                }

                let status_message = ServerMessage::StatusEffectUpdate(StatusEffectUpdate {
                    player_id,
                    effects: Vec::new(),
                });
                if let Ok(payload) = encode_server_message(&status_message) {
                    let _ = network.socket.send_to(&payload, address);
                }

                let exp_message = ServerMessage::ExpUpdateEvent(shared::protocol::ExpUpdateEvent {
                    player_id,
                    level: data.level.max(1),
                    exp_current: data.exp_current,
                    exp_next: data.exp_next.max(1),
                    str_stat: data.base_stats.str_stat,
                    dex: data.base_stats.dex,
                    int_stat: data.base_stats.int_stat,
                    con: data.base_stats.con,
                });
                if let Ok(payload) = encode_server_message(&exp_message) {
                    let _ = network.socket.send_to(&payload, address);
                }

                for quest in &data.quests.active_quests {
                    let quest_message =
                        ServerMessage::QuestUpdateEvent(shared::protocol::QuestUpdateEvent {
                            player_id,
                            quest_id: quest.id,
                            status: quest.status.clone(),
                        });
                    if let Ok(payload) = encode_server_message(&quest_message) {
                        let _ = network.socket.send_to(&payload, address);
                    }
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
    entities: Query<(
        &NetworkEntity,
        &MapId,
        &Position,
        Option<&Health>,
        Option<&CharacterClass>,
        Option<&PlayerCharacter>,
    )>,
) {
    let Some(network) = network else {
        return;
    };

    for (&address, session) in &network.sessions {
        if !session.logged_in {
            continue;
        }
        let Some(player_entity) = session.entity else {
            continue;
        };
        let Ok((_, player_map, _, _, _, _)) = entities.get(player_entity) else {
            continue;
        };
        for (network_entity, entity_map, position, health, class, _) in &entities {
            if entity_map.0 != player_map.0 {
                continue;
            }
            let (health_current, health_max, alive) = if let Some(health) = health {
                (health.current, health.max, health.current > 0)
            } else {
                (0, 0, true)
            };
            let message = ServerMessage::EntityState(EntityState {
                entity_id: network_entity.id,
                kind: network_entity.kind,
                class: class.copied().unwrap_or_default(),
                map_id: entity_map.0.clone(),
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
    mut penalty_events: MessageReader<combat::PlayerDeathPenaltyMessage>,
) {
    let Some(network) = network else {
        return;
    };

    let mut penalty_by_player = HashMap::new();
    for penalty in penalty_events.read() {
        penalty_by_player.insert(penalty.player_id, penalty.exp_lost);
    }

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
            exp_lost: death
                .exp_lost
                .or_else(|| penalty_by_player.get(&death.target_id).copied()),
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

pub fn broadcast_system_notices(
    network: Option<Res<ServerNetwork>>,
    mut notices: MessageReader<combat::SystemNoticeMessage>,
) {
    let Some(network) = network else {
        return;
    };

    for notice in notices.read() {
        let message = ServerMessage::SystemNotice(SystemNotice {
            player_id: notice.player_id,
            text: notice.text.clone(),
        });
        let Ok(payload) = encode_server_message(&message) else {
            continue;
        };

        for (&address, session) in &network.sessions {
            if session.logged_in && session.player_id == Some(notice.player_id) {
                let _ = network.socket.send_to(&payload, address);
            }
        }
    }
}

pub fn broadcast_item_events(
    network: Option<Res<ServerNetwork>>,
    players: Query<&MapId, With<PlayerCharacter>>,
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
            let Some(player_entity) = session.entity else {
                continue;
            };
            let Ok(player_map) = players.get(player_entity) else {
                continue;
            };
            if session.logged_in && player_map.0 == spawned.map_id {
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
            let Some(player_entity) = session.entity else {
                continue;
            };
            let Ok(player_map) = players.get(player_entity) else {
                continue;
            };
            if session.logged_in && player_map.0 == despawned.map_id {
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

pub fn broadcast_dialog_events(
    network: Option<Res<ServerNetwork>>,
    mut dialog_events: MessageReader<npc::DialogueMessage>,
) {
    let Some(network) = network else {
        return;
    };

    for dialog in dialog_events.read() {
        let message = ServerMessage::DialogueResponse(npc::to_dialogue_response(dialog));
        let Ok(payload) = encode_server_message(&message) else {
            continue;
        };
        for (&address, session) in &network.sessions {
            if session.logged_in && session.player_id == Some(dialog.player_id) {
                let _ = network.socket.send_to(&payload, address);
            }
        }
    }
}

pub fn broadcast_quest_events(
    network: Option<Res<ServerNetwork>>,
    mut quest_events: MessageReader<quest::QuestUpdatedMessage>,
) {
    let Some(network) = network else {
        return;
    };

    for changed in quest_events.read() {
        let message = ServerMessage::QuestUpdateEvent(shared::protocol::QuestUpdateEvent {
            player_id: changed.player_id,
            quest_id: changed.quest_id,
            status: changed.status.clone(),
        });
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

pub fn broadcast_map_change_events(
    network: Option<Res<ServerNetwork>>,
    mut map_changed: MessageReader<interaction::MapChangedMessage>,
) {
    let Some(network) = network else {
        return;
    };

    for changed in map_changed.read() {
        let message = ServerMessage::MapChangeEvent(shared::protocol::MapChangeEvent {
            map_id: changed.map_id.clone(),
            x: changed.x,
            y: changed.y,
        });
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

pub fn broadcast_chat_events(
    network: Option<Res<ServerNetwork>>,
    mut chat_events: MessageReader<chat::ChatDelivery>,
) {
    let Some(network) = network else {
        return;
    };

    for chat in chat_events.read() {
        let message = ServerMessage::ChatEvent(chat.event.clone());
        let Ok(payload) = encode_server_message(&message) else {
            continue;
        };
        for (&address, session) in &network.sessions {
            if session.logged_in && session.player_id == Some(chat.recipient_player_id) {
                let _ = network.socket.send_to(&payload, address);
            }
        }
    }
}

pub fn broadcast_status_effect_events(
    network: Option<Res<ServerNetwork>>,
    mut status_events: MessageReader<combat::StatusEffectsChangedMessage>,
) {
    let Some(network) = network else {
        return;
    };

    for status in status_events.read() {
        let message = ServerMessage::StatusEffectUpdate(StatusEffectUpdate {
            player_id: status.player_id,
            effects: status.effects.clone(),
        });
        let Ok(payload) = encode_server_message(&message) else {
            continue;
        };
        for (&address, session) in &network.sessions {
            if session.logged_in && session.player_id == Some(status.player_id) {
                let _ = network.socket.send_to(&payload, address);
            }
        }
    }
}

pub fn broadcast_progression_events(
    network: Option<Res<ServerNetwork>>,
    mut exp_events: MessageReader<combat::ExpChangedMessage>,
    mut level_up_events: MessageReader<combat::LevelUpMessage>,
) {
    let Some(network) = network else {
        return;
    };

    for exp in exp_events.read() {
        let message = ServerMessage::ExpUpdateEvent(shared::protocol::ExpUpdateEvent {
            player_id: exp.player_id,
            level: exp.level,
            exp_current: exp.exp_current,
            exp_next: exp.exp_next,
            str_stat: exp.str_stat,
            dex: exp.dex,
            int_stat: exp.int_stat,
            con: exp.con,
        });
        let Ok(payload) = encode_server_message(&message) else {
            continue;
        };
        for (&address, session) in &network.sessions {
            if session.logged_in && session.player_id == Some(exp.player_id) {
                let _ = network.socket.send_to(&payload, address);
            }
        }
    }

    for level_up in level_up_events.read() {
        let message = ServerMessage::LevelUpEvent(shared::protocol::LevelUpEvent {
            player_id: level_up.player_id,
            new_level: level_up.new_level,
            health_max: level_up.health_max,
            mana_max: level_up.mana_max,
        });
        let Ok(payload) = encode_server_message(&message) else {
            continue;
        };
        for (&address, session) in &network.sessions {
            if session.logged_in && session.player_id == Some(level_up.player_id) {
                let _ = network.socket.send_to(&payload, address);
            }
        }
    }
}
