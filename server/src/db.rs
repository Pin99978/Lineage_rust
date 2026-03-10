use bevy::prelude::*;
use crossbeam_channel::{unbounded, Receiver, Sender};
use rusqlite::{params, Connection};
use shared::{
    class_def, BaseStats, CharacterClass, EquipmentMap, Experience, Inventory, Level, QuestTracker,
};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

use crate::network;

const DB_PATH: &str = "data.db";

#[derive(Debug, Clone)]
pub struct PersistedPlayer {
    pub username: String,
    pub class: CharacterClass,
    pub x: f32,
    pub y: f32,
    pub health_current: i32,
    pub health_max: i32,
    pub mana_current: i32,
    pub mana_max: i32,
    pub level: u32,
    pub exp_current: u32,
    pub exp_next: u32,
    pub base_stats: BaseStats,
    pub inventory: Inventory,
    pub equipment: EquipmentMap,
    pub quests: QuestTracker,
}

#[derive(Debug)]
pub enum DbCommand {
    LoadOrCreate {
        address: std::net::SocketAddr,
        username: String,
        class: CharacterClass,
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
            level INTEGER NOT NULL DEFAULT 1,
            exp_current INTEGER NOT NULL DEFAULT 0,
            exp_next INTEGER NOT NULL DEFAULT 100,
            str_stat INTEGER NOT NULL DEFAULT 15,
            dex INTEGER NOT NULL DEFAULT 15,
            int_stat INTEGER NOT NULL DEFAULT 15,
            con INTEGER NOT NULL DEFAULT 15,
            class TEXT NOT NULL DEFAULT 'Knight',
            inventory_json TEXT NOT NULL,
            equipment_json TEXT NOT NULL DEFAULT '{"weapon":null,"armor":null}'
        );
        CREATE TABLE IF NOT EXISTS quests (
            username TEXT NOT NULL,
            quest_id TEXT NOT NULL,
            status_json TEXT NOT NULL,
            PRIMARY KEY(username, quest_id)
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
    if let Err(error) = ensure_column(
        &conn,
        "users",
        "level",
        "ALTER TABLE users ADD COLUMN level INTEGER NOT NULL DEFAULT 1",
    ) {
        error!("database migration failed: {}", error);
        return;
    }
    if let Err(error) = ensure_column(
        &conn,
        "users",
        "exp_current",
        "ALTER TABLE users ADD COLUMN exp_current INTEGER NOT NULL DEFAULT 0",
    ) {
        error!("database migration failed: {}", error);
        return;
    }
    if let Err(error) = ensure_column(
        &conn,
        "users",
        "exp_next",
        "ALTER TABLE users ADD COLUMN exp_next INTEGER NOT NULL DEFAULT 100",
    ) {
        error!("database migration failed: {}", error);
        return;
    }
    if let Err(error) = ensure_column(
        &conn,
        "users",
        "class",
        "ALTER TABLE users ADD COLUMN class TEXT NOT NULL DEFAULT 'Knight'",
    ) {
        error!("database migration failed: {}", error);
        return;
    }
    if let Err(error) = ensure_column(
        &conn,
        "users",
        "str_stat",
        "ALTER TABLE users ADD COLUMN str_stat INTEGER NOT NULL DEFAULT 15",
    ) {
        error!("database migration failed: {}", error);
        return;
    }
    if let Err(error) = ensure_column(
        &conn,
        "users",
        "dex",
        "ALTER TABLE users ADD COLUMN dex INTEGER NOT NULL DEFAULT 15",
    ) {
        error!("database migration failed: {}", error);
        return;
    }
    if let Err(error) = ensure_column(
        &conn,
        "users",
        "int_stat",
        "ALTER TABLE users ADD COLUMN int_stat INTEGER NOT NULL DEFAULT 15",
    ) {
        error!("database migration failed: {}", error);
        return;
    }
    if let Err(error) = ensure_column(
        &conn,
        "users",
        "con",
        "ALTER TABLE users ADD COLUMN con INTEGER NOT NULL DEFAULT 15",
    ) {
        error!("database migration failed: {}", error);
        return;
    }

    while let Ok(command) = command_rx.recv() {
        match command {
            DbCommand::LoadOrCreate {
                address,
                username,
                class,
            } => {
                if let Some(player) = load_player(&conn, &username) {
                    let _ = result_tx.send(DbResult::PlayerLoaded {
                        address,
                        data: player,
                    });
                    continue;
                }

                let definition = class_def(class);
                let new_player = PersistedPlayer {
                    username: username.clone(),
                    class,
                    x: -300.0,
                    y: 0.0,
                    health_current: definition.base_hp,
                    health_max: definition.base_hp,
                    mana_current: definition.base_mp,
                    mana_max: definition.base_mp,
                    level: 1,
                    exp_current: 0,
                    exp_next: shared::experience_required_for_level(1),
                    base_stats: BaseStats {
                        str_stat: definition.base_str,
                        dex: definition.base_dex,
                        int_stat: definition.base_int,
                        con: definition.base_con,
                    },
                    inventory: Inventory::default(),
                    equipment: EquipmentMap::default(),
                    quests: QuestTracker::default(),
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
            "SELECT x, y, health_current, health_max, mana_current, mana_max, level, exp_current, exp_next, str_stat, dex, int_stat, con, class, inventory_json, equipment_json FROM users WHERE username = ?1",
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
                row.get::<_, u32>(6)?,
                row.get::<_, u32>(7)?,
                row.get::<_, u32>(8)?,
                row.get::<_, u32>(9)?,
                row.get::<_, u32>(10)?,
                row.get::<_, u32>(11)?,
                row.get::<_, u32>(12)?,
                row.get::<_, String>(13)?,
                row.get::<_, String>(14)?,
                row.get::<_, String>(15)?,
            ))
        })
        .ok()?;

    let inventory_items: HashMap<shared::ItemType, u32> =
        serde_json::from_str(&row.14).unwrap_or_default();
    let equipment = serde_json::from_str(&row.15).unwrap_or_default();
    let quests = load_quests(conn, username);
    Some(PersistedPlayer {
        username: username.to_string(),
        class: CharacterClass::from_str(&row.13).unwrap_or_default(),
        x: row.0,
        y: row.1,
        health_current: row.2,
        health_max: row.3,
        mana_current: row.4,
        mana_max: row.5,
        level: row.6.max(1),
        exp_current: row.7,
        exp_next: row.8.max(1),
        base_stats: BaseStats {
            str_stat: row.9,
            dex: row.10,
            int_stat: row.11,
            con: row.12,
        },
        inventory: Inventory {
            items: inventory_items,
        },
        equipment,
        quests,
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
            level,
            exp_current,
            exp_next,
            str_stat,
            dex,
            int_stat,
            con,
            class,
            inventory_json,
            equipment_json
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
        ON CONFLICT(username) DO UPDATE SET
            x = excluded.x,
            y = excluded.y,
            health_current = excluded.health_current,
            health_max = excluded.health_max,
            mana_current = excluded.mana_current,
            mana_max = excluded.mana_max,
            level = excluded.level,
            exp_current = excluded.exp_current,
            exp_next = excluded.exp_next,
            str_stat = excluded.str_stat,
            dex = excluded.dex,
            int_stat = excluded.int_stat,
            con = excluded.con,
            class = excluded.class,
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
            data.level,
            data.exp_current,
            data.exp_next,
            data.base_stats.str_stat,
            data.base_stats.dex,
            data.base_stats.int_stat,
            data.base_stats.con,
            data.class.as_str(),
            inventory_json,
            equipment_json
        ],
    )
    .map_err(|error| format!("save player failed: {}", error))?;
    save_quests(conn, &data.username, &data.quests)?;
    Ok(())
}

fn load_quests(conn: &Connection, username: &str) -> QuestTracker {
    let mut statement =
        match conn.prepare("SELECT quest_id, status_json FROM quests WHERE username = ?1") {
            Ok(stmt) => stmt,
            Err(_) => return QuestTracker::default(),
        };

    let rows = match statement.query_map(params![username], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }) {
        Ok(rows) => rows,
        Err(_) => return QuestTracker::default(),
    };

