use bevy::prelude::*;
use crossbeam_channel::{unbounded, Receiver, Sender};
use shared::{
    class_def, BaseStats, CharacterClass, EquipmentMap, Experience, GuildMembership, GuildRole,
    Inventory, KnownSpells, Level, QuestTracker, SpellType,
};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::collections::HashMap;
use std::thread;

use crate::network;

const DEFAULT_DATABASE_URL: &str = "postgres://postgres:postgres@localhost:5432/lineage";

#[derive(Debug, Clone)]
pub struct PersistedPlayer {
    pub username: String,
    pub class: CharacterClass,
    pub guild_name: Option<String>,
    pub guild_role: Option<GuildRole>,
    pub known_spells: KnownSpells,
    pub pk_count: u32,
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
    CreateGuild {
        username: String,
        guild_name: String,
    },
    JoinGuild {
        username: String,
        guild_name: String,
        role: GuildRole,
    },
    LeaveGuild {
        username: String,
    },
    DisbandGuild {
        username: String,
    },
    QueryGuildMembers {
        username: String,
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
    GuildCreated {
        username: String,
        guild_name: String,
        role: GuildRole,
    },
    GuildJoined {
        username: String,
        guild_name: String,
        role: GuildRole,
    },
    GuildLeft {
        username: String,
    },
    GuildDisbanded {
        username: String,
        guild_name: String,
    },
    GuildMembers {
        username: String,
        guild_name: Option<String>,
        members: Vec<String>,
    },
    GuildOpError {
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

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());

    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime failed");
        runtime.block_on(db_worker_loop(database_url, command_rx, result_tx));
    });

    commands.insert_resource(DbBridge {
        command_tx,
        result_rx,
    });
    commands.insert_resource(SaveTick {
        timer: Timer::from_seconds(8.0, TimerMode::Repeating),
    });
}

