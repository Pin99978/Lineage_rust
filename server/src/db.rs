use bevy::prelude::*;
use crossbeam_channel::{unbounded, Receiver, Sender};
use rusqlite::{params, Connection};
use shared::{EquipmentMap, Inventory};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

use crate::network;

const DB_PATH: &str = "data.db";

#[derive(Debug, Clone)]
pub struct PersistedPlayer {
    pub username: String,
    pub x: f32,
    pub y: f32,
    pub health_current: i32,
    pub health_max: i32,
    pub mana_current: i32,
    pub mana_max: i32,
    pub inventory: Inventory,
    pub equipment: EquipmentMap,
}

#[derive(Debug)]
pub enum DbCommand {
    LoadOrCreate {
        address: std::net::SocketAddr,
        username: String,
    },
    SavePlayer {
        data: PersistedPlayer,
    },
}

#[derive(Debug)]
pub enum DbResult {
    PlayerLoaded {
        address: std::net::SocketAddr,
        data: PersistedPlayer,
    },
    LoginFailed {
        address: std::net::SocketAddr,
        message: String,
    },
    SaveError {
        username: String,
        message: String,
    },
}

#[derive(Resource)]
pub struct DbBridge {
    pub command_tx: Sender<DbCommand>,
    pub result_rx: Receiver<DbResult>,
}

#[derive(Resource)]
pub struct SaveTick {
    pub timer: Timer,
}

pub fn setup_db(mut commands: Commands) {
    let (command_tx, command_rx) = unbounded::<DbCommand>();
    let (result_tx, result_rx) = unbounded::<DbResult>();

    thread::spawn(move || db_worker_loop(command_rx, result_tx));

    commands.insert_resource(DbBridge {
        command_tx,
        result_rx,
    });
    commands.insert_resource(SaveTick {
        timer: Timer::from_seconds(8.0, TimerMode::Repeating),
    });
}

fn db_worker_loop(command_rx: Receiver<DbCommand>, result_tx: Sender<DbResult>) {
    let conn = match Connection::open(DB_PATH) {
        Ok(conn) => conn,
        Err(error) => {
            error!("database open failed: {}", error);
            return;
        }
    };

    if let Err(error) = conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            username TEXT PRIMARY KEY,
            x REAL NOT NULL,
            y REAL NOT NULL,
            health_current INTEGER NOT NULL,
            health_max INTEGER NOT NULL,
            mana_current INTEGER NOT NULL DEFAULT 60,
            mana_max INTEGER NOT NULL DEFAULT 60,
            inventory_json TEXT NOT NULL,
            equipment_json TEXT NOT NULL DEFAULT '{"weapon":null,"armor":null}'
        );
        "#,
    ) {
        error!("database migration failed: {}", error);
        return;
    }
    if let Err(error) = ensure_column(
        &conn,
        "users",
        "mana_current",
        "ALTER TABLE users ADD COLUMN mana_current INTEGER NOT NULL DEFAULT 60",
    ) {
        error!("database migration failed: {}", error);
        return;
    }
    if let Err(error) = ensure_column(
        &conn,
        "users",
        "mana_max",
        "ALTER TABLE users ADD COLUMN mana_max INTEGER NOT NULL DEFAULT 60",
    ) {
        error!("database migration failed: {}", error);
        return;
    }
    if let Err(error) = ensure_column(
        &conn,
        "users",
        "equipment_json",
        "ALTER TABLE users ADD COLUMN equipment_json TEXT NOT NULL DEFAULT '{\"weapon\":null,\"armor\":null}'",
    ) {
        error!("database migration failed: {}", error);
        return;
    }

    while let Ok(command) = command_rx.recv() {
        match command {
            DbCommand::LoadOrCreate { address, username } => {
                if let Some(player) = load_player(&conn, &username) {
                    let _ = result_tx.send(DbResult::PlayerLoaded {
                        address,
                        data: player,
                    });
                    continue;
                }

                let new_player = PersistedPlayer {
                    username: username.clone(),
                    x: -300.0,
                    y: 0.0,
                    health_current: 100,
                    health_max: 100,
                    mana_current: 60,
                    mana_max: 60,
                    inventory: Inventory::default(),
                    equipment: EquipmentMap::default(),
                };
                if let Err(error) = save_player(&conn, &new_player) {
                    let _ = result_tx.send(DbResult::LoginFailed {
                        address,
                        message: format!("create account failed: {}", error),
                    });
                    continue;
                }
                let _ = result_tx.send(DbResult::PlayerLoaded {
                    address,
                    data: new_player,
                });
            }
            DbCommand::SavePlayer { data } => {
                if let Err(error) = save_player(&conn, &data) {
                    let _ = result_tx.send(DbResult::SaveError {
                        username: data.username,
                        message: error,
                    });
                }
            }
        }

        thread::sleep(Duration::from_millis(1));
    }
}

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    alter_sql: &str,
) -> Result<(), String> {
    let mut statement = conn
        .prepare(&format!("PRAGMA table_info({})", table))
        .map_err(|error| format!("prepare table info failed: {}", error))?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| format!("query table info failed: {}", error))?;
    for row in rows {
        let Ok(name) = row else {
            continue;
        };
        if name == column {
            return Ok(());
        }
    }
    conn.execute(alter_sql, [])
        .map_err(|error| format!("add column {} failed: {}", column, error))?;
    Ok(())
}

