use bevy::prelude::*;
use shared::protocol::{
    decode_server_message, encode_client_message, AttackIntent, CastSpellIntent, ChatIntent,
    ClientMessage, EntityState, EquipIntent, InteractNpcIntent, LoginRequest, LootIntent,
    NetworkEntityKind, ServerMessage, UnequipIntent, UseItemIntent,
};
use shared::{CharacterClass, EquipmentSlot, Health, ItemType, Position, SpellType, MAP_TOWN};
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
    pub map_id: String,
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

#[derive(Component)]
pub struct Lootable;

#[derive(Component)]
pub struct NpcInteractable;

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
    commands.insert_resource(LocalPlayer {
        id: None,
        map_id: MAP_TOWN.to_string(),
    });
    commands.insert_resource(NetworkEntityMap::default());
}

pub fn send_move_intent(network: &ClientNetwork, intent: shared::protocol::MoveIntent) {
    send_to_server(network, &ClientMessage::MoveIntent(intent));
}

pub fn send_login_request(network: &ClientNetwork, request: LoginRequest) {
    send_to_server(network, &ClientMessage::LoginRequest(request));
}

pub fn send_attack_intent(network: &ClientNetwork, intent: AttackIntent) {
    send_to_server(network, &ClientMessage::AttackIntent(intent));
}

pub fn send_loot_intent(network: &ClientNetwork, intent: LootIntent) {
    send_to_server(network, &ClientMessage::LootIntent(intent));
}

pub fn send_cast_spell_intent(network: &ClientNetwork, intent: CastSpellIntent) {
    send_to_server(network, &ClientMessage::CastSpellIntent(intent));
}

pub fn send_equip_intent(network: &ClientNetwork, intent: EquipIntent) {
    send_to_server(network, &ClientMessage::EquipIntent(intent));
}

pub fn send_unequip_intent(network: &ClientNetwork, intent: UnequipIntent) {
    send_to_server(network, &ClientMessage::UnequipIntent(intent));
}

pub fn send_interact_npc_intent(network: &ClientNetwork, intent: InteractNpcIntent) {
    send_to_server(network, &ClientMessage::InteractNpcIntent(intent));
}

pub fn send_chat_intent(network: &ClientNetwork, intent: ChatIntent) {
    send_to_server(network, &ClientMessage::ChatIntent(intent));
}