async fn db_worker_loop(
    database_url: String,
    command_rx: Receiver<DbCommand>,
    result_tx: Sender<DbResult>,
) {
    let pool = match PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
    {
        Ok(pool) => pool,
        Err(error) => {
            error!("PostgreSQL connect failed: {}", error);
            return;
        }
    };

    if let Err(error) = run_migrations(&pool).await {
        error!("PostgreSQL migration failed: {}", error);
        return;
    }

    while let Ok(command) = command_rx.recv() {
        match command {
            DbCommand::LoadOrCreate {
                address,
                username,
                class,
            } => {
                match load_player(&pool, &username).await {
                    Ok(Some(player)) => {
                        let _ = result_tx.send(DbResult::PlayerLoaded {
                            address,
                            data: player,
                        });
                    }
                    Ok(None) => {
                        let definition = class_def(class);
                        let new_player = PersistedPlayer {
                            username: username.clone(),
                            class,
                            guild_name: None,
                            guild_role: None,
                            known_spells: initial_known_spells_for_class(class),
                            pk_count: 0,
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

                        if let Err(error) = save_player(&pool, &new_player).await {
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
                    Err(error) => {
                        let _ = result_tx.send(DbResult::LoginFailed {
                            address,
                            message: error,
                        });
                    }
                }
            }
            DbCommand::SavePlayer { data } => {
                if let Err(error) = save_player(&pool, &data).await {
                    let _ = result_tx.send(DbResult::SaveError {
                        username: data.username,
                        message: error,
                    });
                }
            }
            DbCommand::CreateGuild {
                username,
                guild_name,
            } => match create_guild(&pool, &username, &guild_name).await {
                Ok(()) => {
                    let _ = result_tx.send(DbResult::GuildCreated {
                        username,
                        guild_name,
                        role: GuildRole::Leader,
                    });
                }
                Err(message) => {
                    let _ = result_tx.send(DbResult::GuildOpError { username, message });
                }
            },
            DbCommand::JoinGuild {
                username,
                guild_name,
                role,
            } => match join_guild(&pool, &username, &guild_name, role).await {
                Ok(()) => {
                    let _ = result_tx.send(DbResult::GuildJoined {
                        username,
                        guild_name,
                        role,
                    });
                }
                Err(message) => {
                    let _ = result_tx.send(DbResult::GuildOpError { username, message });
                }
            },
            DbCommand::LeaveGuild { username } => match leave_guild(&pool, &username).await {
                Ok(()) => {
                    let _ = result_tx.send(DbResult::GuildLeft { username });
                }
                Err(message) => {
                    let _ = result_tx.send(DbResult::GuildOpError { username, message });
                }
            },
            DbCommand::DisbandGuild { username } => {
                match disband_guild(&pool, &username).await {
                    Ok(guild_name) => {
                        let _ = result_tx.send(DbResult::GuildDisbanded {
                            username,
                            guild_name,
                        });
                    }
                    Err(message) => {
                        let _ = result_tx.send(DbResult::GuildOpError { username, message });
                    }
                }
            }
            DbCommand::QueryGuildMembers { username } => {
                let (guild_name, members) = query_guild_members(&pool, &username).await;
                let _ = result_tx.send(DbResult::GuildMembers {
                    username,
                    guild_name,
                    members,
                });
            }
        }
    }
}

async fn run_migrations(pool: &PgPool) -> Result<(), String> {
    sqlx::query(include_str!("../../migrations/001_initial_schema.sql"))
        .execute(pool)
        .await
        .map_err(|error| error.to_string())?;

    sqlx::query(include_str!("../../migrations/002_add_guilds.sql"))
        .execute(pool)
        .await
        .map_err(|error| error.to_string())?;

    Ok(())
}

fn initial_known_spells_for_class(class: CharacterClass) -> KnownSpells {
    let spells = match class {
        CharacterClass::Knight => Vec::new(),
        CharacterClass::Wizard | CharacterClass::Prince | CharacterClass::DarkElf => {
            vec![SpellType::Fireball]
        }
        CharacterClass::Elf => vec![SpellType::Heal],
    };
    KnownSpells { spells }
}

async fn load_player(pool: &PgPool, username: &str) -> Result<Option<PersistedPlayer>, String> {
    let row = sqlx::query(
        r#"
        SELECT x, y, health_current, health_max, mana_current, mana_max,
               level, exp_current, exp_next,
               str_stat, dex, int_stat, con,
               class, guild_name, guild_role,
               known_spells_json, inventory_json, equipment_json, pk_count
        FROM users
        WHERE username = $1
        "#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await
    .map_err(|error| format!("query player failed: {}", error))?;

    let Some(row) = row else {
        return Ok(None);
    };

    let class_text: String = row
        .try_get("class")
        .map_err(|error| format!("read class failed: {}", error))?;
    let guild_name: Option<String> = row
        .try_get("guild_name")
        .map_err(|error| format!("read guild_name failed: {}", error))?;
    let guild_role_text: Option<String> = row
        .try_get("guild_role")
        .map_err(|error| format!("read guild_role failed: {}", error))?;

    let known_spells_json: serde_json::Value = row
        .try_get("known_spells_json")
        .map_err(|error| format!("read known_spells_json failed: {}", error))?;
    let inventory_json: serde_json::Value = row
        .try_get("inventory_json")
        .map_err(|error| format!("read inventory_json failed: {}", error))?;
    let equipment_json: serde_json::Value = row
        .try_get("equipment_json")
        .map_err(|error| format!("read equipment_json failed: {}", error))?;

    let inventory_items: HashMap<shared::ItemType, u32> =
        serde_json::from_value(inventory_json).unwrap_or_default();
    let equipment: EquipmentMap = serde_json::from_value(equipment_json).unwrap_or_default();
    let known_spells = KnownSpells {
        spells: serde_json::from_value(known_spells_json).unwrap_or_default(),
    };

    let quests = load_quests(pool, username).await;

    Ok(Some(PersistedPlayer {
        username: username.to_string(),
        class: CharacterClass::from_str(&class_text).unwrap_or_default(),
        guild_name,
        guild_role: guild_role_text.as_deref().and_then(GuildRole::from_str),
        known_spells,
        pk_count: row.try_get::<i32, _>("pk_count").unwrap_or(0).max(0) as u32,
        x: row
            .try_get("x")
            .map_err(|error| format!("read x failed: {}", error))?,
        y: row
            .try_get("y")
            .map_err(|error| format!("read y failed: {}", error))?,
        health_current: row
            .try_get("health_current")
            .map_err(|error| format!("read health_current failed: {}", error))?,
        health_max: row
            .try_get("health_max")
            .map_err(|error| format!("read health_max failed: {}", error))?,
        mana_current: row
            .try_get("mana_current")
            .map_err(|error| format!("read mana_current failed: {}", error))?,
        mana_max: row
            .try_get("mana_max")
            .map_err(|error| format!("read mana_max failed: {}", error))?,
        level: row
            .try_get::<i32, _>("level")
            .unwrap_or(1)
            .max(1) as u32,
        exp_current: row
            .try_get::<i32, _>("exp_current")
            .unwrap_or(0)
            .max(0) as u32,
        exp_next: row
            .try_get::<i32, _>("exp_next")
            .unwrap_or(1)
            .max(1) as u32,
        base_stats: BaseStats {
            str_stat: row
                .try_get::<i32, _>("str_stat")
                .unwrap_or(15)
                .max(1) as u32,
            dex: row
                .try_get::<i32, _>("dex")
                .unwrap_or(15)
                .max(1) as u32,
            int_stat: row
                .try_get::<i32, _>("int_stat")
                .unwrap_or(15)
                .max(1) as u32,
            con: row
                .try_get::<i32, _>("con")
                .unwrap_or(15)
                .max(1) as u32,
        },
        inventory: Inventory {
            items: inventory_items,
        },
        equipment,
        quests,
    }))
}

async fn save_player(pool: &PgPool, data: &PersistedPlayer) -> Result<(), String> {
    let inventory_json =
        serde_json::to_value(&data.inventory.items).map_err(|error| error.to_string())?;
    let equipment_json = serde_json::to_value(&data.equipment).map_err(|error| error.to_string())?;
    let known_spells_json =
        serde_json::to_value(&data.known_spells.spells).map_err(|error| error.to_string())?;

    sqlx::query(
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
            guild_name,
            guild_role,
            known_spells_json,
            inventory_json,
            equipment_json,
            pk_count
        )
        VALUES (
            $1, $2, $3, $4, $5,
            $6, $7, $8, $9, $10,
            $11, $12, $13, $14, $15,
            $16, $17, $18, $19, $20,
            $21
        )
        ON CONFLICT (username)
        DO UPDATE SET
            x = EXCLUDED.x,
            y = EXCLUDED.y,
            health_current = EXCLUDED.health_current,
            health_max = EXCLUDED.health_max,
            mana_current = EXCLUDED.mana_current,
            mana_max = EXCLUDED.mana_max,
            level = EXCLUDED.level,
            exp_current = EXCLUDED.exp_current,
            exp_next = EXCLUDED.exp_next,
            str_stat = EXCLUDED.str_stat,
            dex = EXCLUDED.dex,
            int_stat = EXCLUDED.int_stat,
            con = EXCLUDED.con,
            class = EXCLUDED.class,
            guild_name = EXCLUDED.guild_name,
            guild_role = EXCLUDED.guild_role,
            known_spells_json = EXCLUDED.known_spells_json,
            inventory_json = EXCLUDED.inventory_json,
            equipment_json = EXCLUDED.equipment_json,
            pk_count = EXCLUDED.pk_count
        "#,
    )
    .bind(&data.username)
    .bind(data.x)
    .bind(data.y)
    .bind(data.health_current)
    .bind(data.health_max)
    .bind(data.mana_current)
    .bind(data.mana_max)
    .bind(data.level as i32)
    .bind(data.exp_current as i32)
    .bind(data.exp_next as i32)
    .bind(data.base_stats.str_stat as i32)
    .bind(data.base_stats.dex as i32)
    .bind(data.base_stats.int_stat as i32)
    .bind(data.base_stats.con as i32)
    .bind(data.class.as_str())
    .bind(data.guild_name.clone())
    .bind(data.guild_role.map(|role| role.as_str().to_string()))
    .bind(known_spells_json)
    .bind(inventory_json)
    .bind(equipment_json)
    .bind(data.pk_count as i32)
    .execute(pool)
    .await
    .map_err(|error| format!("save player failed: {}", error))?;

    save_quests(pool, &data.username, &data.quests).await?;
    Ok(())
}

async fn load_quests(pool: &PgPool, username: &str) -> QuestTracker {
    let rows = match sqlx::query(
        "SELECT quest_id, status_json FROM quests WHERE username = $1",
    )
    .bind(username)
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(_) => return QuestTracker::default(),
    };

    let mut tracker = QuestTracker::default();
    for row in rows {
        let Ok(quest_id_text) = row.try_get::<String, _>("quest_id") else {
            continue;
        };
        let quest_id = match quest_id_text.as_str() {
            "KillSlimes" => shared::QuestId::KillSlimes,
            _ => continue,
        };
        let status_json = row
            .try_get::<serde_json::Value, _>("status_json")
            .unwrap_or_else(|_| serde_json::json!(shared::QuestStatus::NotStarted));
        let status =
            serde_json::from_value::<shared::QuestStatus>(status_json).unwrap_or(shared::QuestStatus::NotStarted);

        tracker
            .active_quests
            .push(shared::QuestEntry { id: quest_id, status });
    }

    tracker
}

async fn save_quests(pool: &PgPool, username: &str, tracker: &QuestTracker) -> Result<(), String> {
    sqlx::query("DELETE FROM quests WHERE username = $1")
        .bind(username)
        .execute(pool)
        .await
        .map_err(|error| format!("clear quests failed: {}", error))?;

    for entry in &tracker.active_quests {
        let quest_id = match entry.id {
            shared::QuestId::KillSlimes => "KillSlimes",
        };
        let status_json = serde_json::to_value(&entry.status)
            .map_err(|error| format!("serialize quest status failed: {}", error))?;

        sqlx::query("INSERT INTO quests (username, quest_id, status_json) VALUES ($1, $2, $3)")
            .bind(username)
            .bind(quest_id)
            .bind(status_json)
            .execute(pool)
            .await
            .map_err(|error| format!("insert quest failed: {}", error))?;
    }

    Ok(())
}

async fn create_guild(pool: &PgPool, username: &str, guild_name: &str) -> Result<(), String> {
    let guild_name = guild_name.trim();
    if guild_name.len() < 3 || guild_name.len() > 24 {
        return Err("guild name must be 3-24 chars".to_string());
    }

    let player_row = sqlx::query("SELECT guild_name FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(pool)
        .await
        .map_err(|error| format!("query player guild failed: {}", error))?;
    let Some(player_row) = player_row else {
        return Err("player not found".to_string());
    };
    let already_in_guild: Option<String> = player_row.try_get("guild_name").unwrap_or(None);
    if already_in_guild.is_some() {
        return Err("already in a guild".to_string());
    }

    let exists = sqlx::query("SELECT 1 FROM guilds WHERE name = $1")
        .bind(guild_name)
        .fetch_optional(pool)
        .await
        .map_err(|error| format!("query guild failed: {}", error))?
        .is_some();
    if exists {
        return Err("guild name already exists".to_string());
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|error| format!("begin transaction failed: {}", error))?;

    sqlx::query("INSERT INTO guilds (name, leader_username) VALUES ($1, $2)")
        .bind(guild_name)
        .bind(username)
        .execute(&mut *tx)
        .await
        .map_err(|error| format!("create guild failed: {}", error))?;

    sqlx::query("INSERT INTO guild_members (guild_name, username, role) VALUES ($1, $2, $3)")
        .bind(guild_name)
        .bind(username)
        .bind(GuildRole::Leader.as_str())
        .execute(&mut *tx)
        .await
        .map_err(|error| format!("insert guild member failed: {}", error))?;

    sqlx::query("UPDATE users SET guild_name = $1, guild_role = $2 WHERE username = $3")
        .bind(guild_name)
        .bind(GuildRole::Leader.as_str())
        .bind(username)
        .execute(&mut *tx)
        .await
        .map_err(|error| format!("update user guild failed: {}", error))?;

    tx.commit()
        .await
        .map_err(|error| format!("commit transaction failed: {}", error))?;

    Ok(())
}

async fn join_guild(
    pool: &PgPool,
    username: &str,
    guild_name: &str,
    role: GuildRole,
) -> Result<(), String> {
    let exists = sqlx::query("SELECT 1 FROM guilds WHERE name = $1")
        .bind(guild_name)
        .fetch_optional(pool)
        .await
        .map_err(|error| format!("query guild failed: {}", error))?
        .is_some();
    if !exists {
        return Err("guild does not exist".to_string());
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|error| format!("begin transaction failed: {}", error))?;

    sqlx::query(
        "INSERT INTO guild_members (guild_name, username, role) VALUES ($1, $2, $3)
         ON CONFLICT (guild_name, username) DO UPDATE SET role = EXCLUDED.role",
    )
    .bind(guild_name)
    .bind(username)
    .bind(role.as_str())
    .execute(&mut *tx)
    .await
    .map_err(|error| format!("join guild failed: {}", error))?;

    sqlx::query("UPDATE users SET guild_name = $1, guild_role = $2 WHERE username = $3")
        .bind(guild_name)
        .bind(role.as_str())
        .bind(username)
        .execute(&mut *tx)
        .await
        .map_err(|error| format!("update user guild failed: {}", error))?;

    tx.commit()
        .await
        .map_err(|error| format!("commit transaction failed: {}", error))?;

    Ok(())
}

async fn leave_guild(pool: &PgPool, username: &str) -> Result<(), String> {
    let row = sqlx::query("SELECT guild_name, guild_role FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(pool)
        .await
        .map_err(|error| format!("query player guild failed: {}", error))?;

    let Some(row) = row else {
        return Err("player not found".to_string());
    };

    let guild_name: Option<String> = row.try_get("guild_name").unwrap_or(None);
    let guild_role: Option<String> = row.try_get("guild_role").unwrap_or(None);

    let Some(guild_name) = guild_name else {
        return Err("not in a guild".to_string());
    };

    if guild_role.as_deref() == Some(GuildRole::Leader.as_str()) {
        return Err("leader must disband guild".to_string());
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|error| format!("begin transaction failed: {}", error))?;

    sqlx::query("DELETE FROM guild_members WHERE guild_name = $1 AND username = $2")
        .bind(&guild_name)
        .bind(username)
        .execute(&mut *tx)
        .await
        .map_err(|error| format!("leave guild failed: {}", error))?;

    sqlx::query("UPDATE users SET guild_name = NULL, guild_role = NULL WHERE username = $1")
        .bind(username)
        .execute(&mut *tx)
        .await
        .map_err(|error| format!("update user guild failed: {}", error))?;

    tx.commit()
        .await
        .map_err(|error| format!("commit transaction failed: {}", error))?;

    Ok(())
}

async fn disband_guild(pool: &PgPool, username: &str) -> Result<String, String> {
    let row = sqlx::query("SELECT guild_name, guild_role FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(pool)
        .await
        .map_err(|error| format!("query player guild failed: {}", error))?;

    let Some(row) = row else {
        return Err("player not found".to_string());
    };

    let guild_name: Option<String> = row.try_get("guild_name").unwrap_or(None);
    let guild_role: Option<String> = row.try_get("guild_role").unwrap_or(None);

    let Some(guild_name) = guild_name else {
        return Err("not in a guild".to_string());
    };
    if guild_role.as_deref() != Some(GuildRole::Leader.as_str()) {
        return Err("only leader can disband".to_string());
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|error| format!("begin transaction failed: {}", error))?;

    sqlx::query("DELETE FROM guild_members WHERE guild_name = $1")
        .bind(&guild_name)
        .execute(&mut *tx)
        .await
        .map_err(|error| format!("remove guild members failed: {}", error))?;

    sqlx::query("DELETE FROM guilds WHERE name = $1")
        .bind(&guild_name)
        .execute(&mut *tx)
        .await
        .map_err(|error| format!("delete guild failed: {}", error))?;

    sqlx::query("UPDATE users SET guild_name = NULL, guild_role = NULL WHERE guild_name = $1")
        .bind(&guild_name)
        .execute(&mut *tx)
        .await
        .map_err(|error| format!("clear users guild failed: {}", error))?;

    tx.commit()
        .await
        .map_err(|error| format!("commit transaction failed: {}", error))?;

    Ok(guild_name)
}

async fn query_guild_members(pool: &PgPool, username: &str) -> (Option<String>, Vec<String>) {
    let guild_row = match sqlx::query("SELECT guild_name FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(pool)
        .await
    {
        Ok(row) => row,
        Err(_) => return (None, Vec::new()),
    };

    let Some(guild_row) = guild_row else {
        return (None, Vec::new());
    };

    let guild_name: Option<String> = guild_row.try_get("guild_name").unwrap_or(None);
    let Some(guild_name) = guild_name else {
        return (None, Vec::new());
    };

    let rows = match sqlx::query(
        "SELECT username FROM guild_members WHERE guild_name = $1 ORDER BY role DESC, username ASC",
    )
    .bind(&guild_name)
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(_) => return (Some(guild_name), Vec::new()),
    };

    let mut members = Vec::new();
    for row in rows {
        if let Ok(username) = row.try_get::<String, _>("username") {
            members.push(username);
        }
    }

    (Some(guild_name), members)
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
        Option<&GuildMembership>,
        Option<&KnownSpells>,
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
            guild,
            known_spells,
            inventory,
            equipment,
            quests,
        )) = players.get(entity)
        else {
            continue;
        };
        let data = PersistedPlayer {
            username: username.clone(),
            class: *class,
            guild_name: guild.map(|value| value.guild_name.clone()),
            guild_role: guild.map(|value| value.role),
            known_spells: known_spells.cloned().unwrap_or_default(),
            pk_count: 0,
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
            Option<&GuildMembership>,
            Option<&KnownSpells>,
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
            Changed<GuildMembership>,
            Changed<KnownSpells>,
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
            guild,
            known_spells,
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
                class: *class,
                guild_name: guild.map(|value| value.guild_name.clone()),
                guild_role: guild.map(|value| value.role),
                known_spells: known_spells.cloned().unwrap_or_default(),
                pk_count: 0,
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
                inventory: inventory.clone(),
                equipment: equipment.clone(),
                quests: quests.clone(),
            },
        });
    }
}