    let mut tracker = QuestTracker::default();
    for row in rows.flatten() {
        let quest_id = match row.0.as_str() {
            "KillSlimes" => shared::QuestId::KillSlimes,
            _ => continue,
        };
        let status = serde_json::from_str::<shared::QuestStatus>(&row.1)
            .unwrap_or(shared::QuestStatus::NotStarted);
        tracker.active_quests.push(shared::QuestEntry {
            id: quest_id,
            status,
        });
    }
    tracker
}

fn save_quests(conn: &Connection, username: &str, tracker: &QuestTracker) -> Result<(), String> {
    conn.execute("DELETE FROM quests WHERE username = ?1", params![username])
        .map_err(|error| format!("clear quests failed: {}", error))?;

    for entry in &tracker.active_quests {
        let quest_id = match entry.id {
            shared::QuestId::KillSlimes => "KillSlimes",
        };
        let status_json = serde_json::to_string(&entry.status)
            .map_err(|error| format!("serialize quest status failed: {}", error))?;
        conn.execute(
            "INSERT INTO quests (username, quest_id, status_json) VALUES (?1, ?2, ?3)",
            params![username, quest_id, status_json],
        )
        .map_err(|error| format!("insert quest failed: {}", error))?;
    }

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
        &Level,
        &Experience,
        &BaseStats,
        &CharacterClass,
        &Inventory,
        &EquipmentMap,
        &QuestTracker,
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
        let Ok((
            _network_entity,
            position,
            health,
            mana,
            level,
            exp,
            base_stats,
            class,
            inventory,
            equipment,
            quests,
        )) = players.get(entity)
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
            level: level.current,
            exp_current: exp.current,
            exp_next: exp.next_level_req,
            base_stats: *base_stats,
            class: *class,
            inventory: inventory.clone(),
            equipment: equipment.clone(),
            quests: quests.clone(),
        };
        let _ = db_bridge.command_tx.send(DbCommand::SavePlayer { data });
    }
}