fn load_player(conn: &Connection, username: &str) -> Option<PersistedPlayer> {
    let mut statement = conn
        .prepare(
            "SELECT x, y, health_current, health_max, mana_current, mana_max, inventory_json, equipment_json FROM users WHERE username = ?1",
        )
        .ok()?;
    let row = statement
        .query_row(params![username], |row| {
            Ok((
                row.get::<_, f32>(0)?,
                row.get::<_, f32>(1)?,
                row.get::<_, i32>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, i32>(4)?,
                row.get::<_, i32>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
            ))
        })
        .ok()?;

    let inventory_items: HashMap<shared::ItemType, u32> =
        serde_json::from_str(&row.6).unwrap_or_default();
    let equipment = serde_json::from_str(&row.7).unwrap_or_default();
    Some(PersistedPlayer {
        username: username.to_string(),
        x: row.0,
        y: row.1,
        health_current: row.2,
        health_max: row.3,
        mana_current: row.4,
        mana_max: row.5,
        inventory: Inventory {
            items: inventory_items,
        },
        equipment,
    })
}

fn save_player(conn: &Connection, data: &PersistedPlayer) -> Result<(), String> {
    let inventory_json = serde_json::to_string(&data.inventory.items)
        .map_err(|error| format!("serialize inventory failed: {}", error))?;
    let equipment_json = serde_json::to_string(&data.equipment)
        .map_err(|error| format!("serialize equipment failed: {}", error))?;
    conn.execute(
        r#"
        INSERT INTO users (
            username,
            x,
            y,
            health_current,
            health_max,
            mana_current,
            mana_max,
            inventory_json,
            equipment_json
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        ON CONFLICT(username) DO UPDATE SET
            x = excluded.x,
            y = excluded.y,
            health_current = excluded.health_current,
            health_max = excluded.health_max,
            mana_current = excluded.mana_current,
            mana_max = excluded.mana_max,
            inventory_json = excluded.inventory_json,
            equipment_json = excluded.equipment_json
        "#,
        params![
            data.username,
            data.x,
            data.y,
            data.health_current,
            data.health_max,
            data.mana_current,
            data.mana_max,
            inventory_json,
            equipment_json
        ],
    )
    .map_err(|error| format!("save player failed: {}", error))?;
    Ok(())
}

pub fn periodic_save_players(
    time: Res<Time>,
    save_tick: Option<ResMut<SaveTick>>,
    db_bridge: Option<Res<DbBridge>>,
    network: Option<Res<network::ServerNetwork>>,
    players: Query<(
        &network::NetworkEntity,
        &shared::Position,
        &shared::Health,
        &shared::Mana,
        &Inventory,
        &EquipmentMap,
    )>,
) {
    let Some(mut save_tick) = save_tick else {
        return;
    };
    let Some(db_bridge) = db_bridge else {
        return;
    };
    let Some(network) = network else {
        return;
    };

    save_tick.timer.tick(time.delta());
    if !save_tick.timer.just_finished() {
        return;
    }

    for session in network.sessions.values() {
        let (Some(username), Some(entity), true) =
            (session.username.as_ref(), session.entity, session.logged_in)
        else {
            continue;
        };
        let Ok((_network_entity, position, health, mana, inventory, equipment)) =
            players.get(entity)
        else {
            continue;
        };
        let data = PersistedPlayer {
            username: username.clone(),
            x: position.x,
            y: position.y,
            health_current: health.current,
            health_max: health.max,
            mana_current: mana.current,
            mana_max: mana.max,
            inventory: inventory.clone(),
            equipment: equipment.clone(),
        };
        let _ = db_bridge.command_tx.send(DbCommand::SavePlayer { data });
    }
}
