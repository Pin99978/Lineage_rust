CREATE TABLE IF NOT EXISTS guilds (
    name             TEXT PRIMARY KEY,
    leader_username  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS guild_members (
    guild_name  TEXT NOT NULL,
    username    TEXT NOT NULL,
    role        TEXT NOT NULL DEFAULT 'Member',
    PRIMARY KEY (guild_name, username)
);
