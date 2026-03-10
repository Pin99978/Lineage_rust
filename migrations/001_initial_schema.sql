CREATE TABLE IF NOT EXISTS users (
    username            TEXT        PRIMARY KEY,
    x                   REAL        NOT NULL DEFAULT -300.0,
    y                   REAL        NOT NULL DEFAULT 0.0,
    health_current      INTEGER     NOT NULL DEFAULT 100,
    health_max          INTEGER     NOT NULL DEFAULT 100,
    mana_current        INTEGER     NOT NULL DEFAULT 60,
    mana_max            INTEGER     NOT NULL DEFAULT 60,
    level               INTEGER     NOT NULL DEFAULT 1,
    exp_current         INTEGER     NOT NULL DEFAULT 0,
    exp_next            INTEGER     NOT NULL DEFAULT 100,
    str_stat            INTEGER     NOT NULL DEFAULT 15,
    dex                 INTEGER     NOT NULL DEFAULT 15,
    int_stat            INTEGER     NOT NULL DEFAULT 15,
    con                 INTEGER     NOT NULL DEFAULT 15,
    class               TEXT        NOT NULL DEFAULT 'Knight',
    guild_name          TEXT,
    guild_role          TEXT,
    known_spells_json   JSONB       NOT NULL DEFAULT '[]',
    inventory_json      JSONB       NOT NULL DEFAULT '{}',
    equipment_json      JSONB       NOT NULL DEFAULT '{"weapon":null,"armor":null}',
    pk_count            INTEGER     NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS quests (
    username    TEXT    NOT NULL,
    quest_id    TEXT    NOT NULL,
    status_json JSONB   NOT NULL,
    PRIMARY KEY (username, quest_id)
);