pub fn send_use_item_intent(network: &ClientNetwork, intent: UseItemIntent) {
    send_to_server(network, &ClientMessage::UseItemIntent(intent));
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
    app_state: Option<Res<State<systems::ui::AppState>>>,
    mut next_state: Option<ResMut<NextState<systems::ui::AppState>>>,
    mut login_name: Option<ResMut<systems::ui::LoginName>>,
    hud_state: Option<ResMut<systems::ui::HudState>>,
    inventory_state: Option<ResMut<systems::ui::inventory::LocalInventoryState>>,
    equipment_state: Option<ResMut<systems::ui::paperdoll::LocalEquipmentState>>,
    local_player: Option<ResMut<LocalPlayer>>,
    entity_map: Option<ResMut<NetworkEntityMap>>,
    mut state_queries: ParamSet<(
        Query<(Entity, &mut Position, &mut Health, Option<&mut Sprite>), With<Player>>,
        Query<
            (
                Entity,
                &NetworkEntityVisual,
                &mut Position,
                &mut Health,
                Option<&mut Sprite>,
                Option<&Attackable>,
                Option<&Lootable>,
                Option<&NpcInteractable>,
            ),
            Without<Player>,
        >,
    )>,
    mut damage_feedback: MessageWriter<systems::combat_render::DamagePopupEvent>,
    mut death_feedback: MessageWriter<systems::combat_render::DeathVisualEvent>,
    mut attack_animation: MessageWriter<systems::animation::PlayAttackAnimation>,
    dialog_state: Option<ResMut<systems::ui::DialogState>>,
    chat_state: Option<ResMut<systems::ui::chat::ChatUiState>>,
) {
    let Some(network) = network else {
        return;
    };
    let Some(app_state) = app_state else {
        return;
    };
    let Some(mut local_player) = local_player else {
        return;
    };
    let Some(mut hud_state) = hud_state else {
        return;
    };
    let Some(mut inventory_state) = inventory_state else {
        return;
    };
    let Some(mut equipment_state) = equipment_state else {
        return;
    };
    let Some(mut entity_map) = entity_map else {
        return;
    };
    let Some(mut dialog_state) = dialog_state else {
        return;
    };
    let Some(mut chat_state) = chat_state else {
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
            ServerMessage::LoginResponse(response) => {
                if response.success {
                    if let Some(next_state) = next_state.as_deref_mut() {
                        next_state.set(systems::ui::AppState::InGame);
                    }
                } else if let Some(login_name) = login_name.as_deref_mut() {
                    login_name.submitted = false;
                    warn!("login failed: {}", response.message);
                }
            }
            ServerMessage::AssignedPlayer { player_id } => {
                local_player.id = Some(player_id);
                local_player.map_id = MAP_TOWN.to_string();
                inventory_state.items.clear();
                equipment_state.weapon = None;
                equipment_state.armor = None;
                hud_state.quest_entries.clear();
                hud_state.level = 1;
                hud_state.exp_current = 0;
                hud_state.exp_next = 100;
                hud_state.class = CharacterClass::Knight;
                hud_state.str_stat = 15;
                hud_state.dex = 15;
                hud_state.int_stat = 15;
                hud_state.con = 15;
                dialog_state.visible = false;
                dialog_state.text.clear();
                dialog_state.choices.clear();
                dialog_state.npc_id = None;
                let mut player_query = state_queries.p0();
                let Ok((player_entity, _, _, _)) = player_query.single_mut() else {
                    continue;
                };
                if let Some(existing) = entity_map.entity_by_id.get(&player_id).copied() {
                    if existing != player_entity {
                        commands.entity(existing).despawn();
                    }
                }
                commands
                    .entity(player_entity)
                    .insert(NetworkEntityVisual { id: player_id });
                entity_map.entity_by_id.insert(player_id, player_entity);
            }
            ServerMessage::EntityState(state) => {
                if *app_state.get() == systems::ui::AppState::LoginMenu {
                    continue;
                }
                if local_player.id != Some(state.entity_id) && state.map_id != local_player.map_id {
                    continue;
                }
                if local_player.id == Some(state.entity_id) {
                    local_player.map_id = state.map_id.clone();
                    hud_state.class = state.class;
                }
                apply_entity_state(
                    &mut commands,
                    &mut local_player,
                    &mut entity_map,
                    &mut state_queries,
                    state,
                );
            }
            ServerMessage::MapChangeEvent(event) => {
                local_player.map_id = event.map_id.clone();
                clear_map_state(&mut commands, &mut entity_map, local_player.id);

                let mut player_query = state_queries.p0();
                if let Ok((_entity, mut position, _health, _sprite)) = player_query.single_mut() {
                    position.x = event.x;
                    position.y = event.y;
                }
            }
            ServerMessage::DamageEvent(event) => {
                damage_feedback.write(systems::combat_render::DamagePopupEvent {
                    target_id: event.target_id,
                    amount: event.amount,
                });
                attack_animation.write(systems::animation::PlayAttackAnimation {
                    target_id: Some(event.target_id),
                    local_player: false,
                });
            }
            ServerMessage::DeathEvent(event) => {
                death_feedback.write(systems::combat_render::DeathVisualEvent {
                    target_id: event.target_id,
                });
            }
            ServerMessage::ItemSpawnEvent(event) => {
                spawn_or_update_item(
                    &mut commands,
                    &mut entity_map,
                    event.item_id,
                    event.x,
                    event.y,
                    event.item_type,
                );
            }
            ServerMessage::ItemDespawnEvent(event) => {
                if let Some(entity) = entity_map.entity_by_id.remove(&event.item_id) {
                    commands.entity(entity).despawn();
                }
            }
            ServerMessage::InventoryUpdate(event) => {
                if local_player.id == Some(event.player_id) {
                    if event.amount == 0 {
                        inventory_state.items.remove(&event.item_type);
                    } else {
                        inventory_state.items.insert(event.item_type, event.amount);
                    }
                    info!("inventory {:?} => {}", event.item_type, event.amount);
                }
            }
            ServerMessage::ManaUpdate(event) => {
                if local_player.id == Some(event.player_id) {
                    hud_state.mana_current = event.current;
                    hud_state.mana_max = event.max;
                    info!("mana: {}/{}", event.current, event.max);
                }
            }
            ServerMessage::ExpUpdateEvent(event) => {
                if local_player.id == Some(event.player_id) {
                    hud_state.level = event.level.max(1);
                    hud_state.exp_current = event.exp_current;
                    hud_state.exp_next = event.exp_next.max(1);
                    hud_state.str_stat = event.str_stat;
                    hud_state.dex = event.dex;
                    hud_state.int_stat = event.int_stat;
                    hud_state.con = event.con;
                }
            }
            ServerMessage::LevelUpEvent(event) => {
                if local_player.id == Some(event.player_id) {
                    hud_state.level = event.new_level.max(1);
                    hud_state.mana_max = event.mana_max.max(1);
                    hud_state.mana_current = event.mana_max.max(1);
                    dialog_state.text = format!("LEVEL UP! Level {}", event.new_level);
                    dialog_state.choices.clear();
                    dialog_state.npc_id = None;
                    dialog_state.visible = true;
                    dialog_state.timer.reset();
                }
            }
            ServerMessage::EquipmentUpdate(event) => {
                if local_player.id == Some(event.player_id) {
                    hud_state.equipment = event.equipment.clone();
                    equipment_state.weapon = event.equipment.weapon;
                    equipment_state.armor = event.equipment.armor;
                    info!(
                        "equipment changed: weapon={:?} armor={:?}",
                        event.equipment.weapon, event.equipment.armor
                    );
                }
            }
            ServerMessage::HealEvent(event) => {
                info!(
                    "heal event: target {} +{} hp (now {})",
                    event.target_id, event.amount, event.resulting_hp
                );
            }
            ServerMessage::DialogEvent(event) => {
                if local_player.id == Some(event.player_id) {
                    dialog_state.text = event.text;
                    dialog_state.choices.clear();
                    dialog_state.npc_id = None;
                    dialog_state.visible = true;
                    dialog_state.timer.reset();
                }
            }
            ServerMessage::DialogueResponse(event) => {
                if local_player.id == Some(event.player_id) {
                    dialog_state.text = event.text;
                    dialog_state.choices = event.choices;
                    dialog_state.npc_id = Some(event.npc_id);
                    dialog_state.visible = true;
                    dialog_state.timer.reset();
                }
            }
            ServerMessage::ChatEvent(event) => {
                let prefix = match event.channel {
                    shared::protocol::ChatChannel::Say => "[Say]",
                    shared::protocol::ChatChannel::Shout => "[Shout]",
                    shared::protocol::ChatChannel::Whisper => "[Whisper]",
                };
                systems::ui::chat::push_history_line(
                    &mut chat_state,
                    format!("{} {}: {}", prefix, event.sender, event.message),
                );
            }
            ServerMessage::StatusEffectUpdate(event) => {
                if local_player.id == Some(event.player_id) {
                    hud_state.status_effects = event.effects;
                }
            }
            ServerMessage::QuestUpdateEvent(event) => {
                if local_player.id == Some(event.player_id) {
                    let next_status = event.status.clone();
                    if let Some((_, status)) = hud_state
                        .quest_entries
                        .iter_mut()
                        .find(|(quest_id, _)| *quest_id == event.quest_id)
                    {
                        *status = next_status;
                    } else {
                        hud_state.quest_entries.push((event.quest_id, next_status));
                    }
                }
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn apply_entity_state(
    commands: &mut Commands,
    local_player: &mut LocalPlayer,
    entity_map: &mut NetworkEntityMap,
    state_queries: &mut ParamSet<(
        Query<(Entity, &mut Position, &mut Health, Option<&mut Sprite>), With<Player>>,
        Query<
            (
                Entity,
                &NetworkEntityVisual,
                &mut Position,
                &mut Health,
                Option<&mut Sprite>,
                Option<&Attackable>,
                Option<&Lootable>,
                Option<&NpcInteractable>,
            ),
            Without<Player>,
        >,
    )>,
    state: EntityState,
) {
    if local_player.id == Some(state.entity_id) {
        let mut player_query = state_queries.p0();
        let Ok((player_entity, mut position, mut health, _sprite)) = player_query.single_mut()
        else {
            return;
        };
        position.x = state.x;
        position.y = state.y;
        health.current = state.health_current;
        health.max = state.health_max;
        entity_map
            .entity_by_id
            .insert(state.entity_id, player_entity);
        return;
    }

    if is_loot_kind(state.kind) {
        spawn_or_update_item(
            commands,
            entity_map,
            state.entity_id,
            state.x,
            state.y,
            match state.kind {
                NetworkEntityKind::LootGold => shared::ItemType::Gold,
                NetworkEntityKind::LootHealthPotion => shared::ItemType::HealthPotion,
                _ => shared::ItemType::Gold,
            },
        );
        return;
    }

    if let Some(existing_entity) = entity_map.entity_by_id.get(&state.entity_id).copied() {
        let mut visuals_query = state_queries.p1();
        if let Ok((_, _, mut position, mut health, _sprite, _, _, _)) =
            visuals_query.get_mut(existing_entity)
        {
            position.x = state.x;
            position.y = state.y;
            health.current = state.health_current;
            health.max = state.health_max;
            return;
        }
        warn!(
            "entity map mismatch: id {} mapped to {:?} but query lookup failed; respawning visual",
            state.entity_id, existing_entity
        );
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
        Sprite::from_color(
            systems::render::color_for_network_kind(state.kind),
            Vec2::splat(32.0),
        ),
        Transform::from_xyz(state.x, state.y, 0.0),
    ));
    if state.kind == NetworkEntityKind::Enemy {
        entity_commands.insert(Attackable);
    } else if state.kind == NetworkEntityKind::NpcMerchant {
        entity_commands.insert(NpcInteractable);
    }
    let spawned = entity_commands.id();
    entity_map.entity_by_id.insert(state.entity_id, spawned);
}

fn spawn_or_update_item(
    commands: &mut Commands,
    entity_map: &mut NetworkEntityMap,
    item_id: u64,
    x: f32,
    y: f32,
    item_type: shared::ItemType,
) {
    let kind = match item_type {
        shared::ItemType::Gold => NetworkEntityKind::LootGold,
        shared::ItemType::HealthPotion => NetworkEntityKind::LootHealthPotion,
        shared::ItemType::BronzeSword => NetworkEntityKind::LootGold,
        shared::ItemType::LeatherArmor => NetworkEntityKind::LootHealthPotion,
    };

    if let Some(entity) = entity_map.entity_by_id.get(&item_id).copied() {
        commands
            .entity(entity)
            .insert(Position { x, y })
            .insert(Transform::from_xyz(x, y, 0.0))
            .insert(Sprite::from_color(
                systems::render::color_for_network_kind(kind),
                Vec2::splat(16.0),
            ))
            .insert(Lootable);
        return;
    }

    let entity = commands
        .spawn((
            NetworkEntityVisual { id: item_id },
            Lootable,
            systems::render::YSortable,
            Position { x, y },
            Health { current: 0, max: 0 },
            Sprite::from_color(
                systems::render::color_for_network_kind(kind),
                Vec2::splat(16.0),
            ),
            Transform::from_xyz(x, y, 0.0),
        ))
        .id();
    entity_map.entity_by_id.insert(item_id, entity);
}

pub fn cast_spell_by_hotkey(network: &ClientNetwork, spell: SpellType, target_id: Option<u64>) {
    send_cast_spell_intent(network, CastSpellIntent { spell, target_id });
}

pub fn equip_item_by_hotkey(network: &ClientNetwork, item_type: ItemType) {
    send_equip_intent(network, EquipIntent { item_type });
}

pub fn unequip_slot_by_hotkey(network: &ClientNetwork, slot: EquipmentSlot) {
    send_unequip_intent(network, UnequipIntent { slot });
}

fn is_loot_kind(kind: NetworkEntityKind) -> bool {
    matches!(
        kind,
        NetworkEntityKind::LootGold | NetworkEntityKind::LootHealthPotion
    )
}

fn clear_map_state(
    commands: &mut Commands,
    entity_map: &mut NetworkEntityMap,
    keep_id: Option<u64>,
) {
    let to_clear: Vec<u64> = entity_map
        .entity_by_id
        .keys()
        .copied()
        .filter(|id| Some(*id) != keep_id)
        .collect();
    for id in to_clear {
        if let Some(entity) = entity_map.entity_by_id.remove(&id) {
            commands.entity(entity).despawn();
        }
    }
}