#[allow(clippy::type_complexity)]
pub fn save_player_progress_on_change(
    db_bridge: Option<Res<DbBridge>>,
    network: Option<Res<network::ServerNetwork>>,
    players: Query<
        (
            &shared::Position,
            &shared::Health,
            &shared::Mana,
            &Level,
            &Experience,
            &BaseStats,
            &CharacterClass,
            &Inventory,
            &EquipmentMap,
            &QuestTracker,
        ),
        Or<(
            Changed<shared::Position>,
            Changed<shared::Health>,
            Changed<shared::Mana>,
            Changed<Level>,
            Changed<Experience>,
            Changed<BaseStats>,
            Changed<CharacterClass>,
            Changed<Inventory>,
            Changed<EquipmentMap>,
            Changed<QuestTracker>,
        )>,
    >,
) {
    let Some(db_bridge) = db_bridge else {
        return;
    };
    let Some(network) = network else {
        return;
    };

    for session in network.sessions.values() {
        let (Some(username), Some(entity), true) =
            (session.username.as_ref(), session.entity, session.logged_in)
        else {
            continue;
        };
        let Ok((
            position,
            health,
            mana,
            level,
            exp,
            base_stats,
            class,
            inventory,
            equipment,
            quests,
        )) = players.get(entity)
        else {
            continue;
        };
        let _ = db_bridge.command_tx.send(DbCommand::SavePlayer {
            data: PersistedPlayer {
                username: username.clone(),
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
                class: *class,
                inventory: inventory.clone(),
                equipment: equipment.clone(),
                quests: quests.clone(),
            },
        });
    }
}
