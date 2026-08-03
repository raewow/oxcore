-- PostgreSQL migration: world / base_tables
-- Generated schema DDL only; it contains no reference data.

CREATE TABLE IF NOT EXISTS "area_template" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "map_id" BIGINT NOT NULL DEFAULT '0' CHECK ("map_id" >= 0),
    "zone_id" BIGINT NOT NULL DEFAULT '0' CHECK ("zone_id" >= 0),
    "explore_flag" BIGINT NOT NULL DEFAULT '0' CHECK ("explore_flag" >= 0),
    "flags" BIGINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0),
    "area_level" INTEGER NOT NULL DEFAULT '0',
    "name" VARCHAR(100) NOT NULL DEFAULT '',
    "team" SMALLINT NOT NULL DEFAULT '0' CHECK ("team" >= 0),
    "liquid_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("liquid_type" >= 0),
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "area_template" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "area_template" ADD COLUMN IF NOT EXISTS "map_id" BIGINT NOT NULL DEFAULT '0' CHECK ("map_id" >= 0);
ALTER TABLE IF EXISTS "area_template" ADD COLUMN IF NOT EXISTS "zone_id" BIGINT NOT NULL DEFAULT '0' CHECK ("zone_id" >= 0);
ALTER TABLE IF EXISTS "area_template" ADD COLUMN IF NOT EXISTS "explore_flag" BIGINT NOT NULL DEFAULT '0' CHECK ("explore_flag" >= 0);
ALTER TABLE IF EXISTS "area_template" ADD COLUMN IF NOT EXISTS "flags" BIGINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0);
ALTER TABLE IF EXISTS "area_template" ADD COLUMN IF NOT EXISTS "area_level" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "area_template" ADD COLUMN IF NOT EXISTS "name" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "area_template" ADD COLUMN IF NOT EXISTS "team" SMALLINT NOT NULL DEFAULT '0' CHECK ("team" >= 0);
ALTER TABLE IF EXISTS "area_template" ADD COLUMN IF NOT EXISTS "liquid_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("liquid_type" >= 0);

CREATE TABLE IF NOT EXISTS "areatrigger_bg_entrance" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "name" TEXT,
    "team" BIGINT NOT NULL DEFAULT '0' CHECK ("team" >= 0),
    "bg_template" BIGINT NOT NULL DEFAULT '0' CHECK ("bg_template" >= 0),
    "exit_map" INTEGER NOT NULL DEFAULT '0' CHECK ("exit_map" >= 0),
    "exit_position_x" REAL NOT NULL DEFAULT '0',
    "exit_position_y" REAL NOT NULL DEFAULT '0',
    "exit_position_z" REAL NOT NULL DEFAULT '0',
    "exit_orientation" REAL NOT NULL DEFAULT '0',
    PRIMARY KEY ("id")
);
ALTER TABLE IF EXISTS "areatrigger_bg_entrance" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "areatrigger_bg_entrance" ADD COLUMN IF NOT EXISTS "name" TEXT;
ALTER TABLE IF EXISTS "areatrigger_bg_entrance" ADD COLUMN IF NOT EXISTS "team" BIGINT NOT NULL DEFAULT '0' CHECK ("team" >= 0);
ALTER TABLE IF EXISTS "areatrigger_bg_entrance" ADD COLUMN IF NOT EXISTS "bg_template" BIGINT NOT NULL DEFAULT '0' CHECK ("bg_template" >= 0);
ALTER TABLE IF EXISTS "areatrigger_bg_entrance" ADD COLUMN IF NOT EXISTS "exit_map" INTEGER NOT NULL DEFAULT '0' CHECK ("exit_map" >= 0);
ALTER TABLE IF EXISTS "areatrigger_bg_entrance" ADD COLUMN IF NOT EXISTS "exit_position_x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "areatrigger_bg_entrance" ADD COLUMN IF NOT EXISTS "exit_position_y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "areatrigger_bg_entrance" ADD COLUMN IF NOT EXISTS "exit_position_z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "areatrigger_bg_entrance" ADD COLUMN IF NOT EXISTS "exit_orientation" REAL NOT NULL DEFAULT '0';

CREATE TABLE IF NOT EXISTS "areatrigger_involvedrelation" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "quest" BIGINT NOT NULL DEFAULT '0' CHECK ("quest" >= 0),
    PRIMARY KEY ("id")
);
ALTER TABLE IF EXISTS "areatrigger_involvedrelation" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "areatrigger_involvedrelation" ADD COLUMN IF NOT EXISTS "quest" BIGINT NOT NULL DEFAULT '0' CHECK ("quest" >= 0);

CREATE TABLE IF NOT EXISTS "areatrigger_scripts" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "delay" BIGINT NOT NULL DEFAULT '0' CHECK ("delay" >= 0),
    "priority" SMALLINT NOT NULL DEFAULT '0' CHECK ("priority" >= 0),
    "command" SMALLINT NOT NULL DEFAULT '0' CHECK ("command" >= 0),
    "datalong" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong" >= 0),
    "datalong2" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong2" >= 0),
    "datalong3" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong3" >= 0),
    "datalong4" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong4" >= 0),
    "target_param1" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param1" >= 0),
    "target_param2" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param2" >= 0),
    "target_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("target_type" >= 0),
    "data_flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("data_flags" >= 0),
    "dataint" INTEGER NOT NULL DEFAULT '0',
    "dataint2" INTEGER NOT NULL DEFAULT '0',
    "dataint3" INTEGER NOT NULL DEFAULT '0',
    "dataint4" INTEGER NOT NULL DEFAULT '0',
    "x" REAL NOT NULL DEFAULT '0',
    "y" REAL NOT NULL DEFAULT '0',
    "z" REAL NOT NULL DEFAULT '0',
    "o" REAL NOT NULL DEFAULT '0',
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    "comments" VARCHAR(255) NOT NULL
);
ALTER TABLE IF EXISTS "areatrigger_scripts" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "areatrigger_scripts" ADD COLUMN IF NOT EXISTS "delay" BIGINT NOT NULL DEFAULT '0' CHECK ("delay" >= 0);
ALTER TABLE IF EXISTS "areatrigger_scripts" ADD COLUMN IF NOT EXISTS "priority" SMALLINT NOT NULL DEFAULT '0' CHECK ("priority" >= 0);
ALTER TABLE IF EXISTS "areatrigger_scripts" ADD COLUMN IF NOT EXISTS "command" SMALLINT NOT NULL DEFAULT '0' CHECK ("command" >= 0);
ALTER TABLE IF EXISTS "areatrigger_scripts" ADD COLUMN IF NOT EXISTS "datalong" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong" >= 0);
ALTER TABLE IF EXISTS "areatrigger_scripts" ADD COLUMN IF NOT EXISTS "datalong2" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong2" >= 0);
ALTER TABLE IF EXISTS "areatrigger_scripts" ADD COLUMN IF NOT EXISTS "datalong3" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong3" >= 0);
ALTER TABLE IF EXISTS "areatrigger_scripts" ADD COLUMN IF NOT EXISTS "datalong4" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong4" >= 0);
ALTER TABLE IF EXISTS "areatrigger_scripts" ADD COLUMN IF NOT EXISTS "target_param1" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param1" >= 0);
ALTER TABLE IF EXISTS "areatrigger_scripts" ADD COLUMN IF NOT EXISTS "target_param2" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param2" >= 0);
ALTER TABLE IF EXISTS "areatrigger_scripts" ADD COLUMN IF NOT EXISTS "target_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("target_type" >= 0);
ALTER TABLE IF EXISTS "areatrigger_scripts" ADD COLUMN IF NOT EXISTS "data_flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("data_flags" >= 0);
ALTER TABLE IF EXISTS "areatrigger_scripts" ADD COLUMN IF NOT EXISTS "dataint" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "areatrigger_scripts" ADD COLUMN IF NOT EXISTS "dataint2" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "areatrigger_scripts" ADD COLUMN IF NOT EXISTS "dataint3" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "areatrigger_scripts" ADD COLUMN IF NOT EXISTS "dataint4" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "areatrigger_scripts" ADD COLUMN IF NOT EXISTS "x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "areatrigger_scripts" ADD COLUMN IF NOT EXISTS "y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "areatrigger_scripts" ADD COLUMN IF NOT EXISTS "z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "areatrigger_scripts" ADD COLUMN IF NOT EXISTS "o" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "areatrigger_scripts" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);
ALTER TABLE IF EXISTS "areatrigger_scripts" ADD COLUMN IF NOT EXISTS "comments" VARCHAR(255) NOT NULL;

CREATE TABLE IF NOT EXISTS "areatrigger_tavern" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "name" TEXT,
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    PRIMARY KEY ("id")
);
ALTER TABLE IF EXISTS "areatrigger_tavern" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "areatrigger_tavern" ADD COLUMN IF NOT EXISTS "name" TEXT;
ALTER TABLE IF EXISTS "areatrigger_tavern" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);

CREATE TABLE IF NOT EXISTS "areatrigger_teleport" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "patch" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch" >= 0),
    "name" VARCHAR(64) NOT NULL DEFAULT '',
    "message" VARCHAR(128) NOT NULL DEFAULT '',
    "required_level" SMALLINT NOT NULL DEFAULT '0' CHECK ("required_level" >= 0),
    "required_condition" BIGINT NOT NULL DEFAULT '0' CHECK ("required_condition" >= 0),
    "target_map" INTEGER NOT NULL DEFAULT '0' CHECK ("target_map" >= 0),
    "target_position_x" REAL NOT NULL DEFAULT '0',
    "target_position_y" REAL NOT NULL DEFAULT '0',
    "target_position_z" REAL NOT NULL DEFAULT '0',
    "target_orientation" REAL NOT NULL DEFAULT '0',
    PRIMARY KEY ("id", "patch")
);
ALTER TABLE IF EXISTS "areatrigger_teleport" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "areatrigger_teleport" ADD COLUMN IF NOT EXISTS "patch" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch" >= 0);
ALTER TABLE IF EXISTS "areatrigger_teleport" ADD COLUMN IF NOT EXISTS "name" VARCHAR(64) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "areatrigger_teleport" ADD COLUMN IF NOT EXISTS "message" VARCHAR(128) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "areatrigger_teleport" ADD COLUMN IF NOT EXISTS "required_level" SMALLINT NOT NULL DEFAULT '0' CHECK ("required_level" >= 0);
ALTER TABLE IF EXISTS "areatrigger_teleport" ADD COLUMN IF NOT EXISTS "required_condition" BIGINT NOT NULL DEFAULT '0' CHECK ("required_condition" >= 0);
ALTER TABLE IF EXISTS "areatrigger_teleport" ADD COLUMN IF NOT EXISTS "target_map" INTEGER NOT NULL DEFAULT '0' CHECK ("target_map" >= 0);
ALTER TABLE IF EXISTS "areatrigger_teleport" ADD COLUMN IF NOT EXISTS "target_position_x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "areatrigger_teleport" ADD COLUMN IF NOT EXISTS "target_position_y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "areatrigger_teleport" ADD COLUMN IF NOT EXISTS "target_position_z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "areatrigger_teleport" ADD COLUMN IF NOT EXISTS "target_orientation" REAL NOT NULL DEFAULT '0';

CREATE TABLE IF NOT EXISTS "areatrigger_template" (
    "id" INTEGER NOT NULL CHECK ("id" >= 0),
    "build" INTEGER NOT NULL CHECK ("build" >= 0),
    "name" VARCHAR(128) DEFAULT '',
    "map_id" INTEGER NOT NULL DEFAULT '0' CHECK ("map_id" >= 0),
    "x" REAL NOT NULL DEFAULT '0',
    "y" REAL NOT NULL DEFAULT '0',
    "z" REAL NOT NULL DEFAULT '0',
    "radius" REAL NOT NULL DEFAULT '0',
    "box_x" REAL NOT NULL DEFAULT '0',
    "box_y" REAL NOT NULL DEFAULT '0',
    "box_z" REAL NOT NULL DEFAULT '0',
    "box_orientation" REAL NOT NULL DEFAULT '0',
    "cooldown" BIGINT NOT NULL DEFAULT '0' CHECK ("cooldown" >= 0),
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    "script_id" BIGINT NOT NULL DEFAULT '0' CHECK ("script_id" >= 0),
    "script_name" VARCHAR(64) NOT NULL DEFAULT '',
    PRIMARY KEY ("id", "build")
);
ALTER TABLE IF EXISTS "areatrigger_template" ADD COLUMN IF NOT EXISTS "id" INTEGER NOT NULL CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "areatrigger_template" ADD COLUMN IF NOT EXISTS "build" INTEGER NOT NULL CHECK ("build" >= 0);
ALTER TABLE IF EXISTS "areatrigger_template" ADD COLUMN IF NOT EXISTS "name" VARCHAR(128) DEFAULT '';
ALTER TABLE IF EXISTS "areatrigger_template" ADD COLUMN IF NOT EXISTS "map_id" INTEGER NOT NULL DEFAULT '0' CHECK ("map_id" >= 0);
ALTER TABLE IF EXISTS "areatrigger_template" ADD COLUMN IF NOT EXISTS "x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "areatrigger_template" ADD COLUMN IF NOT EXISTS "y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "areatrigger_template" ADD COLUMN IF NOT EXISTS "z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "areatrigger_template" ADD COLUMN IF NOT EXISTS "radius" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "areatrigger_template" ADD COLUMN IF NOT EXISTS "box_x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "areatrigger_template" ADD COLUMN IF NOT EXISTS "box_y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "areatrigger_template" ADD COLUMN IF NOT EXISTS "box_z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "areatrigger_template" ADD COLUMN IF NOT EXISTS "box_orientation" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "areatrigger_template" ADD COLUMN IF NOT EXISTS "cooldown" BIGINT NOT NULL DEFAULT '0' CHECK ("cooldown" >= 0);
ALTER TABLE IF EXISTS "areatrigger_template" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);
ALTER TABLE IF EXISTS "areatrigger_template" ADD COLUMN IF NOT EXISTS "script_id" BIGINT NOT NULL DEFAULT '0' CHECK ("script_id" >= 0);
ALTER TABLE IF EXISTS "areatrigger_template" ADD COLUMN IF NOT EXISTS "script_name" VARCHAR(64) NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS "auctionhousebot" (
    "item" BIGINT NOT NULL CHECK ("item" >= 0),
    "stack" SMALLINT NOT NULL DEFAULT '1' CHECK ("stack" >= 0),
    "bid" BIGINT NOT NULL DEFAULT '1' CHECK ("bid" >= 0),
    "buyout" BIGINT NOT NULL DEFAULT '1' CHECK ("buyout" >= 0)
);
ALTER TABLE IF EXISTS "auctionhousebot" ADD COLUMN IF NOT EXISTS "item" BIGINT NOT NULL CHECK ("item" >= 0);
ALTER TABLE IF EXISTS "auctionhousebot" ADD COLUMN IF NOT EXISTS "stack" SMALLINT NOT NULL DEFAULT '1' CHECK ("stack" >= 0);
ALTER TABLE IF EXISTS "auctionhousebot" ADD COLUMN IF NOT EXISTS "bid" BIGINT NOT NULL DEFAULT '1' CHECK ("bid" >= 0);
ALTER TABLE IF EXISTS "auctionhousebot" ADD COLUMN IF NOT EXISTS "buyout" BIGINT NOT NULL DEFAULT '1' CHECK ("buyout" >= 0);

CREATE TABLE IF NOT EXISTS "autobroadcast" (
    "id" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("id" >= 0),
    "string_id" INTEGER DEFAULT NULL,
    "schedule" VARCHAR(255) NOT NULL,
    "enabled" SMALLINT NOT NULL DEFAULT '1' CHECK ("enabled" >= 0),
    PRIMARY KEY ("id")
);
ALTER TABLE IF EXISTS "autobroadcast" ADD COLUMN IF NOT EXISTS "id" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "autobroadcast" ADD COLUMN IF NOT EXISTS "string_id" INTEGER DEFAULT NULL;
ALTER TABLE IF EXISTS "autobroadcast" ADD COLUMN IF NOT EXISTS "schedule" VARCHAR(255) NOT NULL;
ALTER TABLE IF EXISTS "autobroadcast" ADD COLUMN IF NOT EXISTS "enabled" SMALLINT NOT NULL DEFAULT '1' CHECK ("enabled" >= 0);

CREATE TABLE IF NOT EXISTS "battleground_events" (
    "map" INTEGER NOT NULL CHECK ("map" >= 0),
    "event1" SMALLINT NOT NULL CHECK ("event1" >= 0),
    "event2" SMALLINT NOT NULL CHECK ("event2" >= 0),
    "description" VARCHAR(255) NOT NULL,
    PRIMARY KEY ("map", "event1", "event2")
);
ALTER TABLE IF EXISTS "battleground_events" ADD COLUMN IF NOT EXISTS "map" INTEGER NOT NULL CHECK ("map" >= 0);
ALTER TABLE IF EXISTS "battleground_events" ADD COLUMN IF NOT EXISTS "event1" SMALLINT NOT NULL CHECK ("event1" >= 0);
ALTER TABLE IF EXISTS "battleground_events" ADD COLUMN IF NOT EXISTS "event2" SMALLINT NOT NULL CHECK ("event2" >= 0);
ALTER TABLE IF EXISTS "battleground_events" ADD COLUMN IF NOT EXISTS "description" VARCHAR(255) NOT NULL;

CREATE TABLE IF NOT EXISTS "battleground_template" (
    "id" BIGINT NOT NULL CHECK ("id" >= 0),
    "patch" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch" >= 0),
    "min_players_per_team" INTEGER NOT NULL DEFAULT '0' CHECK ("min_players_per_team" >= 0),
    "max_players_per_team" INTEGER NOT NULL DEFAULT '0' CHECK ("max_players_per_team" >= 0),
    "min_level" SMALLINT NOT NULL DEFAULT '0' CHECK ("min_level" >= 0),
    "max_level" SMALLINT NOT NULL DEFAULT '0' CHECK ("max_level" >= 0),
    "alliance_win_spell" INTEGER NOT NULL DEFAULT '0' CHECK ("alliance_win_spell" >= 0),
    "alliance_lose_spell" INTEGER NOT NULL DEFAULT '0' CHECK ("alliance_lose_spell" >= 0),
    "horde_win_spell" INTEGER NOT NULL DEFAULT '0' CHECK ("horde_win_spell" >= 0),
    "horde_lose_spell" INTEGER NOT NULL DEFAULT '0' CHECK ("horde_lose_spell" >= 0),
    "alliance_start_location" BIGINT NOT NULL DEFAULT '0' CHECK ("alliance_start_location" >= 0),
    "horde_start_location" BIGINT NOT NULL DEFAULT '0' CHECK ("horde_start_location" >= 0),
    "player_loot_id" BIGINT NOT NULL DEFAULT '0' CHECK ("player_loot_id" >= 0),
    PRIMARY KEY ("id", "patch")
);
ALTER TABLE IF EXISTS "battleground_template" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "battleground_template" ADD COLUMN IF NOT EXISTS "patch" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch" >= 0);
ALTER TABLE IF EXISTS "battleground_template" ADD COLUMN IF NOT EXISTS "min_players_per_team" INTEGER NOT NULL DEFAULT '0' CHECK ("min_players_per_team" >= 0);
ALTER TABLE IF EXISTS "battleground_template" ADD COLUMN IF NOT EXISTS "max_players_per_team" INTEGER NOT NULL DEFAULT '0' CHECK ("max_players_per_team" >= 0);
ALTER TABLE IF EXISTS "battleground_template" ADD COLUMN IF NOT EXISTS "min_level" SMALLINT NOT NULL DEFAULT '0' CHECK ("min_level" >= 0);
ALTER TABLE IF EXISTS "battleground_template" ADD COLUMN IF NOT EXISTS "max_level" SMALLINT NOT NULL DEFAULT '0' CHECK ("max_level" >= 0);
ALTER TABLE IF EXISTS "battleground_template" ADD COLUMN IF NOT EXISTS "alliance_win_spell" INTEGER NOT NULL DEFAULT '0' CHECK ("alliance_win_spell" >= 0);
ALTER TABLE IF EXISTS "battleground_template" ADD COLUMN IF NOT EXISTS "alliance_lose_spell" INTEGER NOT NULL DEFAULT '0' CHECK ("alliance_lose_spell" >= 0);
ALTER TABLE IF EXISTS "battleground_template" ADD COLUMN IF NOT EXISTS "horde_win_spell" INTEGER NOT NULL DEFAULT '0' CHECK ("horde_win_spell" >= 0);
ALTER TABLE IF EXISTS "battleground_template" ADD COLUMN IF NOT EXISTS "horde_lose_spell" INTEGER NOT NULL DEFAULT '0' CHECK ("horde_lose_spell" >= 0);
ALTER TABLE IF EXISTS "battleground_template" ADD COLUMN IF NOT EXISTS "alliance_start_location" BIGINT NOT NULL DEFAULT '0' CHECK ("alliance_start_location" >= 0);
ALTER TABLE IF EXISTS "battleground_template" ADD COLUMN IF NOT EXISTS "horde_start_location" BIGINT NOT NULL DEFAULT '0' CHECK ("horde_start_location" >= 0);
ALTER TABLE IF EXISTS "battleground_template" ADD COLUMN IF NOT EXISTS "player_loot_id" BIGINT NOT NULL DEFAULT '0' CHECK ("player_loot_id" >= 0);

CREATE TABLE IF NOT EXISTS "battlemaster_entry" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "bg_template" BIGINT NOT NULL DEFAULT '0' CHECK ("bg_template" >= 0),
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "battlemaster_entry" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "battlemaster_entry" ADD COLUMN IF NOT EXISTS "bg_template" BIGINT NOT NULL DEFAULT '0' CHECK ("bg_template" >= 0);

CREATE TABLE IF NOT EXISTS "broadcast_text" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "male_text" TEXT,
    "female_text" TEXT,
    "chat_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("chat_type" >= 0),
    "sound_id" INTEGER NOT NULL DEFAULT '0' CHECK ("sound_id" >= 0),
    "language_id" SMALLINT NOT NULL DEFAULT '0' CHECK ("language_id" >= 0),
    "emote_id1" INTEGER NOT NULL DEFAULT '0' CHECK ("emote_id1" >= 0),
    "emote_id2" INTEGER NOT NULL DEFAULT '0' CHECK ("emote_id2" >= 0),
    "emote_id3" INTEGER NOT NULL DEFAULT '0' CHECK ("emote_id3" >= 0),
    "emote_delay1" BIGINT NOT NULL DEFAULT '0' CHECK ("emote_delay1" >= 0),
    "emote_delay2" BIGINT NOT NULL DEFAULT '0' CHECK ("emote_delay2" >= 0),
    "emote_delay3" BIGINT NOT NULL DEFAULT '0' CHECK ("emote_delay3" >= 0),
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "broadcast_text" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "broadcast_text" ADD COLUMN IF NOT EXISTS "male_text" TEXT;
ALTER TABLE IF EXISTS "broadcast_text" ADD COLUMN IF NOT EXISTS "female_text" TEXT;
ALTER TABLE IF EXISTS "broadcast_text" ADD COLUMN IF NOT EXISTS "chat_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("chat_type" >= 0);
ALTER TABLE IF EXISTS "broadcast_text" ADD COLUMN IF NOT EXISTS "sound_id" INTEGER NOT NULL DEFAULT '0' CHECK ("sound_id" >= 0);
ALTER TABLE IF EXISTS "broadcast_text" ADD COLUMN IF NOT EXISTS "language_id" SMALLINT NOT NULL DEFAULT '0' CHECK ("language_id" >= 0);
ALTER TABLE IF EXISTS "broadcast_text" ADD COLUMN IF NOT EXISTS "emote_id1" INTEGER NOT NULL DEFAULT '0' CHECK ("emote_id1" >= 0);
ALTER TABLE IF EXISTS "broadcast_text" ADD COLUMN IF NOT EXISTS "emote_id2" INTEGER NOT NULL DEFAULT '0' CHECK ("emote_id2" >= 0);
ALTER TABLE IF EXISTS "broadcast_text" ADD COLUMN IF NOT EXISTS "emote_id3" INTEGER NOT NULL DEFAULT '0' CHECK ("emote_id3" >= 0);
ALTER TABLE IF EXISTS "broadcast_text" ADD COLUMN IF NOT EXISTS "emote_delay1" BIGINT NOT NULL DEFAULT '0' CHECK ("emote_delay1" >= 0);
ALTER TABLE IF EXISTS "broadcast_text" ADD COLUMN IF NOT EXISTS "emote_delay2" BIGINT NOT NULL DEFAULT '0' CHECK ("emote_delay2" >= 0);
ALTER TABLE IF EXISTS "broadcast_text" ADD COLUMN IF NOT EXISTS "emote_delay3" BIGINT NOT NULL DEFAULT '0' CHECK ("emote_delay3" >= 0);

CREATE TABLE IF NOT EXISTS "cinematic_waypoints" (
    "cinematic" BIGINT DEFAULT '0' CHECK ("cinematic" >= 0),
    "timer" BIGINT DEFAULT '0' CHECK ("timer" >= 0),
    "position_x" REAL DEFAULT NULL,
    "position_y" REAL DEFAULT NULL,
    "position_z" REAL DEFAULT NULL,
    "comment" VARCHAR(255) DEFAULT NULL
);
ALTER TABLE IF EXISTS "cinematic_waypoints" ADD COLUMN IF NOT EXISTS "cinematic" BIGINT DEFAULT '0' CHECK ("cinematic" >= 0);
ALTER TABLE IF EXISTS "cinematic_waypoints" ADD COLUMN IF NOT EXISTS "timer" BIGINT DEFAULT '0' CHECK ("timer" >= 0);
ALTER TABLE IF EXISTS "cinematic_waypoints" ADD COLUMN IF NOT EXISTS "position_x" REAL DEFAULT NULL;
ALTER TABLE IF EXISTS "cinematic_waypoints" ADD COLUMN IF NOT EXISTS "position_y" REAL DEFAULT NULL;
ALTER TABLE IF EXISTS "cinematic_waypoints" ADD COLUMN IF NOT EXISTS "position_z" REAL DEFAULT NULL;
ALTER TABLE IF EXISTS "cinematic_waypoints" ADD COLUMN IF NOT EXISTS "comment" VARCHAR(255) DEFAULT NULL;

CREATE TABLE IF NOT EXISTS "command" (
    "name" VARCHAR(50) NOT NULL DEFAULT '',
    "security" SMALLINT NOT NULL DEFAULT '0' CHECK ("security" >= 0),
    "help" TEXT,
    "flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0),
    PRIMARY KEY ("name")
);
ALTER TABLE IF EXISTS "command" ADD COLUMN IF NOT EXISTS "name" VARCHAR(50) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "command" ADD COLUMN IF NOT EXISTS "security" SMALLINT NOT NULL DEFAULT '0' CHECK ("security" >= 0);
ALTER TABLE IF EXISTS "command" ADD COLUMN IF NOT EXISTS "help" TEXT;
ALTER TABLE IF EXISTS "command" ADD COLUMN IF NOT EXISTS "flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0);

CREATE TABLE IF NOT EXISTS "conditions" (
    "condition_entry" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("condition_entry" >= 0),
    "type" SMALLINT NOT NULL DEFAULT '0',
    "value1" INTEGER NOT NULL DEFAULT '0',
    "value2" INTEGER NOT NULL DEFAULT '0',
    "value3" INTEGER NOT NULL DEFAULT '0',
    "value4" INTEGER NOT NULL DEFAULT '0',
    "flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0),
    PRIMARY KEY ("condition_entry"),
    UNIQUE ("type", "value1", "value2", "flags", "value3", "value4")
);
ALTER TABLE IF EXISTS "conditions" ADD COLUMN IF NOT EXISTS "condition_entry" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("condition_entry" >= 0);
ALTER TABLE IF EXISTS "conditions" ADD COLUMN IF NOT EXISTS "type" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "conditions" ADD COLUMN IF NOT EXISTS "value1" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "conditions" ADD COLUMN IF NOT EXISTS "value2" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "conditions" ADD COLUMN IF NOT EXISTS "value3" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "conditions" ADD COLUMN IF NOT EXISTS "value4" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "conditions" ADD COLUMN IF NOT EXISTS "flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0);
CREATE UNIQUE INDEX IF NOT EXISTS idx_conditions_unique_conditions ON "conditions" ("type", "value1", "value2", "flags", "value3", "value4");

CREATE TABLE IF NOT EXISTS "creature" (
    "guid" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("guid" >= 0),
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "id2" BIGINT NOT NULL DEFAULT '0' CHECK ("id2" >= 0),
    "id3" BIGINT NOT NULL DEFAULT '0' CHECK ("id3" >= 0),
    "id4" BIGINT NOT NULL DEFAULT '0' CHECK ("id4" >= 0),
    "id5" BIGINT NOT NULL DEFAULT '0' CHECK ("id5" >= 0),
    "map" INTEGER NOT NULL DEFAULT '0' CHECK ("map" >= 0),
    "position_x" REAL NOT NULL DEFAULT '0',
    "position_y" REAL NOT NULL DEFAULT '0',
    "position_z" REAL NOT NULL DEFAULT '0',
    "orientation" REAL NOT NULL DEFAULT '0',
    "spawntimesecsmin" BIGINT NOT NULL DEFAULT '120' CHECK ("spawntimesecsmin" >= 0),
    "spawntimesecsmax" BIGINT NOT NULL DEFAULT '120' CHECK ("spawntimesecsmax" >= 0),
    "wander_distance" REAL NOT NULL DEFAULT '5',
    "health_percent" REAL NOT NULL DEFAULT '100',
    "mana_percent" REAL NOT NULL DEFAULT '100' CHECK ("mana_percent" >= 0),
    "movement_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("movement_type" >= 0),
    "spawn_flags" BIGINT NOT NULL DEFAULT '0' CHECK ("spawn_flags" >= 0),
    "visibility_mod" REAL DEFAULT '0',
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0),
    PRIMARY KEY ("guid")
);
ALTER TABLE IF EXISTS "creature" ADD COLUMN IF NOT EXISTS "guid" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "creature" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "creature" ADD COLUMN IF NOT EXISTS "id2" BIGINT NOT NULL DEFAULT '0' CHECK ("id2" >= 0);
ALTER TABLE IF EXISTS "creature" ADD COLUMN IF NOT EXISTS "id3" BIGINT NOT NULL DEFAULT '0' CHECK ("id3" >= 0);
ALTER TABLE IF EXISTS "creature" ADD COLUMN IF NOT EXISTS "id4" BIGINT NOT NULL DEFAULT '0' CHECK ("id4" >= 0);
ALTER TABLE IF EXISTS "creature" ADD COLUMN IF NOT EXISTS "id5" BIGINT NOT NULL DEFAULT '0' CHECK ("id5" >= 0);
ALTER TABLE IF EXISTS "creature" ADD COLUMN IF NOT EXISTS "map" INTEGER NOT NULL DEFAULT '0' CHECK ("map" >= 0);
ALTER TABLE IF EXISTS "creature" ADD COLUMN IF NOT EXISTS "position_x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature" ADD COLUMN IF NOT EXISTS "position_y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature" ADD COLUMN IF NOT EXISTS "position_z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature" ADD COLUMN IF NOT EXISTS "orientation" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature" ADD COLUMN IF NOT EXISTS "spawntimesecsmin" BIGINT NOT NULL DEFAULT '120' CHECK ("spawntimesecsmin" >= 0);
ALTER TABLE IF EXISTS "creature" ADD COLUMN IF NOT EXISTS "spawntimesecsmax" BIGINT NOT NULL DEFAULT '120' CHECK ("spawntimesecsmax" >= 0);
ALTER TABLE IF EXISTS "creature" ADD COLUMN IF NOT EXISTS "wander_distance" REAL NOT NULL DEFAULT '5';
ALTER TABLE IF EXISTS "creature" ADD COLUMN IF NOT EXISTS "health_percent" REAL NOT NULL DEFAULT '100';
ALTER TABLE IF EXISTS "creature" ADD COLUMN IF NOT EXISTS "mana_percent" REAL NOT NULL DEFAULT '100' CHECK ("mana_percent" >= 0);
ALTER TABLE IF EXISTS "creature" ADD COLUMN IF NOT EXISTS "movement_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("movement_type" >= 0);
ALTER TABLE IF EXISTS "creature" ADD COLUMN IF NOT EXISTS "spawn_flags" BIGINT NOT NULL DEFAULT '0' CHECK ("spawn_flags" >= 0);
ALTER TABLE IF EXISTS "creature" ADD COLUMN IF NOT EXISTS "visibility_mod" REAL DEFAULT '0';
ALTER TABLE IF EXISTS "creature" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);
ALTER TABLE IF EXISTS "creature" ADD COLUMN IF NOT EXISTS "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0);
CREATE INDEX IF NOT EXISTS idx_creature_idx_map ON "creature" ("map");
CREATE INDEX IF NOT EXISTS idx_creature_idx_id ON "creature" ("id");

CREATE TABLE IF NOT EXISTS "creature_addon" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "patch" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch" >= 0),
    "display_id" INTEGER NOT NULL DEFAULT '0' CHECK ("display_id" >= 0),
    "mount_display_id" SMALLINT NOT NULL DEFAULT '-1',
    "equipment_id" INTEGER NOT NULL DEFAULT '-1',
    "stand_state" SMALLINT NOT NULL DEFAULT '0' CHECK ("stand_state" >= 0),
    "sheath_state" SMALLINT NOT NULL DEFAULT '1' CHECK ("sheath_state" >= 0),
    "emote_state" INTEGER NOT NULL DEFAULT '0' CHECK ("emote_state" >= 0),
    "auras" TEXT,
    PRIMARY KEY ("guid", "patch")
);
ALTER TABLE IF EXISTS "creature_addon" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "creature_addon" ADD COLUMN IF NOT EXISTS "patch" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch" >= 0);
ALTER TABLE IF EXISTS "creature_addon" ADD COLUMN IF NOT EXISTS "display_id" INTEGER NOT NULL DEFAULT '0' CHECK ("display_id" >= 0);
ALTER TABLE IF EXISTS "creature_addon" ADD COLUMN IF NOT EXISTS "mount_display_id" SMALLINT NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "creature_addon" ADD COLUMN IF NOT EXISTS "equipment_id" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "creature_addon" ADD COLUMN IF NOT EXISTS "stand_state" SMALLINT NOT NULL DEFAULT '0' CHECK ("stand_state" >= 0);
ALTER TABLE IF EXISTS "creature_addon" ADD COLUMN IF NOT EXISTS "sheath_state" SMALLINT NOT NULL DEFAULT '1' CHECK ("sheath_state" >= 0);
ALTER TABLE IF EXISTS "creature_addon" ADD COLUMN IF NOT EXISTS "emote_state" INTEGER NOT NULL DEFAULT '0' CHECK ("emote_state" >= 0);
ALTER TABLE IF EXISTS "creature_addon" ADD COLUMN IF NOT EXISTS "auras" TEXT;

CREATE TABLE IF NOT EXISTS "creature_ai_events" (
    "id" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("id" >= 0),
    "creature_id" BIGINT NOT NULL DEFAULT '0' CHECK ("creature_id" >= 0),
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    "event_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("event_type" >= 0),
    "event_inverse_phase_mask" INTEGER NOT NULL DEFAULT '0',
    "event_chance" SMALLINT NOT NULL DEFAULT '100' CHECK ("event_chance" >= 0),
    "event_flags" BIGINT NOT NULL DEFAULT '0' CHECK ("event_flags" >= 0),
    "event_param1" INTEGER NOT NULL DEFAULT '0',
    "event_param2" INTEGER NOT NULL DEFAULT '0',
    "event_param3" INTEGER NOT NULL DEFAULT '0',
    "event_param4" INTEGER NOT NULL DEFAULT '0',
    "action1_script" BIGINT NOT NULL DEFAULT '0' CHECK ("action1_script" >= 0),
    "action2_script" BIGINT NOT NULL DEFAULT '0' CHECK ("action2_script" >= 0),
    "action3_script" BIGINT NOT NULL DEFAULT '0' CHECK ("action3_script" >= 0),
    "comment" VARCHAR(255) NOT NULL DEFAULT '',
    PRIMARY KEY ("id")
);
ALTER TABLE IF EXISTS "creature_ai_events" ADD COLUMN IF NOT EXISTS "id" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "creature_ai_events" ADD COLUMN IF NOT EXISTS "creature_id" BIGINT NOT NULL DEFAULT '0' CHECK ("creature_id" >= 0);
ALTER TABLE IF EXISTS "creature_ai_events" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);
ALTER TABLE IF EXISTS "creature_ai_events" ADD COLUMN IF NOT EXISTS "event_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("event_type" >= 0);
ALTER TABLE IF EXISTS "creature_ai_events" ADD COLUMN IF NOT EXISTS "event_inverse_phase_mask" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_ai_events" ADD COLUMN IF NOT EXISTS "event_chance" SMALLINT NOT NULL DEFAULT '100' CHECK ("event_chance" >= 0);
ALTER TABLE IF EXISTS "creature_ai_events" ADD COLUMN IF NOT EXISTS "event_flags" BIGINT NOT NULL DEFAULT '0' CHECK ("event_flags" >= 0);
ALTER TABLE IF EXISTS "creature_ai_events" ADD COLUMN IF NOT EXISTS "event_param1" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_ai_events" ADD COLUMN IF NOT EXISTS "event_param2" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_ai_events" ADD COLUMN IF NOT EXISTS "event_param3" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_ai_events" ADD COLUMN IF NOT EXISTS "event_param4" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_ai_events" ADD COLUMN IF NOT EXISTS "action1_script" BIGINT NOT NULL DEFAULT '0' CHECK ("action1_script" >= 0);
ALTER TABLE IF EXISTS "creature_ai_events" ADD COLUMN IF NOT EXISTS "action2_script" BIGINT NOT NULL DEFAULT '0' CHECK ("action2_script" >= 0);
ALTER TABLE IF EXISTS "creature_ai_events" ADD COLUMN IF NOT EXISTS "action3_script" BIGINT NOT NULL DEFAULT '0' CHECK ("action3_script" >= 0);
ALTER TABLE IF EXISTS "creature_ai_events" ADD COLUMN IF NOT EXISTS "comment" VARCHAR(255) NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS "creature_ai_scripts" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "delay" BIGINT NOT NULL DEFAULT '0' CHECK ("delay" >= 0),
    "priority" SMALLINT NOT NULL DEFAULT '0' CHECK ("priority" >= 0),
    "command" SMALLINT NOT NULL DEFAULT '0' CHECK ("command" >= 0),
    "datalong" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong" >= 0),
    "datalong2" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong2" >= 0),
    "datalong3" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong3" >= 0),
    "datalong4" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong4" >= 0),
    "target_param1" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param1" >= 0),
    "target_param2" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param2" >= 0),
    "target_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("target_type" >= 0),
    "data_flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("data_flags" >= 0),
    "dataint" INTEGER NOT NULL DEFAULT '0',
    "dataint2" INTEGER NOT NULL DEFAULT '0',
    "dataint3" INTEGER NOT NULL DEFAULT '0',
    "dataint4" INTEGER NOT NULL DEFAULT '0',
    "x" REAL NOT NULL DEFAULT '0',
    "y" REAL NOT NULL DEFAULT '0',
    "z" REAL NOT NULL DEFAULT '0',
    "o" REAL NOT NULL DEFAULT '0',
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    "comments" VARCHAR(255) NOT NULL
);
ALTER TABLE IF EXISTS "creature_ai_scripts" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "creature_ai_scripts" ADD COLUMN IF NOT EXISTS "delay" BIGINT NOT NULL DEFAULT '0' CHECK ("delay" >= 0);
ALTER TABLE IF EXISTS "creature_ai_scripts" ADD COLUMN IF NOT EXISTS "priority" SMALLINT NOT NULL DEFAULT '0' CHECK ("priority" >= 0);
ALTER TABLE IF EXISTS "creature_ai_scripts" ADD COLUMN IF NOT EXISTS "command" SMALLINT NOT NULL DEFAULT '0' CHECK ("command" >= 0);
ALTER TABLE IF EXISTS "creature_ai_scripts" ADD COLUMN IF NOT EXISTS "datalong" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong" >= 0);
ALTER TABLE IF EXISTS "creature_ai_scripts" ADD COLUMN IF NOT EXISTS "datalong2" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong2" >= 0);
ALTER TABLE IF EXISTS "creature_ai_scripts" ADD COLUMN IF NOT EXISTS "datalong3" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong3" >= 0);
ALTER TABLE IF EXISTS "creature_ai_scripts" ADD COLUMN IF NOT EXISTS "datalong4" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong4" >= 0);
ALTER TABLE IF EXISTS "creature_ai_scripts" ADD COLUMN IF NOT EXISTS "target_param1" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param1" >= 0);
ALTER TABLE IF EXISTS "creature_ai_scripts" ADD COLUMN IF NOT EXISTS "target_param2" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param2" >= 0);
ALTER TABLE IF EXISTS "creature_ai_scripts" ADD COLUMN IF NOT EXISTS "target_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("target_type" >= 0);
ALTER TABLE IF EXISTS "creature_ai_scripts" ADD COLUMN IF NOT EXISTS "data_flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("data_flags" >= 0);
ALTER TABLE IF EXISTS "creature_ai_scripts" ADD COLUMN IF NOT EXISTS "dataint" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_ai_scripts" ADD COLUMN IF NOT EXISTS "dataint2" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_ai_scripts" ADD COLUMN IF NOT EXISTS "dataint3" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_ai_scripts" ADD COLUMN IF NOT EXISTS "dataint4" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_ai_scripts" ADD COLUMN IF NOT EXISTS "x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_ai_scripts" ADD COLUMN IF NOT EXISTS "y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_ai_scripts" ADD COLUMN IF NOT EXISTS "z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_ai_scripts" ADD COLUMN IF NOT EXISTS "o" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_ai_scripts" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);
ALTER TABLE IF EXISTS "creature_ai_scripts" ADD COLUMN IF NOT EXISTS "comments" VARCHAR(255) NOT NULL;

CREATE TABLE IF NOT EXISTS "creature_battleground" (
    "guid" BIGINT NOT NULL CHECK ("guid" >= 0),
    "event1" SMALLINT NOT NULL CHECK ("event1" >= 0),
    "event2" SMALLINT NOT NULL CHECK ("event2" >= 0),
    PRIMARY KEY ("guid", "event1")
);
ALTER TABLE IF EXISTS "creature_battleground" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "creature_battleground" ADD COLUMN IF NOT EXISTS "event1" SMALLINT NOT NULL CHECK ("event1" >= 0);
ALTER TABLE IF EXISTS "creature_battleground" ADD COLUMN IF NOT EXISTS "event2" SMALLINT NOT NULL CHECK ("event2" >= 0);

CREATE TABLE IF NOT EXISTS "creature_classlevelstats" (
    "class" SMALLINT NOT NULL CHECK ("class" >= 0),
    "level" SMALLINT NOT NULL CHECK ("level" >= 0),
    "melee_damage" REAL NOT NULL DEFAULT '0',
    "ranged_damage" REAL NOT NULL DEFAULT '0',
    "attack_power" INTEGER NOT NULL DEFAULT '0',
    "ranged_attack_power" INTEGER NOT NULL DEFAULT '0',
    "health" INTEGER NOT NULL DEFAULT '0',
    "base_health" INTEGER NOT NULL DEFAULT '0',
    "mana" INTEGER NOT NULL DEFAULT '0',
    "base_mana" INTEGER NOT NULL DEFAULT '0',
    "strength" INTEGER NOT NULL DEFAULT '0',
    "agility" INTEGER NOT NULL DEFAULT '0',
    "stamina" INTEGER NOT NULL DEFAULT '0',
    "intellect" INTEGER NOT NULL DEFAULT '0',
    "spirit" INTEGER NOT NULL DEFAULT '0',
    "armor" INTEGER NOT NULL DEFAULT '0',
    PRIMARY KEY ("class", "level")
);
ALTER TABLE IF EXISTS "creature_classlevelstats" ADD COLUMN IF NOT EXISTS "class" SMALLINT NOT NULL CHECK ("class" >= 0);
ALTER TABLE IF EXISTS "creature_classlevelstats" ADD COLUMN IF NOT EXISTS "level" SMALLINT NOT NULL CHECK ("level" >= 0);
ALTER TABLE IF EXISTS "creature_classlevelstats" ADD COLUMN IF NOT EXISTS "melee_damage" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_classlevelstats" ADD COLUMN IF NOT EXISTS "ranged_damage" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_classlevelstats" ADD COLUMN IF NOT EXISTS "attack_power" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_classlevelstats" ADD COLUMN IF NOT EXISTS "ranged_attack_power" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_classlevelstats" ADD COLUMN IF NOT EXISTS "health" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_classlevelstats" ADD COLUMN IF NOT EXISTS "base_health" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_classlevelstats" ADD COLUMN IF NOT EXISTS "mana" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_classlevelstats" ADD COLUMN IF NOT EXISTS "base_mana" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_classlevelstats" ADD COLUMN IF NOT EXISTS "strength" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_classlevelstats" ADD COLUMN IF NOT EXISTS "agility" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_classlevelstats" ADD COLUMN IF NOT EXISTS "stamina" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_classlevelstats" ADD COLUMN IF NOT EXISTS "intellect" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_classlevelstats" ADD COLUMN IF NOT EXISTS "spirit" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_classlevelstats" ADD COLUMN IF NOT EXISTS "armor" INTEGER NOT NULL DEFAULT '0';

CREATE TABLE IF NOT EXISTS "creature_display_info_addon" (
    "display_id" BIGINT NOT NULL DEFAULT '0' CHECK ("display_id" >= 0),
    "build" INTEGER NOT NULL DEFAULT '0' CHECK ("build" >= 0),
    "bounding_radius" REAL NOT NULL DEFAULT '0',
    "combat_reach" REAL NOT NULL DEFAULT '0',
    "speed_walk" REAL NOT NULL DEFAULT '1',
    "speed_run" REAL NOT NULL DEFAULT '1.14286',
    "gender" SMALLINT NOT NULL DEFAULT '2' CHECK ("gender" >= 0),
    "display_id_other_gender" BIGINT NOT NULL DEFAULT '0' CHECK ("display_id_other_gender" >= 0),
    PRIMARY KEY ("display_id", "build")
);
ALTER TABLE IF EXISTS "creature_display_info_addon" ADD COLUMN IF NOT EXISTS "display_id" BIGINT NOT NULL DEFAULT '0' CHECK ("display_id" >= 0);
ALTER TABLE IF EXISTS "creature_display_info_addon" ADD COLUMN IF NOT EXISTS "build" INTEGER NOT NULL DEFAULT '0' CHECK ("build" >= 0);
ALTER TABLE IF EXISTS "creature_display_info_addon" ADD COLUMN IF NOT EXISTS "bounding_radius" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_display_info_addon" ADD COLUMN IF NOT EXISTS "combat_reach" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_display_info_addon" ADD COLUMN IF NOT EXISTS "speed_walk" REAL NOT NULL DEFAULT '1';
ALTER TABLE IF EXISTS "creature_display_info_addon" ADD COLUMN IF NOT EXISTS "speed_run" REAL NOT NULL DEFAULT '1.14286';
ALTER TABLE IF EXISTS "creature_display_info_addon" ADD COLUMN IF NOT EXISTS "gender" SMALLINT NOT NULL DEFAULT '2' CHECK ("gender" >= 0);
ALTER TABLE IF EXISTS "creature_display_info_addon" ADD COLUMN IF NOT EXISTS "display_id_other_gender" BIGINT NOT NULL DEFAULT '0' CHECK ("display_id_other_gender" >= 0);

CREATE TABLE IF NOT EXISTS "creature_equip_template" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "probability" BIGINT NOT NULL DEFAULT '1' CHECK ("probability" >= 0),
    "item1" BIGINT NOT NULL DEFAULT '0' CHECK ("item1" >= 0),
    "item2" BIGINT NOT NULL DEFAULT '0' CHECK ("item2" >= 0),
    "item3" BIGINT NOT NULL DEFAULT '0' CHECK ("item3" >= 0),
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0),
    PRIMARY KEY ("entry", "item1", "item2", "item3")
);
ALTER TABLE IF EXISTS "creature_equip_template" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "creature_equip_template" ADD COLUMN IF NOT EXISTS "probability" BIGINT NOT NULL DEFAULT '1' CHECK ("probability" >= 0);
ALTER TABLE IF EXISTS "creature_equip_template" ADD COLUMN IF NOT EXISTS "item1" BIGINT NOT NULL DEFAULT '0' CHECK ("item1" >= 0);
ALTER TABLE IF EXISTS "creature_equip_template" ADD COLUMN IF NOT EXISTS "item2" BIGINT NOT NULL DEFAULT '0' CHECK ("item2" >= 0);
ALTER TABLE IF EXISTS "creature_equip_template" ADD COLUMN IF NOT EXISTS "item3" BIGINT NOT NULL DEFAULT '0' CHECK ("item3" >= 0);
ALTER TABLE IF EXISTS "creature_equip_template" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);
ALTER TABLE IF EXISTS "creature_equip_template" ADD COLUMN IF NOT EXISTS "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0);

CREATE TABLE IF NOT EXISTS "creature_groups" (
    "leader_guid" BIGINT NOT NULL CHECK ("leader_guid" >= 0),
    "member_guid" BIGINT NOT NULL CHECK ("member_guid" >= 0),
    "dist" REAL NOT NULL CHECK ("dist" >= 0),
    "angle" REAL NOT NULL CHECK ("angle" >= 0),
    "flags" BIGINT NOT NULL CHECK ("flags" >= 0),
    PRIMARY KEY ("member_guid")
);
ALTER TABLE IF EXISTS "creature_groups" ADD COLUMN IF NOT EXISTS "leader_guid" BIGINT NOT NULL CHECK ("leader_guid" >= 0);
ALTER TABLE IF EXISTS "creature_groups" ADD COLUMN IF NOT EXISTS "member_guid" BIGINT NOT NULL CHECK ("member_guid" >= 0);
ALTER TABLE IF EXISTS "creature_groups" ADD COLUMN IF NOT EXISTS "dist" REAL NOT NULL CHECK ("dist" >= 0);
ALTER TABLE IF EXISTS "creature_groups" ADD COLUMN IF NOT EXISTS "angle" REAL NOT NULL CHECK ("angle" >= 0);
ALTER TABLE IF EXISTS "creature_groups" ADD COLUMN IF NOT EXISTS "flags" BIGINT NOT NULL CHECK ("flags" >= 0);

CREATE TABLE IF NOT EXISTS "creature_groups_entry_limit" (
    "leader_guid" BIGINT NOT NULL CHECK ("leader_guid" >= 0),
    "creature_id" BIGINT NOT NULL CHECK ("creature_id" >= 0),
    "min_count" BIGINT NOT NULL DEFAULT '0' CHECK ("min_count" >= 0),
    "max_count" BIGINT NOT NULL DEFAULT '1' CHECK ("max_count" >= 0),
    PRIMARY KEY ("leader_guid", "creature_id")
);
ALTER TABLE IF EXISTS "creature_groups_entry_limit" ADD COLUMN IF NOT EXISTS "leader_guid" BIGINT NOT NULL CHECK ("leader_guid" >= 0);
ALTER TABLE IF EXISTS "creature_groups_entry_limit" ADD COLUMN IF NOT EXISTS "creature_id" BIGINT NOT NULL CHECK ("creature_id" >= 0);
ALTER TABLE IF EXISTS "creature_groups_entry_limit" ADD COLUMN IF NOT EXISTS "min_count" BIGINT NOT NULL DEFAULT '0' CHECK ("min_count" >= 0);
ALTER TABLE IF EXISTS "creature_groups_entry_limit" ADD COLUMN IF NOT EXISTS "max_count" BIGINT NOT NULL DEFAULT '1' CHECK ("max_count" >= 0);

CREATE TABLE IF NOT EXISTS "creature_involvedrelation" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "quest" BIGINT NOT NULL DEFAULT '0' CHECK ("quest" >= 0),
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0),
    PRIMARY KEY ("id", "quest")
);
ALTER TABLE IF EXISTS "creature_involvedrelation" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "creature_involvedrelation" ADD COLUMN IF NOT EXISTS "quest" BIGINT NOT NULL DEFAULT '0' CHECK ("quest" >= 0);
ALTER TABLE IF EXISTS "creature_involvedrelation" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);
ALTER TABLE IF EXISTS "creature_involvedrelation" ADD COLUMN IF NOT EXISTS "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0);

CREATE TABLE IF NOT EXISTS "creature_linking" (
    "guid" BIGINT NOT NULL CHECK ("guid" >= 0),
    "master_guid" BIGINT NOT NULL CHECK ("master_guid" >= 0),
    "flag" BIGINT NOT NULL CHECK ("flag" >= 0),
    PRIMARY KEY ("guid")
);
ALTER TABLE IF EXISTS "creature_linking" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "creature_linking" ADD COLUMN IF NOT EXISTS "master_guid" BIGINT NOT NULL CHECK ("master_guid" >= 0);
ALTER TABLE IF EXISTS "creature_linking" ADD COLUMN IF NOT EXISTS "flag" BIGINT NOT NULL CHECK ("flag" >= 0);

CREATE TABLE IF NOT EXISTS "creature_linking_template" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "map" INTEGER NOT NULL DEFAULT '0' CHECK ("map" >= 0),
    "master_entry" BIGINT NOT NULL DEFAULT '0' CHECK ("master_entry" >= 0),
    "flag" BIGINT NOT NULL DEFAULT '0' CHECK ("flag" >= 0),
    "search_range" BIGINT NOT NULL DEFAULT '0' CHECK ("search_range" >= 0),
    PRIMARY KEY ("entry", "map")
);
ALTER TABLE IF EXISTS "creature_linking_template" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "creature_linking_template" ADD COLUMN IF NOT EXISTS "map" INTEGER NOT NULL DEFAULT '0' CHECK ("map" >= 0);
ALTER TABLE IF EXISTS "creature_linking_template" ADD COLUMN IF NOT EXISTS "master_entry" BIGINT NOT NULL DEFAULT '0' CHECK ("master_entry" >= 0);
ALTER TABLE IF EXISTS "creature_linking_template" ADD COLUMN IF NOT EXISTS "flag" BIGINT NOT NULL DEFAULT '0' CHECK ("flag" >= 0);
ALTER TABLE IF EXISTS "creature_linking_template" ADD COLUMN IF NOT EXISTS "search_range" BIGINT NOT NULL DEFAULT '0' CHECK ("search_range" >= 0);

CREATE TABLE IF NOT EXISTS "creature_loot_template" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "item" BIGINT NOT NULL DEFAULT '0' CHECK ("item" >= 0),
    "ChanceOrQuestChance" REAL NOT NULL DEFAULT '100',
    "groupid" SMALLINT NOT NULL DEFAULT '0' CHECK ("groupid" >= 0),
    "mincountOrRef" INTEGER NOT NULL DEFAULT '1',
    "maxcount" SMALLINT NOT NULL DEFAULT '1' CHECK ("maxcount" >= 0),
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0),
    PRIMARY KEY ("entry", "item", "groupid", "patch_min", "patch_max")
);
ALTER TABLE IF EXISTS "creature_loot_template" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "creature_loot_template" ADD COLUMN IF NOT EXISTS "item" BIGINT NOT NULL DEFAULT '0' CHECK ("item" >= 0);
ALTER TABLE IF EXISTS "creature_loot_template" ADD COLUMN IF NOT EXISTS "ChanceOrQuestChance" REAL NOT NULL DEFAULT '100';
ALTER TABLE IF EXISTS "creature_loot_template" ADD COLUMN IF NOT EXISTS "groupid" SMALLINT NOT NULL DEFAULT '0' CHECK ("groupid" >= 0);
ALTER TABLE IF EXISTS "creature_loot_template" ADD COLUMN IF NOT EXISTS "mincountOrRef" INTEGER NOT NULL DEFAULT '1';
ALTER TABLE IF EXISTS "creature_loot_template" ADD COLUMN IF NOT EXISTS "maxcount" SMALLINT NOT NULL DEFAULT '1' CHECK ("maxcount" >= 0);
ALTER TABLE IF EXISTS "creature_loot_template" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);
ALTER TABLE IF EXISTS "creature_loot_template" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);
ALTER TABLE IF EXISTS "creature_loot_template" ADD COLUMN IF NOT EXISTS "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0);

CREATE TABLE IF NOT EXISTS "creature_movement" (
    "id" BIGINT NOT NULL CHECK ("id" >= 0),
    "point" BIGINT NOT NULL DEFAULT '0' CHECK ("point" >= 0),
    "position_x" REAL NOT NULL DEFAULT '0',
    "position_y" REAL NOT NULL DEFAULT '0',
    "position_z" REAL NOT NULL DEFAULT '0',
    "orientation" REAL NOT NULL DEFAULT '0',
    "waittime" BIGINT NOT NULL DEFAULT '0' CHECK ("waittime" >= 0),
    "wander_distance" REAL NOT NULL DEFAULT '0' CHECK ("wander_distance" >= 0),
    "script_id" BIGINT NOT NULL DEFAULT '0' CHECK ("script_id" >= 0),
    "path_id" BIGINT NOT NULL DEFAULT '0' CHECK ("path_id" >= 0),
    PRIMARY KEY ("id", "point")
);
ALTER TABLE IF EXISTS "creature_movement" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "creature_movement" ADD COLUMN IF NOT EXISTS "point" BIGINT NOT NULL DEFAULT '0' CHECK ("point" >= 0);
ALTER TABLE IF EXISTS "creature_movement" ADD COLUMN IF NOT EXISTS "position_x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_movement" ADD COLUMN IF NOT EXISTS "position_y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_movement" ADD COLUMN IF NOT EXISTS "position_z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_movement" ADD COLUMN IF NOT EXISTS "orientation" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_movement" ADD COLUMN IF NOT EXISTS "waittime" BIGINT NOT NULL DEFAULT '0' CHECK ("waittime" >= 0);
ALTER TABLE IF EXISTS "creature_movement" ADD COLUMN IF NOT EXISTS "wander_distance" REAL NOT NULL DEFAULT '0' CHECK ("wander_distance" >= 0);
ALTER TABLE IF EXISTS "creature_movement" ADD COLUMN IF NOT EXISTS "script_id" BIGINT NOT NULL DEFAULT '0' CHECK ("script_id" >= 0);
ALTER TABLE IF EXISTS "creature_movement" ADD COLUMN IF NOT EXISTS "path_id" BIGINT NOT NULL DEFAULT '0' CHECK ("path_id" >= 0);

CREATE TABLE IF NOT EXISTS "creature_movement_scripts" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "delay" BIGINT NOT NULL DEFAULT '0' CHECK ("delay" >= 0),
    "priority" SMALLINT NOT NULL DEFAULT '0' CHECK ("priority" >= 0),
    "command" SMALLINT NOT NULL DEFAULT '0' CHECK ("command" >= 0),
    "datalong" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong" >= 0),
    "datalong2" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong2" >= 0),
    "datalong3" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong3" >= 0),
    "datalong4" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong4" >= 0),
    "target_param1" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param1" >= 0),
    "target_param2" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param2" >= 0),
    "target_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("target_type" >= 0),
    "data_flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("data_flags" >= 0),
    "dataint" INTEGER NOT NULL DEFAULT '0',
    "dataint2" INTEGER NOT NULL DEFAULT '0',
    "dataint3" INTEGER NOT NULL DEFAULT '0',
    "dataint4" INTEGER NOT NULL DEFAULT '0',
    "x" REAL NOT NULL DEFAULT '0',
    "y" REAL NOT NULL DEFAULT '0',
    "z" REAL NOT NULL DEFAULT '0',
    "o" REAL NOT NULL DEFAULT '0',
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    "comments" VARCHAR(255) NOT NULL
);
ALTER TABLE IF EXISTS "creature_movement_scripts" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "creature_movement_scripts" ADD COLUMN IF NOT EXISTS "delay" BIGINT NOT NULL DEFAULT '0' CHECK ("delay" >= 0);
ALTER TABLE IF EXISTS "creature_movement_scripts" ADD COLUMN IF NOT EXISTS "priority" SMALLINT NOT NULL DEFAULT '0' CHECK ("priority" >= 0);
ALTER TABLE IF EXISTS "creature_movement_scripts" ADD COLUMN IF NOT EXISTS "command" SMALLINT NOT NULL DEFAULT '0' CHECK ("command" >= 0);
ALTER TABLE IF EXISTS "creature_movement_scripts" ADD COLUMN IF NOT EXISTS "datalong" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong" >= 0);
ALTER TABLE IF EXISTS "creature_movement_scripts" ADD COLUMN IF NOT EXISTS "datalong2" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong2" >= 0);
ALTER TABLE IF EXISTS "creature_movement_scripts" ADD COLUMN IF NOT EXISTS "datalong3" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong3" >= 0);
ALTER TABLE IF EXISTS "creature_movement_scripts" ADD COLUMN IF NOT EXISTS "datalong4" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong4" >= 0);
ALTER TABLE IF EXISTS "creature_movement_scripts" ADD COLUMN IF NOT EXISTS "target_param1" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param1" >= 0);
ALTER TABLE IF EXISTS "creature_movement_scripts" ADD COLUMN IF NOT EXISTS "target_param2" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param2" >= 0);
ALTER TABLE IF EXISTS "creature_movement_scripts" ADD COLUMN IF NOT EXISTS "target_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("target_type" >= 0);
ALTER TABLE IF EXISTS "creature_movement_scripts" ADD COLUMN IF NOT EXISTS "data_flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("data_flags" >= 0);
ALTER TABLE IF EXISTS "creature_movement_scripts" ADD COLUMN IF NOT EXISTS "dataint" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_movement_scripts" ADD COLUMN IF NOT EXISTS "dataint2" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_movement_scripts" ADD COLUMN IF NOT EXISTS "dataint3" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_movement_scripts" ADD COLUMN IF NOT EXISTS "dataint4" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_movement_scripts" ADD COLUMN IF NOT EXISTS "x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_movement_scripts" ADD COLUMN IF NOT EXISTS "y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_movement_scripts" ADD COLUMN IF NOT EXISTS "z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_movement_scripts" ADD COLUMN IF NOT EXISTS "o" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_movement_scripts" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);
ALTER TABLE IF EXISTS "creature_movement_scripts" ADD COLUMN IF NOT EXISTS "comments" VARCHAR(255) NOT NULL;

CREATE TABLE IF NOT EXISTS "creature_movement_special" (
    "id" BIGINT NOT NULL CHECK ("id" >= 0),
    "point" BIGINT NOT NULL DEFAULT '0' CHECK ("point" >= 0),
    "position_x" REAL NOT NULL DEFAULT '0',
    "position_y" REAL NOT NULL DEFAULT '0',
    "position_z" REAL NOT NULL DEFAULT '0',
    "orientation" REAL NOT NULL DEFAULT '0',
    "waittime" BIGINT NOT NULL DEFAULT '0' CHECK ("waittime" >= 0),
    "wander_distance" REAL NOT NULL DEFAULT '0' CHECK ("wander_distance" >= 0),
    "script_id" BIGINT NOT NULL DEFAULT '0' CHECK ("script_id" >= 0),
    "path_id" BIGINT NOT NULL DEFAULT '0' CHECK ("path_id" >= 0),
    PRIMARY KEY ("id", "point")
);
ALTER TABLE IF EXISTS "creature_movement_special" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "creature_movement_special" ADD COLUMN IF NOT EXISTS "point" BIGINT NOT NULL DEFAULT '0' CHECK ("point" >= 0);
ALTER TABLE IF EXISTS "creature_movement_special" ADD COLUMN IF NOT EXISTS "position_x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_movement_special" ADD COLUMN IF NOT EXISTS "position_y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_movement_special" ADD COLUMN IF NOT EXISTS "position_z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_movement_special" ADD COLUMN IF NOT EXISTS "orientation" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_movement_special" ADD COLUMN IF NOT EXISTS "waittime" BIGINT NOT NULL DEFAULT '0' CHECK ("waittime" >= 0);
ALTER TABLE IF EXISTS "creature_movement_special" ADD COLUMN IF NOT EXISTS "wander_distance" REAL NOT NULL DEFAULT '0' CHECK ("wander_distance" >= 0);
ALTER TABLE IF EXISTS "creature_movement_special" ADD COLUMN IF NOT EXISTS "script_id" BIGINT NOT NULL DEFAULT '0' CHECK ("script_id" >= 0);
ALTER TABLE IF EXISTS "creature_movement_special" ADD COLUMN IF NOT EXISTS "path_id" BIGINT NOT NULL DEFAULT '0' CHECK ("path_id" >= 0);

CREATE TABLE IF NOT EXISTS "creature_movement_template" (
    "entry" BIGINT NOT NULL CHECK ("entry" >= 0),
    "point" BIGINT NOT NULL DEFAULT '0' CHECK ("point" >= 0),
    "position_x" REAL NOT NULL DEFAULT '0',
    "position_y" REAL NOT NULL DEFAULT '0',
    "position_z" REAL NOT NULL DEFAULT '0',
    "orientation" REAL NOT NULL DEFAULT '0',
    "waittime" BIGINT NOT NULL DEFAULT '0' CHECK ("waittime" >= 0),
    "wander_distance" REAL NOT NULL DEFAULT '0' CHECK ("wander_distance" >= 0),
    "script_id" BIGINT NOT NULL DEFAULT '0' CHECK ("script_id" >= 0),
    "path_id" BIGINT NOT NULL DEFAULT '0' CHECK ("path_id" >= 0),
    PRIMARY KEY ("entry", "point")
);
ALTER TABLE IF EXISTS "creature_movement_template" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "creature_movement_template" ADD COLUMN IF NOT EXISTS "point" BIGINT NOT NULL DEFAULT '0' CHECK ("point" >= 0);
ALTER TABLE IF EXISTS "creature_movement_template" ADD COLUMN IF NOT EXISTS "position_x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_movement_template" ADD COLUMN IF NOT EXISTS "position_y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_movement_template" ADD COLUMN IF NOT EXISTS "position_z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_movement_template" ADD COLUMN IF NOT EXISTS "orientation" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_movement_template" ADD COLUMN IF NOT EXISTS "waittime" BIGINT NOT NULL DEFAULT '0' CHECK ("waittime" >= 0);
ALTER TABLE IF EXISTS "creature_movement_template" ADD COLUMN IF NOT EXISTS "wander_distance" REAL NOT NULL DEFAULT '0' CHECK ("wander_distance" >= 0);
ALTER TABLE IF EXISTS "creature_movement_template" ADD COLUMN IF NOT EXISTS "script_id" BIGINT NOT NULL DEFAULT '0' CHECK ("script_id" >= 0);
ALTER TABLE IF EXISTS "creature_movement_template" ADD COLUMN IF NOT EXISTS "path_id" BIGINT NOT NULL DEFAULT '0' CHECK ("path_id" >= 0);

CREATE TABLE IF NOT EXISTS "creature_onkill_reputation" (
    "creature_id" BIGINT NOT NULL DEFAULT '0' CHECK ("creature_id" >= 0),
    "patch" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch" >= 0),
    "RewOnKillRepFaction1" SMALLINT NOT NULL DEFAULT '0',
    "RewOnKillRepFaction2" SMALLINT NOT NULL DEFAULT '0',
    "MaxStanding1" SMALLINT NOT NULL DEFAULT '0',
    "IsTeamAward1" SMALLINT NOT NULL DEFAULT '0',
    "RewOnKillRepValue1" INTEGER NOT NULL DEFAULT '0',
    "MaxStanding2" SMALLINT NOT NULL DEFAULT '0',
    "IsTeamAward2" SMALLINT NOT NULL DEFAULT '0',
    "RewOnKillRepValue2" INTEGER NOT NULL DEFAULT '0',
    "TeamDependent" SMALLINT NOT NULL DEFAULT '0' CHECK ("TeamDependent" >= 0),
    PRIMARY KEY ("creature_id", "patch")
);
ALTER TABLE IF EXISTS "creature_onkill_reputation" ADD COLUMN IF NOT EXISTS "creature_id" BIGINT NOT NULL DEFAULT '0' CHECK ("creature_id" >= 0);
ALTER TABLE IF EXISTS "creature_onkill_reputation" ADD COLUMN IF NOT EXISTS "patch" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch" >= 0);
ALTER TABLE IF EXISTS "creature_onkill_reputation" ADD COLUMN IF NOT EXISTS "RewOnKillRepFaction1" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_onkill_reputation" ADD COLUMN IF NOT EXISTS "RewOnKillRepFaction2" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_onkill_reputation" ADD COLUMN IF NOT EXISTS "MaxStanding1" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_onkill_reputation" ADD COLUMN IF NOT EXISTS "IsTeamAward1" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_onkill_reputation" ADD COLUMN IF NOT EXISTS "RewOnKillRepValue1" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_onkill_reputation" ADD COLUMN IF NOT EXISTS "MaxStanding2" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_onkill_reputation" ADD COLUMN IF NOT EXISTS "IsTeamAward2" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_onkill_reputation" ADD COLUMN IF NOT EXISTS "RewOnKillRepValue2" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_onkill_reputation" ADD COLUMN IF NOT EXISTS "TeamDependent" SMALLINT NOT NULL DEFAULT '0' CHECK ("TeamDependent" >= 0);

CREATE TABLE IF NOT EXISTS "creature_questrelation" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "quest" BIGINT NOT NULL DEFAULT '0' CHECK ("quest" >= 0),
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0),
    PRIMARY KEY ("id", "quest")
);
ALTER TABLE IF EXISTS "creature_questrelation" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "creature_questrelation" ADD COLUMN IF NOT EXISTS "quest" BIGINT NOT NULL DEFAULT '0' CHECK ("quest" >= 0);
ALTER TABLE IF EXISTS "creature_questrelation" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);
ALTER TABLE IF EXISTS "creature_questrelation" ADD COLUMN IF NOT EXISTS "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0);

CREATE TABLE IF NOT EXISTS "creature_spells" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "name" VARCHAR(255) NOT NULL DEFAULT '',
    "spellId_1" INTEGER NOT NULL DEFAULT '0' CHECK ("spellId_1" >= 0),
    "probability_1" SMALLINT NOT NULL DEFAULT '100' CHECK ("probability_1" >= 0),
    "castTarget_1" SMALLINT NOT NULL DEFAULT '1' CHECK ("castTarget_1" >= 0),
    "targetParam1_1" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam1_1" >= 0),
    "targetParam2_1" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam2_1" >= 0),
    "castFlags_1" INTEGER NOT NULL DEFAULT '0' CHECK ("castFlags_1" >= 0),
    "delayInitialMin_1" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMin_1" >= 0),
    "delayInitialMax_1" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMax_1" >= 0),
    "delayRepeatMin_1" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMin_1" >= 0),
    "delayRepeatMax_1" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMax_1" >= 0),
    "scriptId_1" BIGINT NOT NULL DEFAULT '0' CHECK ("scriptId_1" >= 0),
    "spellId_2" INTEGER NOT NULL DEFAULT '0' CHECK ("spellId_2" >= 0),
    "probability_2" SMALLINT NOT NULL DEFAULT '100' CHECK ("probability_2" >= 0),
    "castTarget_2" SMALLINT NOT NULL DEFAULT '1' CHECK ("castTarget_2" >= 0),
    "targetParam1_2" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam1_2" >= 0),
    "targetParam2_2" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam2_2" >= 0),
    "castFlags_2" INTEGER NOT NULL DEFAULT '0' CHECK ("castFlags_2" >= 0),
    "delayInitialMin_2" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMin_2" >= 0),
    "delayInitialMax_2" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMax_2" >= 0),
    "delayRepeatMin_2" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMin_2" >= 0),
    "delayRepeatMax_2" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMax_2" >= 0),
    "scriptId_2" BIGINT NOT NULL DEFAULT '0' CHECK ("scriptId_2" >= 0),
    "spellId_3" INTEGER NOT NULL DEFAULT '0' CHECK ("spellId_3" >= 0),
    "probability_3" SMALLINT NOT NULL DEFAULT '100' CHECK ("probability_3" >= 0),
    "castTarget_3" SMALLINT NOT NULL DEFAULT '1' CHECK ("castTarget_3" >= 0),
    "targetParam1_3" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam1_3" >= 0),
    "targetParam2_3" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam2_3" >= 0),
    "castFlags_3" INTEGER NOT NULL DEFAULT '0' CHECK ("castFlags_3" >= 0),
    "delayInitialMin_3" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMin_3" >= 0),
    "delayInitialMax_3" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMax_3" >= 0),
    "delayRepeatMin_3" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMin_3" >= 0),
    "delayRepeatMax_3" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMax_3" >= 0),
    "scriptId_3" BIGINT NOT NULL DEFAULT '0' CHECK ("scriptId_3" >= 0),
    "spellId_4" INTEGER NOT NULL DEFAULT '0' CHECK ("spellId_4" >= 0),
    "probability_4" SMALLINT NOT NULL DEFAULT '100' CHECK ("probability_4" >= 0),
    "castTarget_4" SMALLINT NOT NULL DEFAULT '1' CHECK ("castTarget_4" >= 0),
    "targetParam1_4" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam1_4" >= 0),
    "targetParam2_4" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam2_4" >= 0),
    "castFlags_4" INTEGER NOT NULL DEFAULT '0' CHECK ("castFlags_4" >= 0),
    "delayInitialMin_4" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMin_4" >= 0),
    "delayInitialMax_4" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMax_4" >= 0),
    "delayRepeatMin_4" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMin_4" >= 0),
    "delayRepeatMax_4" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMax_4" >= 0),
    "scriptId_4" BIGINT NOT NULL DEFAULT '0' CHECK ("scriptId_4" >= 0),
    "spellId_5" INTEGER NOT NULL DEFAULT '0' CHECK ("spellId_5" >= 0),
    "probability_5" SMALLINT NOT NULL DEFAULT '100' CHECK ("probability_5" >= 0),
    "castTarget_5" SMALLINT NOT NULL DEFAULT '1' CHECK ("castTarget_5" >= 0),
    "targetParam1_5" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam1_5" >= 0),
    "targetParam2_5" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam2_5" >= 0),
    "castFlags_5" INTEGER NOT NULL DEFAULT '0' CHECK ("castFlags_5" >= 0),
    "delayInitialMin_5" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMin_5" >= 0),
    "delayInitialMax_5" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMax_5" >= 0),
    "delayRepeatMin_5" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMin_5" >= 0),
    "delayRepeatMax_5" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMax_5" >= 0),
    "scriptId_5" BIGINT NOT NULL DEFAULT '0' CHECK ("scriptId_5" >= 0),
    "spellId_6" INTEGER NOT NULL DEFAULT '0' CHECK ("spellId_6" >= 0),
    "probability_6" SMALLINT NOT NULL DEFAULT '100' CHECK ("probability_6" >= 0),
    "castTarget_6" SMALLINT NOT NULL DEFAULT '1' CHECK ("castTarget_6" >= 0),
    "targetParam1_6" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam1_6" >= 0),
    "targetParam2_6" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam2_6" >= 0),
    "castFlags_6" INTEGER NOT NULL DEFAULT '0' CHECK ("castFlags_6" >= 0),
    "delayInitialMin_6" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMin_6" >= 0),
    "delayInitialMax_6" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMax_6" >= 0),
    "delayRepeatMin_6" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMin_6" >= 0),
    "delayRepeatMax_6" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMax_6" >= 0),
    "scriptId_6" BIGINT NOT NULL DEFAULT '0' CHECK ("scriptId_6" >= 0),
    "spellId_7" INTEGER NOT NULL DEFAULT '0' CHECK ("spellId_7" >= 0),
    "probability_7" SMALLINT NOT NULL DEFAULT '100' CHECK ("probability_7" >= 0),
    "castTarget_7" SMALLINT NOT NULL DEFAULT '1' CHECK ("castTarget_7" >= 0),
    "targetParam1_7" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam1_7" >= 0),
    "targetParam2_7" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam2_7" >= 0),
    "castFlags_7" INTEGER NOT NULL DEFAULT '0' CHECK ("castFlags_7" >= 0),
    "delayInitialMin_7" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMin_7" >= 0),
    "delayInitialMax_7" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMax_7" >= 0),
    "delayRepeatMin_7" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMin_7" >= 0),
    "delayRepeatMax_7" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMax_7" >= 0),
    "scriptId_7" BIGINT NOT NULL DEFAULT '0' CHECK ("scriptId_7" >= 0),
    "spellId_8" INTEGER NOT NULL DEFAULT '0' CHECK ("spellId_8" >= 0),
    "probability_8" SMALLINT NOT NULL DEFAULT '100' CHECK ("probability_8" >= 0),
    "castTarget_8" SMALLINT NOT NULL DEFAULT '1' CHECK ("castTarget_8" >= 0),
    "targetParam1_8" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam1_8" >= 0),
    "targetParam2_8" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam2_8" >= 0),
    "castFlags_8" INTEGER NOT NULL DEFAULT '0' CHECK ("castFlags_8" >= 0),
    "delayInitialMin_8" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMin_8" >= 0),
    "delayInitialMax_8" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMax_8" >= 0),
    "delayRepeatMin_8" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMin_8" >= 0),
    "delayRepeatMax_8" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMax_8" >= 0),
    "scriptId_8" BIGINT NOT NULL DEFAULT '0' CHECK ("scriptId_8" >= 0),
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "name" VARCHAR(255) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "spellId_1" INTEGER NOT NULL DEFAULT '0' CHECK ("spellId_1" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "probability_1" SMALLINT NOT NULL DEFAULT '100' CHECK ("probability_1" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "castTarget_1" SMALLINT NOT NULL DEFAULT '1' CHECK ("castTarget_1" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "targetParam1_1" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam1_1" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "targetParam2_1" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam2_1" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "castFlags_1" INTEGER NOT NULL DEFAULT '0' CHECK ("castFlags_1" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayInitialMin_1" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMin_1" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayInitialMax_1" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMax_1" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayRepeatMin_1" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMin_1" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayRepeatMax_1" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMax_1" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "scriptId_1" BIGINT NOT NULL DEFAULT '0' CHECK ("scriptId_1" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "spellId_2" INTEGER NOT NULL DEFAULT '0' CHECK ("spellId_2" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "probability_2" SMALLINT NOT NULL DEFAULT '100' CHECK ("probability_2" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "castTarget_2" SMALLINT NOT NULL DEFAULT '1' CHECK ("castTarget_2" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "targetParam1_2" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam1_2" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "targetParam2_2" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam2_2" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "castFlags_2" INTEGER NOT NULL DEFAULT '0' CHECK ("castFlags_2" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayInitialMin_2" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMin_2" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayInitialMax_2" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMax_2" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayRepeatMin_2" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMin_2" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayRepeatMax_2" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMax_2" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "scriptId_2" BIGINT NOT NULL DEFAULT '0' CHECK ("scriptId_2" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "spellId_3" INTEGER NOT NULL DEFAULT '0' CHECK ("spellId_3" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "probability_3" SMALLINT NOT NULL DEFAULT '100' CHECK ("probability_3" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "castTarget_3" SMALLINT NOT NULL DEFAULT '1' CHECK ("castTarget_3" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "targetParam1_3" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam1_3" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "targetParam2_3" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam2_3" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "castFlags_3" INTEGER NOT NULL DEFAULT '0' CHECK ("castFlags_3" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayInitialMin_3" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMin_3" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayInitialMax_3" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMax_3" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayRepeatMin_3" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMin_3" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayRepeatMax_3" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMax_3" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "scriptId_3" BIGINT NOT NULL DEFAULT '0' CHECK ("scriptId_3" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "spellId_4" INTEGER NOT NULL DEFAULT '0' CHECK ("spellId_4" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "probability_4" SMALLINT NOT NULL DEFAULT '100' CHECK ("probability_4" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "castTarget_4" SMALLINT NOT NULL DEFAULT '1' CHECK ("castTarget_4" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "targetParam1_4" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam1_4" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "targetParam2_4" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam2_4" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "castFlags_4" INTEGER NOT NULL DEFAULT '0' CHECK ("castFlags_4" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayInitialMin_4" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMin_4" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayInitialMax_4" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMax_4" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayRepeatMin_4" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMin_4" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayRepeatMax_4" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMax_4" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "scriptId_4" BIGINT NOT NULL DEFAULT '0' CHECK ("scriptId_4" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "spellId_5" INTEGER NOT NULL DEFAULT '0' CHECK ("spellId_5" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "probability_5" SMALLINT NOT NULL DEFAULT '100' CHECK ("probability_5" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "castTarget_5" SMALLINT NOT NULL DEFAULT '1' CHECK ("castTarget_5" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "targetParam1_5" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam1_5" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "targetParam2_5" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam2_5" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "castFlags_5" INTEGER NOT NULL DEFAULT '0' CHECK ("castFlags_5" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayInitialMin_5" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMin_5" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayInitialMax_5" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMax_5" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayRepeatMin_5" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMin_5" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayRepeatMax_5" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMax_5" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "scriptId_5" BIGINT NOT NULL DEFAULT '0' CHECK ("scriptId_5" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "spellId_6" INTEGER NOT NULL DEFAULT '0' CHECK ("spellId_6" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "probability_6" SMALLINT NOT NULL DEFAULT '100' CHECK ("probability_6" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "castTarget_6" SMALLINT NOT NULL DEFAULT '1' CHECK ("castTarget_6" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "targetParam1_6" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam1_6" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "targetParam2_6" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam2_6" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "castFlags_6" INTEGER NOT NULL DEFAULT '0' CHECK ("castFlags_6" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayInitialMin_6" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMin_6" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayInitialMax_6" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMax_6" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayRepeatMin_6" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMin_6" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayRepeatMax_6" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMax_6" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "scriptId_6" BIGINT NOT NULL DEFAULT '0' CHECK ("scriptId_6" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "spellId_7" INTEGER NOT NULL DEFAULT '0' CHECK ("spellId_7" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "probability_7" SMALLINT NOT NULL DEFAULT '100' CHECK ("probability_7" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "castTarget_7" SMALLINT NOT NULL DEFAULT '1' CHECK ("castTarget_7" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "targetParam1_7" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam1_7" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "targetParam2_7" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam2_7" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "castFlags_7" INTEGER NOT NULL DEFAULT '0' CHECK ("castFlags_7" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayInitialMin_7" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMin_7" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayInitialMax_7" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMax_7" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayRepeatMin_7" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMin_7" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayRepeatMax_7" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMax_7" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "scriptId_7" BIGINT NOT NULL DEFAULT '0' CHECK ("scriptId_7" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "spellId_8" INTEGER NOT NULL DEFAULT '0' CHECK ("spellId_8" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "probability_8" SMALLINT NOT NULL DEFAULT '100' CHECK ("probability_8" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "castTarget_8" SMALLINT NOT NULL DEFAULT '1' CHECK ("castTarget_8" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "targetParam1_8" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam1_8" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "targetParam2_8" INTEGER NOT NULL DEFAULT '0' CHECK ("targetParam2_8" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "castFlags_8" INTEGER NOT NULL DEFAULT '0' CHECK ("castFlags_8" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayInitialMin_8" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMin_8" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayInitialMax_8" INTEGER NOT NULL DEFAULT '0' CHECK ("delayInitialMax_8" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayRepeatMin_8" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMin_8" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "delayRepeatMax_8" INTEGER NOT NULL DEFAULT '0' CHECK ("delayRepeatMax_8" >= 0);
ALTER TABLE IF EXISTS "creature_spells" ADD COLUMN IF NOT EXISTS "scriptId_8" BIGINT NOT NULL DEFAULT '0' CHECK ("scriptId_8" >= 0);

CREATE TABLE IF NOT EXISTS "creature_spells_scripts" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "delay" BIGINT NOT NULL DEFAULT '0' CHECK ("delay" >= 0),
    "priority" SMALLINT NOT NULL DEFAULT '0' CHECK ("priority" >= 0),
    "command" SMALLINT NOT NULL DEFAULT '0' CHECK ("command" >= 0),
    "datalong" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong" >= 0),
    "datalong2" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong2" >= 0),
    "datalong3" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong3" >= 0),
    "datalong4" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong4" >= 0),
    "target_param1" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param1" >= 0),
    "target_param2" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param2" >= 0),
    "target_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("target_type" >= 0),
    "data_flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("data_flags" >= 0),
    "dataint" INTEGER NOT NULL DEFAULT '0',
    "dataint2" INTEGER NOT NULL DEFAULT '0',
    "dataint3" INTEGER NOT NULL DEFAULT '0',
    "dataint4" INTEGER NOT NULL DEFAULT '0',
    "x" REAL NOT NULL DEFAULT '0',
    "y" REAL NOT NULL DEFAULT '0',
    "z" REAL NOT NULL DEFAULT '0',
    "o" REAL NOT NULL DEFAULT '0',
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    "comments" VARCHAR(255) NOT NULL
);
ALTER TABLE IF EXISTS "creature_spells_scripts" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "creature_spells_scripts" ADD COLUMN IF NOT EXISTS "delay" BIGINT NOT NULL DEFAULT '0' CHECK ("delay" >= 0);
ALTER TABLE IF EXISTS "creature_spells_scripts" ADD COLUMN IF NOT EXISTS "priority" SMALLINT NOT NULL DEFAULT '0' CHECK ("priority" >= 0);
ALTER TABLE IF EXISTS "creature_spells_scripts" ADD COLUMN IF NOT EXISTS "command" SMALLINT NOT NULL DEFAULT '0' CHECK ("command" >= 0);
ALTER TABLE IF EXISTS "creature_spells_scripts" ADD COLUMN IF NOT EXISTS "datalong" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong" >= 0);
ALTER TABLE IF EXISTS "creature_spells_scripts" ADD COLUMN IF NOT EXISTS "datalong2" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong2" >= 0);
ALTER TABLE IF EXISTS "creature_spells_scripts" ADD COLUMN IF NOT EXISTS "datalong3" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong3" >= 0);
ALTER TABLE IF EXISTS "creature_spells_scripts" ADD COLUMN IF NOT EXISTS "datalong4" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong4" >= 0);
ALTER TABLE IF EXISTS "creature_spells_scripts" ADD COLUMN IF NOT EXISTS "target_param1" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param1" >= 0);
ALTER TABLE IF EXISTS "creature_spells_scripts" ADD COLUMN IF NOT EXISTS "target_param2" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param2" >= 0);
ALTER TABLE IF EXISTS "creature_spells_scripts" ADD COLUMN IF NOT EXISTS "target_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("target_type" >= 0);
ALTER TABLE IF EXISTS "creature_spells_scripts" ADD COLUMN IF NOT EXISTS "data_flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("data_flags" >= 0);
ALTER TABLE IF EXISTS "creature_spells_scripts" ADD COLUMN IF NOT EXISTS "dataint" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_spells_scripts" ADD COLUMN IF NOT EXISTS "dataint2" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_spells_scripts" ADD COLUMN IF NOT EXISTS "dataint3" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_spells_scripts" ADD COLUMN IF NOT EXISTS "dataint4" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_spells_scripts" ADD COLUMN IF NOT EXISTS "x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_spells_scripts" ADD COLUMN IF NOT EXISTS "y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_spells_scripts" ADD COLUMN IF NOT EXISTS "z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_spells_scripts" ADD COLUMN IF NOT EXISTS "o" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_spells_scripts" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);
ALTER TABLE IF EXISTS "creature_spells_scripts" ADD COLUMN IF NOT EXISTS "comments" VARCHAR(255) NOT NULL;

CREATE TABLE IF NOT EXISTS "creature_template" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "patch" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch" >= 0),
    "name" CHAR(100) NOT NULL DEFAULT '0',
    "subname" CHAR(100) DEFAULT NULL,
    "level_min" SMALLINT NOT NULL DEFAULT '1' CHECK ("level_min" >= 0),
    "level_max" SMALLINT NOT NULL DEFAULT '1' CHECK ("level_max" >= 0),
    "faction" INTEGER NOT NULL DEFAULT '0' CHECK ("faction" >= 0),
    "npc_flags" BIGINT NOT NULL DEFAULT '0' CHECK ("npc_flags" >= 0),
    "gossip_menu_id" BIGINT NOT NULL DEFAULT '0' CHECK ("gossip_menu_id" >= 0),
    "display_id1" BIGINT NOT NULL DEFAULT '0' CHECK ("display_id1" >= 0),
    "display_id2" BIGINT NOT NULL DEFAULT '0' CHECK ("display_id2" >= 0),
    "display_id3" BIGINT NOT NULL DEFAULT '0' CHECK ("display_id3" >= 0),
    "display_id4" BIGINT NOT NULL DEFAULT '0' CHECK ("display_id4" >= 0),
    "display_scale1" REAL NOT NULL DEFAULT '0',
    "display_scale2" REAL NOT NULL DEFAULT '0',
    "display_scale3" REAL NOT NULL DEFAULT '0',
    "display_scale4" REAL NOT NULL DEFAULT '0',
    "display_probability1" INTEGER NOT NULL DEFAULT '0' CHECK ("display_probability1" >= 0),
    "display_probability2" INTEGER NOT NULL DEFAULT '0' CHECK ("display_probability2" >= 0),
    "display_probability3" INTEGER NOT NULL DEFAULT '0' CHECK ("display_probability3" >= 0),
    "display_probability4" INTEGER NOT NULL DEFAULT '0' CHECK ("display_probability4" >= 0),
    "display_total_probability" INTEGER NOT NULL DEFAULT '0' CHECK ("display_total_probability" >= 0),
    "mount_display_id" INTEGER NOT NULL DEFAULT '0' CHECK ("mount_display_id" >= 0),
    "speed_walk" REAL NOT NULL DEFAULT '1',
    "speed_run" REAL NOT NULL DEFAULT '1.14286',
    "detection_range" REAL NOT NULL DEFAULT '18',
    "call_for_help_range" REAL NOT NULL DEFAULT '5',
    "leash_range" REAL NOT NULL DEFAULT '0',
    "type" SMALLINT NOT NULL DEFAULT '0' CHECK ("type" >= 0),
    "pet_family" SMALLINT NOT NULL DEFAULT '0' CHECK ("pet_family" >= 0),
    "rank" SMALLINT NOT NULL DEFAULT '0' CHECK ("rank" >= 0),
    "unit_class" SMALLINT NOT NULL DEFAULT '0' CHECK ("unit_class" >= 0),
    "xp_multiplier" REAL NOT NULL DEFAULT '1',
    "health_multiplier" REAL NOT NULL DEFAULT '1',
    "mana_multiplier" REAL NOT NULL DEFAULT '1',
    "armor_multiplier" REAL NOT NULL DEFAULT '1',
    "damage_multiplier" REAL NOT NULL DEFAULT '1',
    "damage_variance" REAL NOT NULL DEFAULT '0.14',
    "damage_school" SMALLINT NOT NULL DEFAULT '0' CHECK ("damage_school" >= 0),
    "base_attack_time" BIGINT NOT NULL DEFAULT '2000' CHECK ("base_attack_time" >= 0),
    "ranged_attack_time" BIGINT NOT NULL DEFAULT '2000' CHECK ("ranged_attack_time" >= 0),
    "holy_res" SMALLINT NOT NULL DEFAULT '0',
    "fire_res" SMALLINT NOT NULL DEFAULT '0',
    "nature_res" SMALLINT NOT NULL DEFAULT '0',
    "frost_res" SMALLINT NOT NULL DEFAULT '0',
    "shadow_res" SMALLINT NOT NULL DEFAULT '0',
    "arcane_res" SMALLINT NOT NULL DEFAULT '0',
    "trainer_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("trainer_type" >= 0),
    "trainer_spell" INTEGER NOT NULL DEFAULT '0' CHECK ("trainer_spell" >= 0),
    "trainer_class" SMALLINT NOT NULL DEFAULT '0' CHECK ("trainer_class" >= 0),
    "trainer_race" SMALLINT NOT NULL DEFAULT '0' CHECK ("trainer_race" >= 0),
    "loot_id" BIGINT NOT NULL DEFAULT '0' CHECK ("loot_id" >= 0),
    "pickpocket_loot_id" BIGINT NOT NULL DEFAULT '0' CHECK ("pickpocket_loot_id" >= 0),
    "skinning_loot_id" BIGINT NOT NULL DEFAULT '0' CHECK ("skinning_loot_id" >= 0),
    "gold_min" BIGINT NOT NULL DEFAULT '0' CHECK ("gold_min" >= 0),
    "gold_max" BIGINT NOT NULL DEFAULT '0' CHECK ("gold_max" >= 0),
    "spell_id1" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_id1" >= 0),
    "spell_id2" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_id2" >= 0),
    "spell_id3" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_id3" >= 0),
    "spell_id4" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_id4" >= 0),
    "spell_list_id" BIGINT NOT NULL DEFAULT '0' CHECK ("spell_list_id" >= 0),
    "pet_spell_list_id" BIGINT NOT NULL DEFAULT '0' CHECK ("pet_spell_list_id" >= 0),
    "spawn_spell_id" INTEGER NOT NULL DEFAULT '0' CHECK ("spawn_spell_id" >= 0),
    "auras" TEXT,
    "ai_name" CHAR(64) NOT NULL DEFAULT '',
    "movement_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("movement_type" >= 0),
    "inhabit_type" SMALLINT NOT NULL DEFAULT '3' CHECK ("inhabit_type" >= 0),
    "civilian" SMALLINT NOT NULL DEFAULT '0' CHECK ("civilian" >= 0),
    "racial_leader" SMALLINT NOT NULL DEFAULT '0' CHECK ("racial_leader" >= 0),
    "equipment_id" BIGINT NOT NULL DEFAULT '0' CHECK ("equipment_id" >= 0),
    "trainer_id" BIGINT NOT NULL DEFAULT '0' CHECK ("trainer_id" >= 0),
    "vendor_id" BIGINT NOT NULL DEFAULT '0' CHECK ("vendor_id" >= 0),
    "mechanic_immune_mask" BIGINT NOT NULL DEFAULT '0' CHECK ("mechanic_immune_mask" >= 0),
    "school_immune_mask" BIGINT NOT NULL DEFAULT '0' CHECK ("school_immune_mask" >= 0),
    "immunity_flags" BIGINT NOT NULL DEFAULT '0' CHECK ("immunity_flags" >= 0),
    "static_flags1" BIGINT NOT NULL DEFAULT '0' CHECK ("static_flags1" >= 0),
    "static_flags2" BIGINT NOT NULL DEFAULT '0' CHECK ("static_flags2" >= 0),
    "flags_extra" BIGINT NOT NULL DEFAULT '0' CHECK ("flags_extra" >= 0),
    "script_name" CHAR(64) NOT NULL DEFAULT '',
    PRIMARY KEY ("entry", "patch")
);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "patch" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "name" CHAR(100) NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "subname" CHAR(100) DEFAULT NULL;
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "level_min" SMALLINT NOT NULL DEFAULT '1' CHECK ("level_min" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "level_max" SMALLINT NOT NULL DEFAULT '1' CHECK ("level_max" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "faction" INTEGER NOT NULL DEFAULT '0' CHECK ("faction" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "npc_flags" BIGINT NOT NULL DEFAULT '0' CHECK ("npc_flags" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "gossip_menu_id" BIGINT NOT NULL DEFAULT '0' CHECK ("gossip_menu_id" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "display_id1" BIGINT NOT NULL DEFAULT '0' CHECK ("display_id1" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "display_id2" BIGINT NOT NULL DEFAULT '0' CHECK ("display_id2" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "display_id3" BIGINT NOT NULL DEFAULT '0' CHECK ("display_id3" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "display_id4" BIGINT NOT NULL DEFAULT '0' CHECK ("display_id4" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "display_scale1" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "display_scale2" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "display_scale3" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "display_scale4" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "display_probability1" INTEGER NOT NULL DEFAULT '0' CHECK ("display_probability1" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "display_probability2" INTEGER NOT NULL DEFAULT '0' CHECK ("display_probability2" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "display_probability3" INTEGER NOT NULL DEFAULT '0' CHECK ("display_probability3" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "display_probability4" INTEGER NOT NULL DEFAULT '0' CHECK ("display_probability4" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "display_total_probability" INTEGER NOT NULL DEFAULT '0' CHECK ("display_total_probability" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "mount_display_id" INTEGER NOT NULL DEFAULT '0' CHECK ("mount_display_id" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "speed_walk" REAL NOT NULL DEFAULT '1';
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "speed_run" REAL NOT NULL DEFAULT '1.14286';
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "detection_range" REAL NOT NULL DEFAULT '18';
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "call_for_help_range" REAL NOT NULL DEFAULT '5';
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "leash_range" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "type" SMALLINT NOT NULL DEFAULT '0' CHECK ("type" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "pet_family" SMALLINT NOT NULL DEFAULT '0' CHECK ("pet_family" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "rank" SMALLINT NOT NULL DEFAULT '0' CHECK ("rank" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "unit_class" SMALLINT NOT NULL DEFAULT '0' CHECK ("unit_class" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "xp_multiplier" REAL NOT NULL DEFAULT '1';
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "health_multiplier" REAL NOT NULL DEFAULT '1';
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "mana_multiplier" REAL NOT NULL DEFAULT '1';
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "armor_multiplier" REAL NOT NULL DEFAULT '1';
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "damage_multiplier" REAL NOT NULL DEFAULT '1';
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "damage_variance" REAL NOT NULL DEFAULT '0.14';
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "damage_school" SMALLINT NOT NULL DEFAULT '0' CHECK ("damage_school" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "base_attack_time" BIGINT NOT NULL DEFAULT '2000' CHECK ("base_attack_time" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "ranged_attack_time" BIGINT NOT NULL DEFAULT '2000' CHECK ("ranged_attack_time" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "holy_res" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "fire_res" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "nature_res" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "frost_res" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "shadow_res" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "arcane_res" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "trainer_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("trainer_type" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "trainer_spell" INTEGER NOT NULL DEFAULT '0' CHECK ("trainer_spell" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "trainer_class" SMALLINT NOT NULL DEFAULT '0' CHECK ("trainer_class" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "trainer_race" SMALLINT NOT NULL DEFAULT '0' CHECK ("trainer_race" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "loot_id" BIGINT NOT NULL DEFAULT '0' CHECK ("loot_id" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "pickpocket_loot_id" BIGINT NOT NULL DEFAULT '0' CHECK ("pickpocket_loot_id" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "skinning_loot_id" BIGINT NOT NULL DEFAULT '0' CHECK ("skinning_loot_id" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "gold_min" BIGINT NOT NULL DEFAULT '0' CHECK ("gold_min" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "gold_max" BIGINT NOT NULL DEFAULT '0' CHECK ("gold_max" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "spell_id1" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_id1" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "spell_id2" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_id2" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "spell_id3" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_id3" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "spell_id4" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_id4" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "spell_list_id" BIGINT NOT NULL DEFAULT '0' CHECK ("spell_list_id" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "pet_spell_list_id" BIGINT NOT NULL DEFAULT '0' CHECK ("pet_spell_list_id" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "spawn_spell_id" INTEGER NOT NULL DEFAULT '0' CHECK ("spawn_spell_id" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "auras" TEXT;
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "ai_name" CHAR(64) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "movement_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("movement_type" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "inhabit_type" SMALLINT NOT NULL DEFAULT '3' CHECK ("inhabit_type" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "civilian" SMALLINT NOT NULL DEFAULT '0' CHECK ("civilian" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "racial_leader" SMALLINT NOT NULL DEFAULT '0' CHECK ("racial_leader" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "equipment_id" BIGINT NOT NULL DEFAULT '0' CHECK ("equipment_id" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "trainer_id" BIGINT NOT NULL DEFAULT '0' CHECK ("trainer_id" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "vendor_id" BIGINT NOT NULL DEFAULT '0' CHECK ("vendor_id" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "mechanic_immune_mask" BIGINT NOT NULL DEFAULT '0' CHECK ("mechanic_immune_mask" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "school_immune_mask" BIGINT NOT NULL DEFAULT '0' CHECK ("school_immune_mask" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "immunity_flags" BIGINT NOT NULL DEFAULT '0' CHECK ("immunity_flags" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "static_flags1" BIGINT NOT NULL DEFAULT '0' CHECK ("static_flags1" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "static_flags2" BIGINT NOT NULL DEFAULT '0' CHECK ("static_flags2" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "flags_extra" BIGINT NOT NULL DEFAULT '0' CHECK ("flags_extra" >= 0);
ALTER TABLE IF EXISTS "creature_template" ADD COLUMN IF NOT EXISTS "script_name" CHAR(64) NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS "custom_texts" (
    "entry" INTEGER NOT NULL,
    "content_default" TEXT NOT NULL,
    "content_loc1" TEXT,
    "content_loc2" TEXT,
    "content_loc3" TEXT,
    "content_loc4" TEXT,
    "content_loc5" TEXT,
    "content_loc6" TEXT,
    "content_loc7" TEXT,
    "content_loc8" TEXT,
    "sound" BIGINT NOT NULL DEFAULT '0' CHECK ("sound" >= 0),
    "type" SMALLINT NOT NULL DEFAULT '0' CHECK ("type" >= 0),
    "language" SMALLINT NOT NULL DEFAULT '0' CHECK ("language" >= 0),
    "emote" INTEGER NOT NULL DEFAULT '0' CHECK ("emote" >= 0),
    "comment" TEXT,
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "custom_texts" ADD COLUMN IF NOT EXISTS "entry" INTEGER NOT NULL;
ALTER TABLE IF EXISTS "custom_texts" ADD COLUMN IF NOT EXISTS "content_default" TEXT NOT NULL;
ALTER TABLE IF EXISTS "custom_texts" ADD COLUMN IF NOT EXISTS "content_loc1" TEXT;
ALTER TABLE IF EXISTS "custom_texts" ADD COLUMN IF NOT EXISTS "content_loc2" TEXT;
ALTER TABLE IF EXISTS "custom_texts" ADD COLUMN IF NOT EXISTS "content_loc3" TEXT;
ALTER TABLE IF EXISTS "custom_texts" ADD COLUMN IF NOT EXISTS "content_loc4" TEXT;
ALTER TABLE IF EXISTS "custom_texts" ADD COLUMN IF NOT EXISTS "content_loc5" TEXT;
ALTER TABLE IF EXISTS "custom_texts" ADD COLUMN IF NOT EXISTS "content_loc6" TEXT;
ALTER TABLE IF EXISTS "custom_texts" ADD COLUMN IF NOT EXISTS "content_loc7" TEXT;
ALTER TABLE IF EXISTS "custom_texts" ADD COLUMN IF NOT EXISTS "content_loc8" TEXT;
ALTER TABLE IF EXISTS "custom_texts" ADD COLUMN IF NOT EXISTS "sound" BIGINT NOT NULL DEFAULT '0' CHECK ("sound" >= 0);
ALTER TABLE IF EXISTS "custom_texts" ADD COLUMN IF NOT EXISTS "type" SMALLINT NOT NULL DEFAULT '0' CHECK ("type" >= 0);
ALTER TABLE IF EXISTS "custom_texts" ADD COLUMN IF NOT EXISTS "language" SMALLINT NOT NULL DEFAULT '0' CHECK ("language" >= 0);
ALTER TABLE IF EXISTS "custom_texts" ADD COLUMN IF NOT EXISTS "emote" INTEGER NOT NULL DEFAULT '0' CHECK ("emote" >= 0);
ALTER TABLE IF EXISTS "custom_texts" ADD COLUMN IF NOT EXISTS "comment" TEXT;

CREATE TABLE IF NOT EXISTS "disenchant_loot_template" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "item" BIGINT NOT NULL DEFAULT '0' CHECK ("item" >= 0),
    "ChanceOrQuestChance" REAL NOT NULL DEFAULT '100',
    "groupid" SMALLINT NOT NULL DEFAULT '0' CHECK ("groupid" >= 0),
    "mincountOrRef" INTEGER NOT NULL DEFAULT '1',
    "maxcount" SMALLINT NOT NULL DEFAULT '1' CHECK ("maxcount" >= 0),
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0),
    PRIMARY KEY ("entry", "item")
);
ALTER TABLE IF EXISTS "disenchant_loot_template" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "disenchant_loot_template" ADD COLUMN IF NOT EXISTS "item" BIGINT NOT NULL DEFAULT '0' CHECK ("item" >= 0);
ALTER TABLE IF EXISTS "disenchant_loot_template" ADD COLUMN IF NOT EXISTS "ChanceOrQuestChance" REAL NOT NULL DEFAULT '100';
ALTER TABLE IF EXISTS "disenchant_loot_template" ADD COLUMN IF NOT EXISTS "groupid" SMALLINT NOT NULL DEFAULT '0' CHECK ("groupid" >= 0);
ALTER TABLE IF EXISTS "disenchant_loot_template" ADD COLUMN IF NOT EXISTS "mincountOrRef" INTEGER NOT NULL DEFAULT '1';
ALTER TABLE IF EXISTS "disenchant_loot_template" ADD COLUMN IF NOT EXISTS "maxcount" SMALLINT NOT NULL DEFAULT '1' CHECK ("maxcount" >= 0);
ALTER TABLE IF EXISTS "disenchant_loot_template" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);
ALTER TABLE IF EXISTS "disenchant_loot_template" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);
ALTER TABLE IF EXISTS "disenchant_loot_template" ADD COLUMN IF NOT EXISTS "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0);

CREATE TABLE IF NOT EXISTS "event_scripts" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "delay" BIGINT NOT NULL DEFAULT '0' CHECK ("delay" >= 0),
    "priority" SMALLINT NOT NULL DEFAULT '0' CHECK ("priority" >= 0),
    "command" SMALLINT NOT NULL DEFAULT '0' CHECK ("command" >= 0),
    "datalong" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong" >= 0),
    "datalong2" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong2" >= 0),
    "datalong3" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong3" >= 0),
    "datalong4" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong4" >= 0),
    "target_param1" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param1" >= 0),
    "target_param2" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param2" >= 0),
    "target_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("target_type" >= 0),
    "data_flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("data_flags" >= 0),
    "dataint" INTEGER NOT NULL DEFAULT '0',
    "dataint2" INTEGER NOT NULL DEFAULT '0',
    "dataint3" INTEGER NOT NULL DEFAULT '0',
    "dataint4" INTEGER NOT NULL DEFAULT '0',
    "x" REAL NOT NULL DEFAULT '0',
    "y" REAL NOT NULL DEFAULT '0',
    "z" REAL NOT NULL DEFAULT '0',
    "o" REAL NOT NULL DEFAULT '0',
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    "comments" VARCHAR(255) NOT NULL
);
ALTER TABLE IF EXISTS "event_scripts" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "event_scripts" ADD COLUMN IF NOT EXISTS "delay" BIGINT NOT NULL DEFAULT '0' CHECK ("delay" >= 0);
ALTER TABLE IF EXISTS "event_scripts" ADD COLUMN IF NOT EXISTS "priority" SMALLINT NOT NULL DEFAULT '0' CHECK ("priority" >= 0);
ALTER TABLE IF EXISTS "event_scripts" ADD COLUMN IF NOT EXISTS "command" SMALLINT NOT NULL DEFAULT '0' CHECK ("command" >= 0);
ALTER TABLE IF EXISTS "event_scripts" ADD COLUMN IF NOT EXISTS "datalong" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong" >= 0);
ALTER TABLE IF EXISTS "event_scripts" ADD COLUMN IF NOT EXISTS "datalong2" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong2" >= 0);
ALTER TABLE IF EXISTS "event_scripts" ADD COLUMN IF NOT EXISTS "datalong3" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong3" >= 0);
ALTER TABLE IF EXISTS "event_scripts" ADD COLUMN IF NOT EXISTS "datalong4" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong4" >= 0);
ALTER TABLE IF EXISTS "event_scripts" ADD COLUMN IF NOT EXISTS "target_param1" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param1" >= 0);
ALTER TABLE IF EXISTS "event_scripts" ADD COLUMN IF NOT EXISTS "target_param2" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param2" >= 0);
ALTER TABLE IF EXISTS "event_scripts" ADD COLUMN IF NOT EXISTS "target_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("target_type" >= 0);
ALTER TABLE IF EXISTS "event_scripts" ADD COLUMN IF NOT EXISTS "data_flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("data_flags" >= 0);
ALTER TABLE IF EXISTS "event_scripts" ADD COLUMN IF NOT EXISTS "dataint" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "event_scripts" ADD COLUMN IF NOT EXISTS "dataint2" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "event_scripts" ADD COLUMN IF NOT EXISTS "dataint3" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "event_scripts" ADD COLUMN IF NOT EXISTS "dataint4" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "event_scripts" ADD COLUMN IF NOT EXISTS "x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "event_scripts" ADD COLUMN IF NOT EXISTS "y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "event_scripts" ADD COLUMN IF NOT EXISTS "z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "event_scripts" ADD COLUMN IF NOT EXISTS "o" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "event_scripts" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);
ALTER TABLE IF EXISTS "event_scripts" ADD COLUMN IF NOT EXISTS "comments" VARCHAR(255) NOT NULL;

CREATE TABLE IF NOT EXISTS "exploration_basexp" (
    "level" SMALLINT NOT NULL DEFAULT '0',
    "basexp" INTEGER NOT NULL DEFAULT '0',
    PRIMARY KEY ("level")
);
ALTER TABLE IF EXISTS "exploration_basexp" ADD COLUMN IF NOT EXISTS "level" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "exploration_basexp" ADD COLUMN IF NOT EXISTS "basexp" INTEGER NOT NULL DEFAULT '0';

CREATE TABLE IF NOT EXISTS "faction" (
    "id" INTEGER NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "build" INTEGER NOT NULL DEFAULT '5875' CHECK ("build" >= 0),
    "reputation_list_id" INTEGER NOT NULL DEFAULT '0',
    "base_rep_race_mask1" BIGINT NOT NULL DEFAULT '0' CHECK ("base_rep_race_mask1" >= 0),
    "base_rep_race_mask2" BIGINT NOT NULL DEFAULT '0' CHECK ("base_rep_race_mask2" >= 0),
    "base_rep_race_mask3" BIGINT NOT NULL DEFAULT '0' CHECK ("base_rep_race_mask3" >= 0),
    "base_rep_race_mask4" BIGINT NOT NULL DEFAULT '0' CHECK ("base_rep_race_mask4" >= 0),
    "base_rep_class_mask1" BIGINT NOT NULL DEFAULT '0' CHECK ("base_rep_class_mask1" >= 0),
    "base_rep_class_mask2" BIGINT NOT NULL DEFAULT '0' CHECK ("base_rep_class_mask2" >= 0),
    "base_rep_class_mask3" BIGINT NOT NULL DEFAULT '0' CHECK ("base_rep_class_mask3" >= 0),
    "base_rep_class_mask4" BIGINT NOT NULL DEFAULT '0' CHECK ("base_rep_class_mask4" >= 0),
    "base_rep_value1" INTEGER NOT NULL DEFAULT '0',
    "base_rep_value2" INTEGER NOT NULL DEFAULT '0',
    "base_rep_value3" INTEGER NOT NULL DEFAULT '0',
    "base_rep_value4" INTEGER NOT NULL DEFAULT '0',
    "reputation_flags1" BIGINT NOT NULL DEFAULT '0' CHECK ("reputation_flags1" >= 0),
    "reputation_flags2" BIGINT NOT NULL DEFAULT '0' CHECK ("reputation_flags2" >= 0),
    "reputation_flags3" BIGINT NOT NULL DEFAULT '0' CHECK ("reputation_flags3" >= 0),
    "reputation_flags4" BIGINT NOT NULL DEFAULT '0' CHECK ("reputation_flags4" >= 0),
    "team" BIGINT NOT NULL DEFAULT '0' CHECK ("team" >= 0),
    "name" VARCHAR(256) NOT NULL DEFAULT '',
    "description" VARCHAR(512) NOT NULL DEFAULT '',
    PRIMARY KEY ("id", "build")
);
ALTER TABLE IF EXISTS "faction" ADD COLUMN IF NOT EXISTS "id" INTEGER NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "faction" ADD COLUMN IF NOT EXISTS "build" INTEGER NOT NULL DEFAULT '5875' CHECK ("build" >= 0);
ALTER TABLE IF EXISTS "faction" ADD COLUMN IF NOT EXISTS "reputation_list_id" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "faction" ADD COLUMN IF NOT EXISTS "base_rep_race_mask1" BIGINT NOT NULL DEFAULT '0' CHECK ("base_rep_race_mask1" >= 0);
ALTER TABLE IF EXISTS "faction" ADD COLUMN IF NOT EXISTS "base_rep_race_mask2" BIGINT NOT NULL DEFAULT '0' CHECK ("base_rep_race_mask2" >= 0);
ALTER TABLE IF EXISTS "faction" ADD COLUMN IF NOT EXISTS "base_rep_race_mask3" BIGINT NOT NULL DEFAULT '0' CHECK ("base_rep_race_mask3" >= 0);
ALTER TABLE IF EXISTS "faction" ADD COLUMN IF NOT EXISTS "base_rep_race_mask4" BIGINT NOT NULL DEFAULT '0' CHECK ("base_rep_race_mask4" >= 0);
ALTER TABLE IF EXISTS "faction" ADD COLUMN IF NOT EXISTS "base_rep_class_mask1" BIGINT NOT NULL DEFAULT '0' CHECK ("base_rep_class_mask1" >= 0);
ALTER TABLE IF EXISTS "faction" ADD COLUMN IF NOT EXISTS "base_rep_class_mask2" BIGINT NOT NULL DEFAULT '0' CHECK ("base_rep_class_mask2" >= 0);
ALTER TABLE IF EXISTS "faction" ADD COLUMN IF NOT EXISTS "base_rep_class_mask3" BIGINT NOT NULL DEFAULT '0' CHECK ("base_rep_class_mask3" >= 0);
ALTER TABLE IF EXISTS "faction" ADD COLUMN IF NOT EXISTS "base_rep_class_mask4" BIGINT NOT NULL DEFAULT '0' CHECK ("base_rep_class_mask4" >= 0);
ALTER TABLE IF EXISTS "faction" ADD COLUMN IF NOT EXISTS "base_rep_value1" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "faction" ADD COLUMN IF NOT EXISTS "base_rep_value2" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "faction" ADD COLUMN IF NOT EXISTS "base_rep_value3" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "faction" ADD COLUMN IF NOT EXISTS "base_rep_value4" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "faction" ADD COLUMN IF NOT EXISTS "reputation_flags1" BIGINT NOT NULL DEFAULT '0' CHECK ("reputation_flags1" >= 0);
ALTER TABLE IF EXISTS "faction" ADD COLUMN IF NOT EXISTS "reputation_flags2" BIGINT NOT NULL DEFAULT '0' CHECK ("reputation_flags2" >= 0);
ALTER TABLE IF EXISTS "faction" ADD COLUMN IF NOT EXISTS "reputation_flags3" BIGINT NOT NULL DEFAULT '0' CHECK ("reputation_flags3" >= 0);
ALTER TABLE IF EXISTS "faction" ADD COLUMN IF NOT EXISTS "reputation_flags4" BIGINT NOT NULL DEFAULT '0' CHECK ("reputation_flags4" >= 0);
ALTER TABLE IF EXISTS "faction" ADD COLUMN IF NOT EXISTS "team" BIGINT NOT NULL DEFAULT '0' CHECK ("team" >= 0);
ALTER TABLE IF EXISTS "faction" ADD COLUMN IF NOT EXISTS "name" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "faction" ADD COLUMN IF NOT EXISTS "description" VARCHAR(512) NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS "faction_template" (
    "id" INTEGER NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "build" INTEGER NOT NULL DEFAULT '5875' CHECK ("build" >= 0),
    "faction_id" BIGINT NOT NULL DEFAULT '0' CHECK ("faction_id" >= 0),
    "faction_flags" BIGINT NOT NULL DEFAULT '0' CHECK ("faction_flags" >= 0),
    "our_mask" BIGINT NOT NULL DEFAULT '0' CHECK ("our_mask" >= 0),
    "friendly_mask" BIGINT NOT NULL DEFAULT '0' CHECK ("friendly_mask" >= 0),
    "hostile_mask" BIGINT NOT NULL DEFAULT '0' CHECK ("hostile_mask" >= 0),
    "enemy_faction1" BIGINT NOT NULL DEFAULT '0' CHECK ("enemy_faction1" >= 0),
    "enemy_faction2" BIGINT NOT NULL DEFAULT '0' CHECK ("enemy_faction2" >= 0),
    "enemy_faction3" BIGINT NOT NULL DEFAULT '0' CHECK ("enemy_faction3" >= 0),
    "enemy_faction4" BIGINT NOT NULL DEFAULT '0' CHECK ("enemy_faction4" >= 0),
    "friend_faction1" BIGINT NOT NULL DEFAULT '0' CHECK ("friend_faction1" >= 0),
    "friend_faction2" BIGINT NOT NULL DEFAULT '0' CHECK ("friend_faction2" >= 0),
    "friend_faction3" BIGINT NOT NULL DEFAULT '0' CHECK ("friend_faction3" >= 0),
    "friend_faction4" BIGINT NOT NULL DEFAULT '0' CHECK ("friend_faction4" >= 0),
    PRIMARY KEY ("id", "build")
);
ALTER TABLE IF EXISTS "faction_template" ADD COLUMN IF NOT EXISTS "id" INTEGER NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "faction_template" ADD COLUMN IF NOT EXISTS "build" INTEGER NOT NULL DEFAULT '5875' CHECK ("build" >= 0);
ALTER TABLE IF EXISTS "faction_template" ADD COLUMN IF NOT EXISTS "faction_id" BIGINT NOT NULL DEFAULT '0' CHECK ("faction_id" >= 0);
ALTER TABLE IF EXISTS "faction_template" ADD COLUMN IF NOT EXISTS "faction_flags" BIGINT NOT NULL DEFAULT '0' CHECK ("faction_flags" >= 0);
ALTER TABLE IF EXISTS "faction_template" ADD COLUMN IF NOT EXISTS "our_mask" BIGINT NOT NULL DEFAULT '0' CHECK ("our_mask" >= 0);
ALTER TABLE IF EXISTS "faction_template" ADD COLUMN IF NOT EXISTS "friendly_mask" BIGINT NOT NULL DEFAULT '0' CHECK ("friendly_mask" >= 0);
ALTER TABLE IF EXISTS "faction_template" ADD COLUMN IF NOT EXISTS "hostile_mask" BIGINT NOT NULL DEFAULT '0' CHECK ("hostile_mask" >= 0);
ALTER TABLE IF EXISTS "faction_template" ADD COLUMN IF NOT EXISTS "enemy_faction1" BIGINT NOT NULL DEFAULT '0' CHECK ("enemy_faction1" >= 0);
ALTER TABLE IF EXISTS "faction_template" ADD COLUMN IF NOT EXISTS "enemy_faction2" BIGINT NOT NULL DEFAULT '0' CHECK ("enemy_faction2" >= 0);
ALTER TABLE IF EXISTS "faction_template" ADD COLUMN IF NOT EXISTS "enemy_faction3" BIGINT NOT NULL DEFAULT '0' CHECK ("enemy_faction3" >= 0);
ALTER TABLE IF EXISTS "faction_template" ADD COLUMN IF NOT EXISTS "enemy_faction4" BIGINT NOT NULL DEFAULT '0' CHECK ("enemy_faction4" >= 0);
ALTER TABLE IF EXISTS "faction_template" ADD COLUMN IF NOT EXISTS "friend_faction1" BIGINT NOT NULL DEFAULT '0' CHECK ("friend_faction1" >= 0);
ALTER TABLE IF EXISTS "faction_template" ADD COLUMN IF NOT EXISTS "friend_faction2" BIGINT NOT NULL DEFAULT '0' CHECK ("friend_faction2" >= 0);
ALTER TABLE IF EXISTS "faction_template" ADD COLUMN IF NOT EXISTS "friend_faction3" BIGINT NOT NULL DEFAULT '0' CHECK ("friend_faction3" >= 0);
ALTER TABLE IF EXISTS "faction_template" ADD COLUMN IF NOT EXISTS "friend_faction4" BIGINT NOT NULL DEFAULT '0' CHECK ("friend_faction4" >= 0);

CREATE TABLE IF NOT EXISTS "fishing_loot_template" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "item" BIGINT NOT NULL DEFAULT '0' CHECK ("item" >= 0),
    "ChanceOrQuestChance" REAL NOT NULL DEFAULT '100',
    "groupid" SMALLINT NOT NULL DEFAULT '0' CHECK ("groupid" >= 0),
    "mincountOrRef" INTEGER NOT NULL DEFAULT '1',
    "maxcount" SMALLINT NOT NULL DEFAULT '1' CHECK ("maxcount" >= 0),
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0),
    PRIMARY KEY ("entry", "item")
);
ALTER TABLE IF EXISTS "fishing_loot_template" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "fishing_loot_template" ADD COLUMN IF NOT EXISTS "item" BIGINT NOT NULL DEFAULT '0' CHECK ("item" >= 0);
ALTER TABLE IF EXISTS "fishing_loot_template" ADD COLUMN IF NOT EXISTS "ChanceOrQuestChance" REAL NOT NULL DEFAULT '100';
ALTER TABLE IF EXISTS "fishing_loot_template" ADD COLUMN IF NOT EXISTS "groupid" SMALLINT NOT NULL DEFAULT '0' CHECK ("groupid" >= 0);
ALTER TABLE IF EXISTS "fishing_loot_template" ADD COLUMN IF NOT EXISTS "mincountOrRef" INTEGER NOT NULL DEFAULT '1';
ALTER TABLE IF EXISTS "fishing_loot_template" ADD COLUMN IF NOT EXISTS "maxcount" SMALLINT NOT NULL DEFAULT '1' CHECK ("maxcount" >= 0);
ALTER TABLE IF EXISTS "fishing_loot_template" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);
ALTER TABLE IF EXISTS "fishing_loot_template" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);
ALTER TABLE IF EXISTS "fishing_loot_template" ADD COLUMN IF NOT EXISTS "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0);

CREATE TABLE IF NOT EXISTS "forbidden_items" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "patch" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch" >= 0),
    "after_or_before" SMALLINT NOT NULL DEFAULT '0' CHECK ("after_or_before" >= 0),
    PRIMARY KEY ("entry", "after_or_before")
);
ALTER TABLE IF EXISTS "forbidden_items" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "forbidden_items" ADD COLUMN IF NOT EXISTS "patch" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch" >= 0);
ALTER TABLE IF EXISTS "forbidden_items" ADD COLUMN IF NOT EXISTS "after_or_before" SMALLINT NOT NULL DEFAULT '0' CHECK ("after_or_before" >= 0);

CREATE TABLE IF NOT EXISTS "game_event" (
    "entry" BIGINT NOT NULL CHECK ("entry" >= 0),
    "start_time" TIMESTAMP NOT NULL DEFAULT '0001-01-01 00:00:00',
    "end_time" TIMESTAMP NOT NULL DEFAULT '0001-01-01 00:00:00',
    "occurence" NUMERIC(20,0) NOT NULL DEFAULT '5184000' CHECK ("occurence" >= 0),
    "length" NUMERIC(20,0) NOT NULL DEFAULT '2592000' CHECK ("length" >= 0),
    "holiday" BIGINT NOT NULL DEFAULT '0' CHECK ("holiday" >= 0),
    "description" VARCHAR(255) DEFAULT NULL,
    "hardcoded" SMALLINT NOT NULL DEFAULT '0',
    "disabled" SMALLINT NOT NULL DEFAULT '0',
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0),
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "game_event" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "game_event" ADD COLUMN IF NOT EXISTS "start_time" TIMESTAMP NOT NULL DEFAULT '0001-01-01 00:00:00';
ALTER TABLE IF EXISTS "game_event" ADD COLUMN IF NOT EXISTS "end_time" TIMESTAMP NOT NULL DEFAULT '0001-01-01 00:00:00';
ALTER TABLE IF EXISTS "game_event" ADD COLUMN IF NOT EXISTS "occurence" NUMERIC(20,0) NOT NULL DEFAULT '5184000' CHECK ("occurence" >= 0);
ALTER TABLE IF EXISTS "game_event" ADD COLUMN IF NOT EXISTS "length" NUMERIC(20,0) NOT NULL DEFAULT '2592000' CHECK ("length" >= 0);
ALTER TABLE IF EXISTS "game_event" ADD COLUMN IF NOT EXISTS "holiday" BIGINT NOT NULL DEFAULT '0' CHECK ("holiday" >= 0);
ALTER TABLE IF EXISTS "game_event" ADD COLUMN IF NOT EXISTS "description" VARCHAR(255) DEFAULT NULL;
ALTER TABLE IF EXISTS "game_event" ADD COLUMN IF NOT EXISTS "hardcoded" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "game_event" ADD COLUMN IF NOT EXISTS "disabled" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "game_event" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);
ALTER TABLE IF EXISTS "game_event" ADD COLUMN IF NOT EXISTS "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0);

CREATE TABLE IF NOT EXISTS "game_event_creature" (
    "guid" BIGINT NOT NULL CHECK ("guid" >= 0),
    "event" SMALLINT NOT NULL DEFAULT '0',
    PRIMARY KEY ("guid", "event")
);
ALTER TABLE IF EXISTS "game_event_creature" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "game_event_creature" ADD COLUMN IF NOT EXISTS "event" SMALLINT NOT NULL DEFAULT '0';

CREATE TABLE IF NOT EXISTS "game_event_creature_data" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "patch" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch" >= 0),
    "entry_id" BIGINT NOT NULL DEFAULT '0' CHECK ("entry_id" >= 0),
    "display_id" BIGINT NOT NULL DEFAULT '0' CHECK ("display_id" >= 0),
    "equipment_id" BIGINT NOT NULL DEFAULT '0' CHECK ("equipment_id" >= 0),
    "spell_start" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_start" >= 0),
    "spell_end" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_end" >= 0),
    "event" INTEGER NOT NULL DEFAULT '0' CHECK ("event" >= 0),
    PRIMARY KEY ("guid", "event", "patch")
);
ALTER TABLE IF EXISTS "game_event_creature_data" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "game_event_creature_data" ADD COLUMN IF NOT EXISTS "patch" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch" >= 0);
ALTER TABLE IF EXISTS "game_event_creature_data" ADD COLUMN IF NOT EXISTS "entry_id" BIGINT NOT NULL DEFAULT '0' CHECK ("entry_id" >= 0);
ALTER TABLE IF EXISTS "game_event_creature_data" ADD COLUMN IF NOT EXISTS "display_id" BIGINT NOT NULL DEFAULT '0' CHECK ("display_id" >= 0);
ALTER TABLE IF EXISTS "game_event_creature_data" ADD COLUMN IF NOT EXISTS "equipment_id" BIGINT NOT NULL DEFAULT '0' CHECK ("equipment_id" >= 0);
ALTER TABLE IF EXISTS "game_event_creature_data" ADD COLUMN IF NOT EXISTS "spell_start" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_start" >= 0);
ALTER TABLE IF EXISTS "game_event_creature_data" ADD COLUMN IF NOT EXISTS "spell_end" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_end" >= 0);
ALTER TABLE IF EXISTS "game_event_creature_data" ADD COLUMN IF NOT EXISTS "event" INTEGER NOT NULL DEFAULT '0' CHECK ("event" >= 0);

CREATE TABLE IF NOT EXISTS "game_event_gameobject" (
    "guid" BIGINT NOT NULL CHECK ("guid" >= 0),
    "event" SMALLINT NOT NULL DEFAULT '0',
    PRIMARY KEY ("guid", "event")
);
ALTER TABLE IF EXISTS "game_event_gameobject" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "game_event_gameobject" ADD COLUMN IF NOT EXISTS "event" SMALLINT NOT NULL DEFAULT '0';

CREATE TABLE IF NOT EXISTS "game_event_mail" (
    "event" SMALLINT NOT NULL DEFAULT '0',
    "raceMask" BIGINT NOT NULL DEFAULT '0' CHECK ("raceMask" >= 0),
    "quest" BIGINT NOT NULL DEFAULT '0' CHECK ("quest" >= 0),
    "mailTemplateId" BIGINT NOT NULL DEFAULT '0' CHECK ("mailTemplateId" >= 0),
    "senderEntry" BIGINT NOT NULL DEFAULT '0' CHECK ("senderEntry" >= 0),
    PRIMARY KEY ("event", "raceMask", "quest")
);
ALTER TABLE IF EXISTS "game_event_mail" ADD COLUMN IF NOT EXISTS "event" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "game_event_mail" ADD COLUMN IF NOT EXISTS "raceMask" BIGINT NOT NULL DEFAULT '0' CHECK ("raceMask" >= 0);
ALTER TABLE IF EXISTS "game_event_mail" ADD COLUMN IF NOT EXISTS "quest" BIGINT NOT NULL DEFAULT '0' CHECK ("quest" >= 0);
ALTER TABLE IF EXISTS "game_event_mail" ADD COLUMN IF NOT EXISTS "mailTemplateId" BIGINT NOT NULL DEFAULT '0' CHECK ("mailTemplateId" >= 0);
ALTER TABLE IF EXISTS "game_event_mail" ADD COLUMN IF NOT EXISTS "senderEntry" BIGINT NOT NULL DEFAULT '0' CHECK ("senderEntry" >= 0);

CREATE TABLE IF NOT EXISTS "game_event_quest" (
    "quest" BIGINT NOT NULL DEFAULT '0' CHECK ("quest" >= 0),
    "event" INTEGER NOT NULL DEFAULT '0' CHECK ("event" >= 0),
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    PRIMARY KEY ("quest", "event")
);
ALTER TABLE IF EXISTS "game_event_quest" ADD COLUMN IF NOT EXISTS "quest" BIGINT NOT NULL DEFAULT '0' CHECK ("quest" >= 0);
ALTER TABLE IF EXISTS "game_event_quest" ADD COLUMN IF NOT EXISTS "event" INTEGER NOT NULL DEFAULT '0' CHECK ("event" >= 0);
ALTER TABLE IF EXISTS "game_event_quest" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);

CREATE TABLE IF NOT EXISTS "game_graveyard_zone" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "ghost_zone" BIGINT NOT NULL DEFAULT '0' CHECK ("ghost_zone" >= 0),
    "faction" INTEGER NOT NULL DEFAULT '0' CHECK ("faction" >= 0),
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0),
    PRIMARY KEY ("id", "ghost_zone", "patch_max")
);
ALTER TABLE IF EXISTS "game_graveyard_zone" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "game_graveyard_zone" ADD COLUMN IF NOT EXISTS "ghost_zone" BIGINT NOT NULL DEFAULT '0' CHECK ("ghost_zone" >= 0);
ALTER TABLE IF EXISTS "game_graveyard_zone" ADD COLUMN IF NOT EXISTS "faction" INTEGER NOT NULL DEFAULT '0' CHECK ("faction" >= 0);
ALTER TABLE IF EXISTS "game_graveyard_zone" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);
ALTER TABLE IF EXISTS "game_graveyard_zone" ADD COLUMN IF NOT EXISTS "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0);

CREATE TABLE IF NOT EXISTS "game_tele" (
    "id" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("id" >= 0),
    "position_x" REAL NOT NULL DEFAULT '0',
    "position_y" REAL NOT NULL DEFAULT '0',
    "position_z" REAL NOT NULL DEFAULT '0',
    "orientation" REAL NOT NULL DEFAULT '0',
    "map" INTEGER NOT NULL DEFAULT '0' CHECK ("map" >= 0),
    "name" VARCHAR(100) NOT NULL DEFAULT '',
    PRIMARY KEY ("id")
);
ALTER TABLE IF EXISTS "game_tele" ADD COLUMN IF NOT EXISTS "id" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "game_tele" ADD COLUMN IF NOT EXISTS "position_x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "game_tele" ADD COLUMN IF NOT EXISTS "position_y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "game_tele" ADD COLUMN IF NOT EXISTS "position_z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "game_tele" ADD COLUMN IF NOT EXISTS "orientation" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "game_tele" ADD COLUMN IF NOT EXISTS "map" INTEGER NOT NULL DEFAULT '0' CHECK ("map" >= 0);
ALTER TABLE IF EXISTS "game_tele" ADD COLUMN IF NOT EXISTS "name" VARCHAR(100) NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS "game_weather" (
    "zone" BIGINT NOT NULL DEFAULT '0' CHECK ("zone" >= 0),
    "spring_rain_chance" SMALLINT NOT NULL DEFAULT '25' CHECK ("spring_rain_chance" >= 0),
    "spring_snow_chance" SMALLINT NOT NULL DEFAULT '25' CHECK ("spring_snow_chance" >= 0),
    "spring_storm_chance" SMALLINT NOT NULL DEFAULT '25' CHECK ("spring_storm_chance" >= 0),
    "summer_rain_chance" SMALLINT NOT NULL DEFAULT '25' CHECK ("summer_rain_chance" >= 0),
    "summer_snow_chance" SMALLINT NOT NULL DEFAULT '25' CHECK ("summer_snow_chance" >= 0),
    "summer_storm_chance" SMALLINT NOT NULL DEFAULT '25' CHECK ("summer_storm_chance" >= 0),
    "fall_rain_chance" SMALLINT NOT NULL DEFAULT '25' CHECK ("fall_rain_chance" >= 0),
    "fall_snow_chance" SMALLINT NOT NULL DEFAULT '25' CHECK ("fall_snow_chance" >= 0),
    "fall_storm_chance" SMALLINT NOT NULL DEFAULT '25' CHECK ("fall_storm_chance" >= 0),
    "winter_rain_chance" SMALLINT NOT NULL DEFAULT '25' CHECK ("winter_rain_chance" >= 0),
    "winter_snow_chance" SMALLINT NOT NULL DEFAULT '25' CHECK ("winter_snow_chance" >= 0),
    "winter_storm_chance" SMALLINT NOT NULL DEFAULT '25' CHECK ("winter_storm_chance" >= 0),
    PRIMARY KEY ("zone")
);
ALTER TABLE IF EXISTS "game_weather" ADD COLUMN IF NOT EXISTS "zone" BIGINT NOT NULL DEFAULT '0' CHECK ("zone" >= 0);
ALTER TABLE IF EXISTS "game_weather" ADD COLUMN IF NOT EXISTS "spring_rain_chance" SMALLINT NOT NULL DEFAULT '25' CHECK ("spring_rain_chance" >= 0);
ALTER TABLE IF EXISTS "game_weather" ADD COLUMN IF NOT EXISTS "spring_snow_chance" SMALLINT NOT NULL DEFAULT '25' CHECK ("spring_snow_chance" >= 0);
ALTER TABLE IF EXISTS "game_weather" ADD COLUMN IF NOT EXISTS "spring_storm_chance" SMALLINT NOT NULL DEFAULT '25' CHECK ("spring_storm_chance" >= 0);
ALTER TABLE IF EXISTS "game_weather" ADD COLUMN IF NOT EXISTS "summer_rain_chance" SMALLINT NOT NULL DEFAULT '25' CHECK ("summer_rain_chance" >= 0);
ALTER TABLE IF EXISTS "game_weather" ADD COLUMN IF NOT EXISTS "summer_snow_chance" SMALLINT NOT NULL DEFAULT '25' CHECK ("summer_snow_chance" >= 0);
ALTER TABLE IF EXISTS "game_weather" ADD COLUMN IF NOT EXISTS "summer_storm_chance" SMALLINT NOT NULL DEFAULT '25' CHECK ("summer_storm_chance" >= 0);
ALTER TABLE IF EXISTS "game_weather" ADD COLUMN IF NOT EXISTS "fall_rain_chance" SMALLINT NOT NULL DEFAULT '25' CHECK ("fall_rain_chance" >= 0);
ALTER TABLE IF EXISTS "game_weather" ADD COLUMN IF NOT EXISTS "fall_snow_chance" SMALLINT NOT NULL DEFAULT '25' CHECK ("fall_snow_chance" >= 0);
ALTER TABLE IF EXISTS "game_weather" ADD COLUMN IF NOT EXISTS "fall_storm_chance" SMALLINT NOT NULL DEFAULT '25' CHECK ("fall_storm_chance" >= 0);
ALTER TABLE IF EXISTS "game_weather" ADD COLUMN IF NOT EXISTS "winter_rain_chance" SMALLINT NOT NULL DEFAULT '25' CHECK ("winter_rain_chance" >= 0);
ALTER TABLE IF EXISTS "game_weather" ADD COLUMN IF NOT EXISTS "winter_snow_chance" SMALLINT NOT NULL DEFAULT '25' CHECK ("winter_snow_chance" >= 0);
ALTER TABLE IF EXISTS "game_weather" ADD COLUMN IF NOT EXISTS "winter_storm_chance" SMALLINT NOT NULL DEFAULT '25' CHECK ("winter_storm_chance" >= 0);

CREATE TABLE IF NOT EXISTS "gameobject" (
    "guid" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("guid" >= 0),
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "map" INTEGER NOT NULL DEFAULT '0' CHECK ("map" >= 0),
    "position_x" REAL NOT NULL DEFAULT '0',
    "position_y" REAL NOT NULL DEFAULT '0',
    "position_z" REAL NOT NULL DEFAULT '0',
    "orientation" REAL NOT NULL DEFAULT '0',
    "rotation0" REAL NOT NULL DEFAULT '0',
    "rotation1" REAL NOT NULL DEFAULT '0',
    "rotation2" REAL NOT NULL DEFAULT '0',
    "rotation3" REAL NOT NULL DEFAULT '0',
    "spawntimesecsmin" INTEGER NOT NULL DEFAULT '0',
    "spawntimesecsmax" INTEGER NOT NULL DEFAULT '0',
    "animprogress" SMALLINT NOT NULL DEFAULT '0' CHECK ("animprogress" >= 0),
    "state" SMALLINT NOT NULL DEFAULT '0' CHECK ("state" >= 0),
    "spawn_flags" BIGINT NOT NULL DEFAULT '0' CHECK ("spawn_flags" >= 0),
    "visibility_mod" REAL DEFAULT '0',
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0),
    PRIMARY KEY ("guid")
);
ALTER TABLE IF EXISTS "gameobject" ADD COLUMN IF NOT EXISTS "guid" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "gameobject" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "gameobject" ADD COLUMN IF NOT EXISTS "map" INTEGER NOT NULL DEFAULT '0' CHECK ("map" >= 0);
ALTER TABLE IF EXISTS "gameobject" ADD COLUMN IF NOT EXISTS "position_x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject" ADD COLUMN IF NOT EXISTS "position_y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject" ADD COLUMN IF NOT EXISTS "position_z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject" ADD COLUMN IF NOT EXISTS "orientation" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject" ADD COLUMN IF NOT EXISTS "rotation0" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject" ADD COLUMN IF NOT EXISTS "rotation1" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject" ADD COLUMN IF NOT EXISTS "rotation2" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject" ADD COLUMN IF NOT EXISTS "rotation3" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject" ADD COLUMN IF NOT EXISTS "spawntimesecsmin" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject" ADD COLUMN IF NOT EXISTS "spawntimesecsmax" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject" ADD COLUMN IF NOT EXISTS "animprogress" SMALLINT NOT NULL DEFAULT '0' CHECK ("animprogress" >= 0);
ALTER TABLE IF EXISTS "gameobject" ADD COLUMN IF NOT EXISTS "state" SMALLINT NOT NULL DEFAULT '0' CHECK ("state" >= 0);
ALTER TABLE IF EXISTS "gameobject" ADD COLUMN IF NOT EXISTS "spawn_flags" BIGINT NOT NULL DEFAULT '0' CHECK ("spawn_flags" >= 0);
ALTER TABLE IF EXISTS "gameobject" ADD COLUMN IF NOT EXISTS "visibility_mod" REAL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);
ALTER TABLE IF EXISTS "gameobject" ADD COLUMN IF NOT EXISTS "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0);
CREATE INDEX IF NOT EXISTS idx_gameobject_idx_map ON "gameobject" ("map");
CREATE INDEX IF NOT EXISTS idx_gameobject_idx_id ON "gameobject" ("id");

CREATE TABLE IF NOT EXISTS "gameobject_battleground" (
    "guid" BIGINT NOT NULL CHECK ("guid" >= 0),
    "event1" SMALLINT NOT NULL CHECK ("event1" >= 0),
    "event2" SMALLINT NOT NULL CHECK ("event2" >= 0),
    PRIMARY KEY ("guid", "event1", "event2")
);
ALTER TABLE IF EXISTS "gameobject_battleground" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "gameobject_battleground" ADD COLUMN IF NOT EXISTS "event1" SMALLINT NOT NULL CHECK ("event1" >= 0);
ALTER TABLE IF EXISTS "gameobject_battleground" ADD COLUMN IF NOT EXISTS "event2" SMALLINT NOT NULL CHECK ("event2" >= 0);

CREATE TABLE IF NOT EXISTS "gameobject_display_info_addon" (
    "display_id" INTEGER NOT NULL DEFAULT '0',
    "min_x" REAL NOT NULL DEFAULT '0',
    "min_y" REAL NOT NULL DEFAULT '0',
    "min_z" REAL NOT NULL DEFAULT '0',
    "max_x" REAL NOT NULL DEFAULT '0',
    "max_y" REAL NOT NULL DEFAULT '0',
    "max_z" REAL NOT NULL DEFAULT '0',
    PRIMARY KEY ("display_id")
);
ALTER TABLE IF EXISTS "gameobject_display_info_addon" ADD COLUMN IF NOT EXISTS "display_id" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_display_info_addon" ADD COLUMN IF NOT EXISTS "min_x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_display_info_addon" ADD COLUMN IF NOT EXISTS "min_y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_display_info_addon" ADD COLUMN IF NOT EXISTS "min_z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_display_info_addon" ADD COLUMN IF NOT EXISTS "max_x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_display_info_addon" ADD COLUMN IF NOT EXISTS "max_y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_display_info_addon" ADD COLUMN IF NOT EXISTS "max_z" REAL NOT NULL DEFAULT '0';

CREATE TABLE IF NOT EXISTS "gameobject_involvedrelation" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "quest" BIGINT NOT NULL DEFAULT '0' CHECK ("quest" >= 0),
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0),
    PRIMARY KEY ("id", "quest")
);
ALTER TABLE IF EXISTS "gameobject_involvedrelation" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "gameobject_involvedrelation" ADD COLUMN IF NOT EXISTS "quest" BIGINT NOT NULL DEFAULT '0' CHECK ("quest" >= 0);
ALTER TABLE IF EXISTS "gameobject_involvedrelation" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);
ALTER TABLE IF EXISTS "gameobject_involvedrelation" ADD COLUMN IF NOT EXISTS "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0);

CREATE TABLE IF NOT EXISTS "gameobject_loot_template" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "item" BIGINT NOT NULL DEFAULT '0' CHECK ("item" >= 0),
    "ChanceOrQuestChance" REAL NOT NULL DEFAULT '100',
    "groupid" SMALLINT NOT NULL DEFAULT '0' CHECK ("groupid" >= 0),
    "mincountOrRef" INTEGER NOT NULL DEFAULT '1',
    "maxcount" SMALLINT NOT NULL DEFAULT '1' CHECK ("maxcount" >= 0),
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0),
    PRIMARY KEY ("entry", "item")
);
ALTER TABLE IF EXISTS "gameobject_loot_template" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "gameobject_loot_template" ADD COLUMN IF NOT EXISTS "item" BIGINT NOT NULL DEFAULT '0' CHECK ("item" >= 0);
ALTER TABLE IF EXISTS "gameobject_loot_template" ADD COLUMN IF NOT EXISTS "ChanceOrQuestChance" REAL NOT NULL DEFAULT '100';
ALTER TABLE IF EXISTS "gameobject_loot_template" ADD COLUMN IF NOT EXISTS "groupid" SMALLINT NOT NULL DEFAULT '0' CHECK ("groupid" >= 0);
ALTER TABLE IF EXISTS "gameobject_loot_template" ADD COLUMN IF NOT EXISTS "mincountOrRef" INTEGER NOT NULL DEFAULT '1';
ALTER TABLE IF EXISTS "gameobject_loot_template" ADD COLUMN IF NOT EXISTS "maxcount" SMALLINT NOT NULL DEFAULT '1' CHECK ("maxcount" >= 0);
ALTER TABLE IF EXISTS "gameobject_loot_template" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);
ALTER TABLE IF EXISTS "gameobject_loot_template" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);
ALTER TABLE IF EXISTS "gameobject_loot_template" ADD COLUMN IF NOT EXISTS "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0);

CREATE TABLE IF NOT EXISTS "gameobject_questrelation" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "quest" BIGINT NOT NULL DEFAULT '0' CHECK ("quest" >= 0),
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0),
    PRIMARY KEY ("id", "quest")
);
ALTER TABLE IF EXISTS "gameobject_questrelation" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "gameobject_questrelation" ADD COLUMN IF NOT EXISTS "quest" BIGINT NOT NULL DEFAULT '0' CHECK ("quest" >= 0);
ALTER TABLE IF EXISTS "gameobject_questrelation" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);
ALTER TABLE IF EXISTS "gameobject_questrelation" ADD COLUMN IF NOT EXISTS "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0);

CREATE TABLE IF NOT EXISTS "gameobject_requirement" (
    "guid" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("guid" >= 0),
    "reqType" BIGINT NOT NULL DEFAULT '0' CHECK ("reqType" >= 0),
    "reqGuid" BIGINT NOT NULL DEFAULT '0' CHECK ("reqGuid" >= 0),
    PRIMARY KEY ("guid")
);
ALTER TABLE IF EXISTS "gameobject_requirement" ADD COLUMN IF NOT EXISTS "guid" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "gameobject_requirement" ADD COLUMN IF NOT EXISTS "reqType" BIGINT NOT NULL DEFAULT '0' CHECK ("reqType" >= 0);
ALTER TABLE IF EXISTS "gameobject_requirement" ADD COLUMN IF NOT EXISTS "reqGuid" BIGINT NOT NULL DEFAULT '0' CHECK ("reqGuid" >= 0);

CREATE TABLE IF NOT EXISTS "gameobject_scripts" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "delay" BIGINT NOT NULL DEFAULT '0' CHECK ("delay" >= 0),
    "priority" SMALLINT NOT NULL DEFAULT '0' CHECK ("priority" >= 0),
    "command" SMALLINT NOT NULL DEFAULT '0' CHECK ("command" >= 0),
    "datalong" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong" >= 0),
    "datalong2" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong2" >= 0),
    "datalong3" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong3" >= 0),
    "datalong4" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong4" >= 0),
    "target_param1" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param1" >= 0),
    "target_param2" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param2" >= 0),
    "target_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("target_type" >= 0),
    "data_flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("data_flags" >= 0),
    "dataint" INTEGER NOT NULL DEFAULT '0',
    "dataint2" INTEGER NOT NULL DEFAULT '0',
    "dataint3" INTEGER NOT NULL DEFAULT '0',
    "dataint4" INTEGER NOT NULL DEFAULT '0',
    "x" REAL NOT NULL DEFAULT '0',
    "y" REAL NOT NULL DEFAULT '0',
    "z" REAL NOT NULL DEFAULT '0',
    "o" REAL NOT NULL DEFAULT '0',
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    "comments" VARCHAR(255) NOT NULL
);
ALTER TABLE IF EXISTS "gameobject_scripts" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "gameobject_scripts" ADD COLUMN IF NOT EXISTS "delay" BIGINT NOT NULL DEFAULT '0' CHECK ("delay" >= 0);
ALTER TABLE IF EXISTS "gameobject_scripts" ADD COLUMN IF NOT EXISTS "priority" SMALLINT NOT NULL DEFAULT '0' CHECK ("priority" >= 0);
ALTER TABLE IF EXISTS "gameobject_scripts" ADD COLUMN IF NOT EXISTS "command" SMALLINT NOT NULL DEFAULT '0' CHECK ("command" >= 0);
ALTER TABLE IF EXISTS "gameobject_scripts" ADD COLUMN IF NOT EXISTS "datalong" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong" >= 0);
ALTER TABLE IF EXISTS "gameobject_scripts" ADD COLUMN IF NOT EXISTS "datalong2" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong2" >= 0);
ALTER TABLE IF EXISTS "gameobject_scripts" ADD COLUMN IF NOT EXISTS "datalong3" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong3" >= 0);
ALTER TABLE IF EXISTS "gameobject_scripts" ADD COLUMN IF NOT EXISTS "datalong4" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong4" >= 0);
ALTER TABLE IF EXISTS "gameobject_scripts" ADD COLUMN IF NOT EXISTS "target_param1" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param1" >= 0);
ALTER TABLE IF EXISTS "gameobject_scripts" ADD COLUMN IF NOT EXISTS "target_param2" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param2" >= 0);
ALTER TABLE IF EXISTS "gameobject_scripts" ADD COLUMN IF NOT EXISTS "target_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("target_type" >= 0);
ALTER TABLE IF EXISTS "gameobject_scripts" ADD COLUMN IF NOT EXISTS "data_flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("data_flags" >= 0);
ALTER TABLE IF EXISTS "gameobject_scripts" ADD COLUMN IF NOT EXISTS "dataint" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_scripts" ADD COLUMN IF NOT EXISTS "dataint2" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_scripts" ADD COLUMN IF NOT EXISTS "dataint3" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_scripts" ADD COLUMN IF NOT EXISTS "dataint4" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_scripts" ADD COLUMN IF NOT EXISTS "x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_scripts" ADD COLUMN IF NOT EXISTS "y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_scripts" ADD COLUMN IF NOT EXISTS "z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_scripts" ADD COLUMN IF NOT EXISTS "o" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_scripts" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);
ALTER TABLE IF EXISTS "gameobject_scripts" ADD COLUMN IF NOT EXISTS "comments" VARCHAR(255) NOT NULL;

CREATE TABLE IF NOT EXISTS "gameobject_template" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "patch" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch" >= 0),
    "type" SMALLINT NOT NULL DEFAULT '0' CHECK ("type" >= 0),
    "displayId" BIGINT NOT NULL DEFAULT '0' CHECK ("displayId" >= 0),
    "name" VARCHAR(100) NOT NULL DEFAULT '',
    "icon" VARCHAR(100) NOT NULL DEFAULT '',
    "faction" INTEGER NOT NULL DEFAULT '0' CHECK ("faction" >= 0),
    "flags" BIGINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0),
    "size" REAL NOT NULL DEFAULT '1',
    "data0" INTEGER NOT NULL DEFAULT '0',
    "data1" INTEGER NOT NULL DEFAULT '0',
    "data2" INTEGER NOT NULL DEFAULT '0',
    "data3" INTEGER NOT NULL DEFAULT '0',
    "data4" INTEGER NOT NULL DEFAULT '0',
    "data5" INTEGER NOT NULL DEFAULT '0',
    "data6" INTEGER NOT NULL DEFAULT '0',
    "data7" INTEGER NOT NULL DEFAULT '0',
    "data8" INTEGER NOT NULL DEFAULT '0',
    "data9" INTEGER NOT NULL DEFAULT '0',
    "data10" INTEGER NOT NULL DEFAULT '0',
    "data11" INTEGER NOT NULL DEFAULT '0',
    "data12" INTEGER NOT NULL DEFAULT '0',
    "data13" INTEGER NOT NULL DEFAULT '0',
    "data14" INTEGER NOT NULL DEFAULT '0',
    "data15" INTEGER NOT NULL DEFAULT '0',
    "data16" INTEGER NOT NULL DEFAULT '0',
    "data17" INTEGER NOT NULL DEFAULT '0',
    "data18" INTEGER NOT NULL DEFAULT '0',
    "data19" INTEGER NOT NULL DEFAULT '0',
    "data20" INTEGER NOT NULL DEFAULT '0',
    "data21" INTEGER NOT NULL DEFAULT '0',
    "data22" INTEGER NOT NULL DEFAULT '0',
    "data23" INTEGER NOT NULL DEFAULT '0',
    "mingold" BIGINT NOT NULL DEFAULT '0' CHECK ("mingold" >= 0),
    "maxgold" BIGINT NOT NULL DEFAULT '0' CHECK ("maxgold" >= 0),
    "script_name" VARCHAR(64) NOT NULL DEFAULT '',
    PRIMARY KEY ("entry", "patch")
);
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "patch" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch" >= 0);
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "type" SMALLINT NOT NULL DEFAULT '0' CHECK ("type" >= 0);
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "displayId" BIGINT NOT NULL DEFAULT '0' CHECK ("displayId" >= 0);
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "name" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "icon" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "faction" INTEGER NOT NULL DEFAULT '0' CHECK ("faction" >= 0);
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "flags" BIGINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0);
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "size" REAL NOT NULL DEFAULT '1';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "data0" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "data1" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "data2" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "data3" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "data4" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "data5" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "data6" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "data7" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "data8" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "data9" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "data10" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "data11" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "data12" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "data13" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "data14" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "data15" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "data16" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "data17" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "data18" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "data19" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "data20" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "data21" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "data22" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "data23" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "mingold" BIGINT NOT NULL DEFAULT '0' CHECK ("mingold" >= 0);
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "maxgold" BIGINT NOT NULL DEFAULT '0' CHECK ("maxgold" >= 0);
ALTER TABLE IF EXISTS "gameobject_template" ADD COLUMN IF NOT EXISTS "script_name" VARCHAR(64) NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS "generic_scripts" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "delay" BIGINT NOT NULL DEFAULT '0' CHECK ("delay" >= 0),
    "priority" SMALLINT NOT NULL DEFAULT '0' CHECK ("priority" >= 0),
    "command" SMALLINT NOT NULL DEFAULT '0' CHECK ("command" >= 0),
    "datalong" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong" >= 0),
    "datalong2" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong2" >= 0),
    "datalong3" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong3" >= 0),
    "datalong4" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong4" >= 0),
    "target_param1" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param1" >= 0),
    "target_param2" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param2" >= 0),
    "target_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("target_type" >= 0),
    "data_flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("data_flags" >= 0),
    "dataint" INTEGER NOT NULL DEFAULT '0',
    "dataint2" INTEGER NOT NULL DEFAULT '0',
    "dataint3" INTEGER NOT NULL DEFAULT '0',
    "dataint4" INTEGER NOT NULL DEFAULT '0',
    "x" REAL NOT NULL DEFAULT '0',
    "y" REAL NOT NULL DEFAULT '0',
    "z" REAL NOT NULL DEFAULT '0',
    "o" REAL NOT NULL DEFAULT '0',
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    "comments" VARCHAR(255) NOT NULL
);
ALTER TABLE IF EXISTS "generic_scripts" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "generic_scripts" ADD COLUMN IF NOT EXISTS "delay" BIGINT NOT NULL DEFAULT '0' CHECK ("delay" >= 0);
ALTER TABLE IF EXISTS "generic_scripts" ADD COLUMN IF NOT EXISTS "priority" SMALLINT NOT NULL DEFAULT '0' CHECK ("priority" >= 0);
ALTER TABLE IF EXISTS "generic_scripts" ADD COLUMN IF NOT EXISTS "command" SMALLINT NOT NULL DEFAULT '0' CHECK ("command" >= 0);
ALTER TABLE IF EXISTS "generic_scripts" ADD COLUMN IF NOT EXISTS "datalong" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong" >= 0);
ALTER TABLE IF EXISTS "generic_scripts" ADD COLUMN IF NOT EXISTS "datalong2" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong2" >= 0);
ALTER TABLE IF EXISTS "generic_scripts" ADD COLUMN IF NOT EXISTS "datalong3" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong3" >= 0);
ALTER TABLE IF EXISTS "generic_scripts" ADD COLUMN IF NOT EXISTS "datalong4" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong4" >= 0);
ALTER TABLE IF EXISTS "generic_scripts" ADD COLUMN IF NOT EXISTS "target_param1" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param1" >= 0);
ALTER TABLE IF EXISTS "generic_scripts" ADD COLUMN IF NOT EXISTS "target_param2" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param2" >= 0);
ALTER TABLE IF EXISTS "generic_scripts" ADD COLUMN IF NOT EXISTS "target_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("target_type" >= 0);
ALTER TABLE IF EXISTS "generic_scripts" ADD COLUMN IF NOT EXISTS "data_flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("data_flags" >= 0);
ALTER TABLE IF EXISTS "generic_scripts" ADD COLUMN IF NOT EXISTS "dataint" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "generic_scripts" ADD COLUMN IF NOT EXISTS "dataint2" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "generic_scripts" ADD COLUMN IF NOT EXISTS "dataint3" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "generic_scripts" ADD COLUMN IF NOT EXISTS "dataint4" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "generic_scripts" ADD COLUMN IF NOT EXISTS "x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "generic_scripts" ADD COLUMN IF NOT EXISTS "y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "generic_scripts" ADD COLUMN IF NOT EXISTS "z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "generic_scripts" ADD COLUMN IF NOT EXISTS "o" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "generic_scripts" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);
ALTER TABLE IF EXISTS "generic_scripts" ADD COLUMN IF NOT EXISTS "comments" VARCHAR(255) NOT NULL;

CREATE TABLE IF NOT EXISTS "gossip_menu" (
    "entry" INTEGER NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "text_id" BIGINT NOT NULL DEFAULT '0' CHECK ("text_id" >= 0),
    "script_id" BIGINT NOT NULL DEFAULT '0' CHECK ("script_id" >= 0),
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    PRIMARY KEY ("entry", "text_id")
);
ALTER TABLE IF EXISTS "gossip_menu" ADD COLUMN IF NOT EXISTS "entry" INTEGER NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "gossip_menu" ADD COLUMN IF NOT EXISTS "text_id" BIGINT NOT NULL DEFAULT '0' CHECK ("text_id" >= 0);
ALTER TABLE IF EXISTS "gossip_menu" ADD COLUMN IF NOT EXISTS "script_id" BIGINT NOT NULL DEFAULT '0' CHECK ("script_id" >= 0);
ALTER TABLE IF EXISTS "gossip_menu" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);

CREATE TABLE IF NOT EXISTS "gossip_menu_option" (
    "menu_id" INTEGER NOT NULL DEFAULT '0' CHECK ("menu_id" >= 0),
    "id" INTEGER NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "option_icon" BIGINT NOT NULL DEFAULT '0' CHECK ("option_icon" >= 0),
    "option_text" TEXT,
    "option_broadcast_text" BIGINT NOT NULL DEFAULT '0' CHECK ("option_broadcast_text" >= 0),
    "option_id" SMALLINT NOT NULL DEFAULT '0' CHECK ("option_id" >= 0),
    "npc_option_npcflag" BIGINT NOT NULL DEFAULT '0' CHECK ("npc_option_npcflag" >= 0),
    "action_menu_id" INTEGER NOT NULL DEFAULT '0',
    "action_poi_id" BIGINT NOT NULL DEFAULT '0' CHECK ("action_poi_id" >= 0),
    "action_script_id" BIGINT NOT NULL DEFAULT '0' CHECK ("action_script_id" >= 0),
    "box_coded" SMALLINT NOT NULL DEFAULT '0' CHECK ("box_coded" >= 0),
    "box_money" BIGINT NOT NULL DEFAULT '0' CHECK ("box_money" >= 0),
    "box_text" TEXT,
    "box_broadcast_text" BIGINT NOT NULL DEFAULT '0' CHECK ("box_broadcast_text" >= 0),
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    PRIMARY KEY ("menu_id", "id")
);
ALTER TABLE IF EXISTS "gossip_menu_option" ADD COLUMN IF NOT EXISTS "menu_id" INTEGER NOT NULL DEFAULT '0' CHECK ("menu_id" >= 0);
ALTER TABLE IF EXISTS "gossip_menu_option" ADD COLUMN IF NOT EXISTS "id" INTEGER NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "gossip_menu_option" ADD COLUMN IF NOT EXISTS "option_icon" BIGINT NOT NULL DEFAULT '0' CHECK ("option_icon" >= 0);
ALTER TABLE IF EXISTS "gossip_menu_option" ADD COLUMN IF NOT EXISTS "option_text" TEXT;
ALTER TABLE IF EXISTS "gossip_menu_option" ADD COLUMN IF NOT EXISTS "option_broadcast_text" BIGINT NOT NULL DEFAULT '0' CHECK ("option_broadcast_text" >= 0);
ALTER TABLE IF EXISTS "gossip_menu_option" ADD COLUMN IF NOT EXISTS "option_id" SMALLINT NOT NULL DEFAULT '0' CHECK ("option_id" >= 0);
ALTER TABLE IF EXISTS "gossip_menu_option" ADD COLUMN IF NOT EXISTS "npc_option_npcflag" BIGINT NOT NULL DEFAULT '0' CHECK ("npc_option_npcflag" >= 0);
ALTER TABLE IF EXISTS "gossip_menu_option" ADD COLUMN IF NOT EXISTS "action_menu_id" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gossip_menu_option" ADD COLUMN IF NOT EXISTS "action_poi_id" BIGINT NOT NULL DEFAULT '0' CHECK ("action_poi_id" >= 0);
ALTER TABLE IF EXISTS "gossip_menu_option" ADD COLUMN IF NOT EXISTS "action_script_id" BIGINT NOT NULL DEFAULT '0' CHECK ("action_script_id" >= 0);
ALTER TABLE IF EXISTS "gossip_menu_option" ADD COLUMN IF NOT EXISTS "box_coded" SMALLINT NOT NULL DEFAULT '0' CHECK ("box_coded" >= 0);
ALTER TABLE IF EXISTS "gossip_menu_option" ADD COLUMN IF NOT EXISTS "box_money" BIGINT NOT NULL DEFAULT '0' CHECK ("box_money" >= 0);
ALTER TABLE IF EXISTS "gossip_menu_option" ADD COLUMN IF NOT EXISTS "box_text" TEXT;
ALTER TABLE IF EXISTS "gossip_menu_option" ADD COLUMN IF NOT EXISTS "box_broadcast_text" BIGINT NOT NULL DEFAULT '0' CHECK ("box_broadcast_text" >= 0);
ALTER TABLE IF EXISTS "gossip_menu_option" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);

CREATE TABLE IF NOT EXISTS "gossip_scripts" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "delay" BIGINT NOT NULL DEFAULT '0' CHECK ("delay" >= 0),
    "priority" SMALLINT NOT NULL DEFAULT '0' CHECK ("priority" >= 0),
    "command" SMALLINT NOT NULL DEFAULT '0' CHECK ("command" >= 0),
    "datalong" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong" >= 0),
    "datalong2" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong2" >= 0),
    "datalong3" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong3" >= 0),
    "datalong4" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong4" >= 0),
    "target_param1" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param1" >= 0),
    "target_param2" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param2" >= 0),
    "target_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("target_type" >= 0),
    "data_flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("data_flags" >= 0),
    "dataint" INTEGER NOT NULL DEFAULT '0',
    "dataint2" INTEGER NOT NULL DEFAULT '0',
    "dataint3" INTEGER NOT NULL DEFAULT '0',
    "dataint4" INTEGER NOT NULL DEFAULT '0',
    "x" REAL NOT NULL DEFAULT '0',
    "y" REAL NOT NULL DEFAULT '0',
    "z" REAL NOT NULL DEFAULT '0',
    "o" REAL NOT NULL DEFAULT '0',
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    "comments" VARCHAR(255) NOT NULL
);
ALTER TABLE IF EXISTS "gossip_scripts" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "gossip_scripts" ADD COLUMN IF NOT EXISTS "delay" BIGINT NOT NULL DEFAULT '0' CHECK ("delay" >= 0);
ALTER TABLE IF EXISTS "gossip_scripts" ADD COLUMN IF NOT EXISTS "priority" SMALLINT NOT NULL DEFAULT '0' CHECK ("priority" >= 0);
ALTER TABLE IF EXISTS "gossip_scripts" ADD COLUMN IF NOT EXISTS "command" SMALLINT NOT NULL DEFAULT '0' CHECK ("command" >= 0);
ALTER TABLE IF EXISTS "gossip_scripts" ADD COLUMN IF NOT EXISTS "datalong" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong" >= 0);
ALTER TABLE IF EXISTS "gossip_scripts" ADD COLUMN IF NOT EXISTS "datalong2" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong2" >= 0);
ALTER TABLE IF EXISTS "gossip_scripts" ADD COLUMN IF NOT EXISTS "datalong3" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong3" >= 0);
ALTER TABLE IF EXISTS "gossip_scripts" ADD COLUMN IF NOT EXISTS "datalong4" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong4" >= 0);
ALTER TABLE IF EXISTS "gossip_scripts" ADD COLUMN IF NOT EXISTS "target_param1" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param1" >= 0);
ALTER TABLE IF EXISTS "gossip_scripts" ADD COLUMN IF NOT EXISTS "target_param2" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param2" >= 0);
ALTER TABLE IF EXISTS "gossip_scripts" ADD COLUMN IF NOT EXISTS "target_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("target_type" >= 0);
ALTER TABLE IF EXISTS "gossip_scripts" ADD COLUMN IF NOT EXISTS "data_flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("data_flags" >= 0);
ALTER TABLE IF EXISTS "gossip_scripts" ADD COLUMN IF NOT EXISTS "dataint" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gossip_scripts" ADD COLUMN IF NOT EXISTS "dataint2" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gossip_scripts" ADD COLUMN IF NOT EXISTS "dataint3" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gossip_scripts" ADD COLUMN IF NOT EXISTS "dataint4" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gossip_scripts" ADD COLUMN IF NOT EXISTS "x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gossip_scripts" ADD COLUMN IF NOT EXISTS "y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gossip_scripts" ADD COLUMN IF NOT EXISTS "z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gossip_scripts" ADD COLUMN IF NOT EXISTS "o" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gossip_scripts" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);
ALTER TABLE IF EXISTS "gossip_scripts" ADD COLUMN IF NOT EXISTS "comments" VARCHAR(255) NOT NULL;

CREATE TABLE IF NOT EXISTS "instance_buff_removal" (
    "map_id" BIGINT NOT NULL CHECK ("map_id" >= 0),
    "spell_id" INTEGER NOT NULL CHECK ("spell_id" >= 0),
    "enabled" SMALLINT NOT NULL,
    "flags" INTEGER NOT NULL,
    "comment" VARCHAR(256) NOT NULL,
    PRIMARY KEY ("map_id", "spell_id")
);
ALTER TABLE IF EXISTS "instance_buff_removal" ADD COLUMN IF NOT EXISTS "map_id" BIGINT NOT NULL CHECK ("map_id" >= 0);
ALTER TABLE IF EXISTS "instance_buff_removal" ADD COLUMN IF NOT EXISTS "spell_id" INTEGER NOT NULL CHECK ("spell_id" >= 0);
ALTER TABLE IF EXISTS "instance_buff_removal" ADD COLUMN IF NOT EXISTS "enabled" SMALLINT NOT NULL;
ALTER TABLE IF EXISTS "instance_buff_removal" ADD COLUMN IF NOT EXISTS "flags" INTEGER NOT NULL;
ALTER TABLE IF EXISTS "instance_buff_removal" ADD COLUMN IF NOT EXISTS "comment" VARCHAR(256) NOT NULL;

CREATE TABLE IF NOT EXISTS "item_display_info" (
    "ID" INTEGER NOT NULL,
    "icon" VARCHAR(255) DEFAULT NULL
);
ALTER TABLE IF EXISTS "item_display_info" ADD COLUMN IF NOT EXISTS "ID" INTEGER NOT NULL;
ALTER TABLE IF EXISTS "item_display_info" ADD COLUMN IF NOT EXISTS "icon" VARCHAR(255) DEFAULT NULL;

CREATE TABLE IF NOT EXISTS "item_enchantment_template" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "ench" BIGINT NOT NULL DEFAULT '0' CHECK ("ench" >= 0),
    "chance" REAL NOT NULL DEFAULT '0' CHECK ("chance" >= 0),
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0),
    PRIMARY KEY ("entry", "ench", "patch_min", "patch_max")
);
ALTER TABLE IF EXISTS "item_enchantment_template" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "item_enchantment_template" ADD COLUMN IF NOT EXISTS "ench" BIGINT NOT NULL DEFAULT '0' CHECK ("ench" >= 0);
ALTER TABLE IF EXISTS "item_enchantment_template" ADD COLUMN IF NOT EXISTS "chance" REAL NOT NULL DEFAULT '0' CHECK ("chance" >= 0);
ALTER TABLE IF EXISTS "item_enchantment_template" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);
ALTER TABLE IF EXISTS "item_enchantment_template" ADD COLUMN IF NOT EXISTS "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0);

CREATE TABLE IF NOT EXISTS "item_loot_template" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "item" BIGINT NOT NULL DEFAULT '0' CHECK ("item" >= 0),
    "ChanceOrQuestChance" REAL NOT NULL DEFAULT '100',
    "groupid" SMALLINT NOT NULL DEFAULT '0' CHECK ("groupid" >= 0),
    "mincountOrRef" INTEGER NOT NULL DEFAULT '1',
    "maxcount" SMALLINT NOT NULL DEFAULT '1' CHECK ("maxcount" >= 0),
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0),
    PRIMARY KEY ("entry", "item", "patch_min", "patch_max")
);
ALTER TABLE IF EXISTS "item_loot_template" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "item_loot_template" ADD COLUMN IF NOT EXISTS "item" BIGINT NOT NULL DEFAULT '0' CHECK ("item" >= 0);
ALTER TABLE IF EXISTS "item_loot_template" ADD COLUMN IF NOT EXISTS "ChanceOrQuestChance" REAL NOT NULL DEFAULT '100';
ALTER TABLE IF EXISTS "item_loot_template" ADD COLUMN IF NOT EXISTS "groupid" SMALLINT NOT NULL DEFAULT '0' CHECK ("groupid" >= 0);
ALTER TABLE IF EXISTS "item_loot_template" ADD COLUMN IF NOT EXISTS "mincountOrRef" INTEGER NOT NULL DEFAULT '1';
ALTER TABLE IF EXISTS "item_loot_template" ADD COLUMN IF NOT EXISTS "maxcount" SMALLINT NOT NULL DEFAULT '1' CHECK ("maxcount" >= 0);
ALTER TABLE IF EXISTS "item_loot_template" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);
ALTER TABLE IF EXISTS "item_loot_template" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);
ALTER TABLE IF EXISTS "item_loot_template" ADD COLUMN IF NOT EXISTS "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0);

CREATE TABLE IF NOT EXISTS "item_required_target" (
    "entry" BIGINT NOT NULL CHECK ("entry" >= 0),
    "type" SMALLINT NOT NULL DEFAULT '0' CHECK ("type" >= 0),
    "target_entry" BIGINT NOT NULL DEFAULT '0' CHECK ("target_entry" >= 0),
    UNIQUE ("entry", "type", "target_entry")
);
ALTER TABLE IF EXISTS "item_required_target" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "item_required_target" ADD COLUMN IF NOT EXISTS "type" SMALLINT NOT NULL DEFAULT '0' CHECK ("type" >= 0);
ALTER TABLE IF EXISTS "item_required_target" ADD COLUMN IF NOT EXISTS "target_entry" BIGINT NOT NULL DEFAULT '0' CHECK ("target_entry" >= 0);
CREATE UNIQUE INDEX IF NOT EXISTS idx_item_required_target_entry_type_target ON "item_required_target" ("entry", "type", "target_entry");

CREATE TABLE IF NOT EXISTS "item_template" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "patch" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch" >= 0),
    "class" SMALLINT NOT NULL DEFAULT '0' CHECK ("class" >= 0),
    "subclass" SMALLINT NOT NULL DEFAULT '0' CHECK ("subclass" >= 0),
    "name" VARCHAR(255) NOT NULL DEFAULT '',
    "description" VARCHAR(255) NOT NULL DEFAULT '',
    "display_id" BIGINT NOT NULL DEFAULT '0' CHECK ("display_id" >= 0),
    "quality" SMALLINT NOT NULL DEFAULT '0' CHECK ("quality" >= 0),
    "flags" BIGINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0),
    "buy_count" SMALLINT NOT NULL DEFAULT '1' CHECK ("buy_count" >= 0),
    "buy_price" BIGINT NOT NULL DEFAULT '0' CHECK ("buy_price" >= 0),
    "sell_price" BIGINT NOT NULL DEFAULT '0' CHECK ("sell_price" >= 0),
    "inventory_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("inventory_type" >= 0),
    "allowable_class" INTEGER NOT NULL DEFAULT '-1',
    "allowable_race" INTEGER NOT NULL DEFAULT '-1',
    "item_level" SMALLINT NOT NULL DEFAULT '0' CHECK ("item_level" >= 0),
    "required_level" SMALLINT NOT NULL DEFAULT '0' CHECK ("required_level" >= 0),
    "required_skill" INTEGER NOT NULL DEFAULT '0' CHECK ("required_skill" >= 0),
    "required_skill_rank" INTEGER NOT NULL DEFAULT '0' CHECK ("required_skill_rank" >= 0),
    "required_spell" INTEGER NOT NULL DEFAULT '0' CHECK ("required_spell" >= 0),
    "required_honor_rank" BIGINT NOT NULL DEFAULT '0' CHECK ("required_honor_rank" >= 0),
    "required_city_rank" BIGINT NOT NULL DEFAULT '0' CHECK ("required_city_rank" >= 0),
    "required_reputation_faction" INTEGER NOT NULL DEFAULT '0' CHECK ("required_reputation_faction" >= 0),
    "required_reputation_rank" INTEGER NOT NULL DEFAULT '0' CHECK ("required_reputation_rank" >= 0),
    "max_count" INTEGER NOT NULL DEFAULT '0' CHECK ("max_count" >= 0),
    "stackable" INTEGER NOT NULL DEFAULT '1' CHECK ("stackable" >= 0),
    "container_slots" SMALLINT NOT NULL DEFAULT '0' CHECK ("container_slots" >= 0),
    "stat_type1" SMALLINT NOT NULL DEFAULT '0' CHECK ("stat_type1" >= 0),
    "stat_value1" SMALLINT NOT NULL DEFAULT '0',
    "stat_type2" SMALLINT NOT NULL DEFAULT '0' CHECK ("stat_type2" >= 0),
    "stat_value2" SMALLINT NOT NULL DEFAULT '0',
    "stat_type3" SMALLINT NOT NULL DEFAULT '0' CHECK ("stat_type3" >= 0),
    "stat_value3" SMALLINT NOT NULL DEFAULT '0',
    "stat_type4" SMALLINT NOT NULL DEFAULT '0' CHECK ("stat_type4" >= 0),
    "stat_value4" SMALLINT NOT NULL DEFAULT '0',
    "stat_type5" SMALLINT NOT NULL DEFAULT '0' CHECK ("stat_type5" >= 0),
    "stat_value5" SMALLINT NOT NULL DEFAULT '0',
    "stat_type6" SMALLINT NOT NULL DEFAULT '0' CHECK ("stat_type6" >= 0),
    "stat_value6" SMALLINT NOT NULL DEFAULT '0',
    "stat_type7" SMALLINT NOT NULL DEFAULT '0' CHECK ("stat_type7" >= 0),
    "stat_value7" SMALLINT NOT NULL DEFAULT '0',
    "stat_type8" SMALLINT NOT NULL DEFAULT '0' CHECK ("stat_type8" >= 0),
    "stat_value8" SMALLINT NOT NULL DEFAULT '0',
    "stat_type9" SMALLINT NOT NULL DEFAULT '0' CHECK ("stat_type9" >= 0),
    "stat_value9" SMALLINT NOT NULL DEFAULT '0',
    "stat_type10" SMALLINT NOT NULL DEFAULT '0' CHECK ("stat_type10" >= 0),
    "stat_value10" SMALLINT NOT NULL DEFAULT '0',
    "delay" INTEGER NOT NULL DEFAULT '1000' CHECK ("delay" >= 0),
    "range_mod" REAL NOT NULL DEFAULT '0',
    "ammo_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("ammo_type" >= 0),
    "dmg_min1" REAL NOT NULL DEFAULT '0',
    "dmg_max1" REAL NOT NULL DEFAULT '0',
    "dmg_type1" SMALLINT NOT NULL DEFAULT '0' CHECK ("dmg_type1" >= 0),
    "dmg_min2" REAL NOT NULL DEFAULT '0',
    "dmg_max2" REAL NOT NULL DEFAULT '0',
    "dmg_type2" SMALLINT NOT NULL DEFAULT '0' CHECK ("dmg_type2" >= 0),
    "dmg_min3" REAL NOT NULL DEFAULT '0',
    "dmg_max3" REAL NOT NULL DEFAULT '0',
    "dmg_type3" SMALLINT NOT NULL DEFAULT '0' CHECK ("dmg_type3" >= 0),
    "dmg_min4" REAL NOT NULL DEFAULT '0',
    "dmg_max4" REAL NOT NULL DEFAULT '0',
    "dmg_type4" SMALLINT NOT NULL DEFAULT '0' CHECK ("dmg_type4" >= 0),
    "dmg_min5" REAL NOT NULL DEFAULT '0',
    "dmg_max5" REAL NOT NULL DEFAULT '0',
    "dmg_type5" SMALLINT NOT NULL DEFAULT '0' CHECK ("dmg_type5" >= 0),
    "block" BIGINT NOT NULL DEFAULT '0' CHECK ("block" >= 0),
    "armor" SMALLINT NOT NULL DEFAULT '0',
    "holy_res" SMALLINT NOT NULL DEFAULT '0',
    "fire_res" SMALLINT NOT NULL DEFAULT '0',
    "nature_res" SMALLINT NOT NULL DEFAULT '0',
    "frost_res" SMALLINT NOT NULL DEFAULT '0',
    "shadow_res" SMALLINT NOT NULL DEFAULT '0',
    "arcane_res" SMALLINT NOT NULL DEFAULT '0',
    "spellid_1" INTEGER NOT NULL DEFAULT '0' CHECK ("spellid_1" >= 0),
    "spelltrigger_1" SMALLINT NOT NULL DEFAULT '0' CHECK ("spelltrigger_1" >= 0),
    "spellcharges_1" SMALLINT NOT NULL DEFAULT '0',
    "spellppmrate_1" REAL NOT NULL DEFAULT '0',
    "spellcooldown_1" INTEGER NOT NULL DEFAULT '-1',
    "spellcategory_1" INTEGER NOT NULL DEFAULT '0' CHECK ("spellcategory_1" >= 0),
    "spellcategorycooldown_1" INTEGER NOT NULL DEFAULT '-1',
    "spellid_2" INTEGER NOT NULL DEFAULT '0' CHECK ("spellid_2" >= 0),
    "spelltrigger_2" SMALLINT NOT NULL DEFAULT '0' CHECK ("spelltrigger_2" >= 0),
    "spellcharges_2" SMALLINT NOT NULL DEFAULT '0',
    "spellppmrate_2" REAL NOT NULL DEFAULT '0',
    "spellcooldown_2" INTEGER NOT NULL DEFAULT '-1',
    "spellcategory_2" INTEGER NOT NULL DEFAULT '0' CHECK ("spellcategory_2" >= 0),
    "spellcategorycooldown_2" INTEGER NOT NULL DEFAULT '-1',
    "spellid_3" INTEGER NOT NULL DEFAULT '0' CHECK ("spellid_3" >= 0),
    "spelltrigger_3" SMALLINT NOT NULL DEFAULT '0' CHECK ("spelltrigger_3" >= 0),
    "spellcharges_3" SMALLINT NOT NULL DEFAULT '0',
    "spellppmrate_3" REAL NOT NULL DEFAULT '0',
    "spellcooldown_3" INTEGER NOT NULL DEFAULT '-1',
    "spellcategory_3" INTEGER NOT NULL DEFAULT '0' CHECK ("spellcategory_3" >= 0),
    "spellcategorycooldown_3" INTEGER NOT NULL DEFAULT '-1',
    "spellid_4" INTEGER NOT NULL DEFAULT '0' CHECK ("spellid_4" >= 0),
    "spelltrigger_4" SMALLINT NOT NULL DEFAULT '0' CHECK ("spelltrigger_4" >= 0),
    "spellcharges_4" SMALLINT NOT NULL DEFAULT '0',
    "spellppmrate_4" REAL NOT NULL DEFAULT '0',
    "spellcooldown_4" INTEGER NOT NULL DEFAULT '-1',
    "spellcategory_4" INTEGER NOT NULL DEFAULT '0' CHECK ("spellcategory_4" >= 0),
    "spellcategorycooldown_4" INTEGER NOT NULL DEFAULT '-1',
    "spellid_5" INTEGER NOT NULL DEFAULT '0' CHECK ("spellid_5" >= 0),
    "spelltrigger_5" SMALLINT NOT NULL DEFAULT '0' CHECK ("spelltrigger_5" >= 0),
    "spellcharges_5" SMALLINT NOT NULL DEFAULT '0',
    "spellppmrate_5" REAL NOT NULL DEFAULT '0',
    "spellcooldown_5" INTEGER NOT NULL DEFAULT '-1',
    "spellcategory_5" INTEGER NOT NULL DEFAULT '0' CHECK ("spellcategory_5" >= 0),
    "spellcategorycooldown_5" INTEGER NOT NULL DEFAULT '-1',
    "bonding" SMALLINT NOT NULL DEFAULT '0' CHECK ("bonding" >= 0),
    "page_text" BIGINT NOT NULL DEFAULT '0' CHECK ("page_text" >= 0),
    "page_language" SMALLINT NOT NULL DEFAULT '0' CHECK ("page_language" >= 0),
    "page_material" SMALLINT NOT NULL DEFAULT '0' CHECK ("page_material" >= 0),
    "start_quest" BIGINT NOT NULL DEFAULT '0' CHECK ("start_quest" >= 0),
    "lock_id" BIGINT NOT NULL DEFAULT '0' CHECK ("lock_id" >= 0),
    "material" SMALLINT NOT NULL DEFAULT '0',
    "sheath" SMALLINT NOT NULL DEFAULT '0' CHECK ("sheath" >= 0),
    "random_property" BIGINT NOT NULL DEFAULT '0' CHECK ("random_property" >= 0),
    "set_id" BIGINT NOT NULL DEFAULT '0' CHECK ("set_id" >= 0),
    "max_durability" INTEGER NOT NULL DEFAULT '0' CHECK ("max_durability" >= 0),
    "area_bound" BIGINT NOT NULL DEFAULT '0' CHECK ("area_bound" >= 0),
    "map_bound" SMALLINT NOT NULL DEFAULT '0',
    "duration" BIGINT NOT NULL DEFAULT '0' CHECK ("duration" >= 0),
    "bag_family" INTEGER NOT NULL DEFAULT '0',
    "disenchant_id" BIGINT NOT NULL DEFAULT '0' CHECK ("disenchant_id" >= 0),
    "food_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("food_type" >= 0),
    "min_money_loot" BIGINT NOT NULL DEFAULT '0' CHECK ("min_money_loot" >= 0),
    "max_money_loot" BIGINT NOT NULL DEFAULT '0' CHECK ("max_money_loot" >= 0),
    "wrapped_gift" BIGINT NOT NULL DEFAULT '0' CHECK ("wrapped_gift" >= 0),
    "extra_flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("extra_flags" >= 0),
    "other_team_entry" BIGINT DEFAULT '1' CHECK ("other_team_entry" >= 0),
    PRIMARY KEY ("entry", "patch")
);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "patch" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "class" SMALLINT NOT NULL DEFAULT '0' CHECK ("class" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "subclass" SMALLINT NOT NULL DEFAULT '0' CHECK ("subclass" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "name" VARCHAR(255) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "description" VARCHAR(255) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "display_id" BIGINT NOT NULL DEFAULT '0' CHECK ("display_id" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "quality" SMALLINT NOT NULL DEFAULT '0' CHECK ("quality" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "flags" BIGINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "buy_count" SMALLINT NOT NULL DEFAULT '1' CHECK ("buy_count" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "buy_price" BIGINT NOT NULL DEFAULT '0' CHECK ("buy_price" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "sell_price" BIGINT NOT NULL DEFAULT '0' CHECK ("sell_price" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "inventory_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("inventory_type" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "allowable_class" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "allowable_race" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "item_level" SMALLINT NOT NULL DEFAULT '0' CHECK ("item_level" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "required_level" SMALLINT NOT NULL DEFAULT '0' CHECK ("required_level" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "required_skill" INTEGER NOT NULL DEFAULT '0' CHECK ("required_skill" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "required_skill_rank" INTEGER NOT NULL DEFAULT '0' CHECK ("required_skill_rank" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "required_spell" INTEGER NOT NULL DEFAULT '0' CHECK ("required_spell" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "required_honor_rank" BIGINT NOT NULL DEFAULT '0' CHECK ("required_honor_rank" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "required_city_rank" BIGINT NOT NULL DEFAULT '0' CHECK ("required_city_rank" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "required_reputation_faction" INTEGER NOT NULL DEFAULT '0' CHECK ("required_reputation_faction" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "required_reputation_rank" INTEGER NOT NULL DEFAULT '0' CHECK ("required_reputation_rank" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "max_count" INTEGER NOT NULL DEFAULT '0' CHECK ("max_count" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "stackable" INTEGER NOT NULL DEFAULT '1' CHECK ("stackable" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "container_slots" SMALLINT NOT NULL DEFAULT '0' CHECK ("container_slots" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "stat_type1" SMALLINT NOT NULL DEFAULT '0' CHECK ("stat_type1" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "stat_value1" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "stat_type2" SMALLINT NOT NULL DEFAULT '0' CHECK ("stat_type2" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "stat_value2" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "stat_type3" SMALLINT NOT NULL DEFAULT '0' CHECK ("stat_type3" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "stat_value3" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "stat_type4" SMALLINT NOT NULL DEFAULT '0' CHECK ("stat_type4" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "stat_value4" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "stat_type5" SMALLINT NOT NULL DEFAULT '0' CHECK ("stat_type5" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "stat_value5" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "stat_type6" SMALLINT NOT NULL DEFAULT '0' CHECK ("stat_type6" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "stat_value6" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "stat_type7" SMALLINT NOT NULL DEFAULT '0' CHECK ("stat_type7" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "stat_value7" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "stat_type8" SMALLINT NOT NULL DEFAULT '0' CHECK ("stat_type8" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "stat_value8" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "stat_type9" SMALLINT NOT NULL DEFAULT '0' CHECK ("stat_type9" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "stat_value9" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "stat_type10" SMALLINT NOT NULL DEFAULT '0' CHECK ("stat_type10" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "stat_value10" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "delay" INTEGER NOT NULL DEFAULT '1000' CHECK ("delay" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "range_mod" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "ammo_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("ammo_type" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "dmg_min1" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "dmg_max1" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "dmg_type1" SMALLINT NOT NULL DEFAULT '0' CHECK ("dmg_type1" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "dmg_min2" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "dmg_max2" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "dmg_type2" SMALLINT NOT NULL DEFAULT '0' CHECK ("dmg_type2" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "dmg_min3" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "dmg_max3" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "dmg_type3" SMALLINT NOT NULL DEFAULT '0' CHECK ("dmg_type3" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "dmg_min4" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "dmg_max4" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "dmg_type4" SMALLINT NOT NULL DEFAULT '0' CHECK ("dmg_type4" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "dmg_min5" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "dmg_max5" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "dmg_type5" SMALLINT NOT NULL DEFAULT '0' CHECK ("dmg_type5" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "block" BIGINT NOT NULL DEFAULT '0' CHECK ("block" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "armor" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "holy_res" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "fire_res" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "nature_res" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "frost_res" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "shadow_res" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "arcane_res" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellid_1" INTEGER NOT NULL DEFAULT '0' CHECK ("spellid_1" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spelltrigger_1" SMALLINT NOT NULL DEFAULT '0' CHECK ("spelltrigger_1" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellcharges_1" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellppmrate_1" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellcooldown_1" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellcategory_1" INTEGER NOT NULL DEFAULT '0' CHECK ("spellcategory_1" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellcategorycooldown_1" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellid_2" INTEGER NOT NULL DEFAULT '0' CHECK ("spellid_2" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spelltrigger_2" SMALLINT NOT NULL DEFAULT '0' CHECK ("spelltrigger_2" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellcharges_2" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellppmrate_2" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellcooldown_2" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellcategory_2" INTEGER NOT NULL DEFAULT '0' CHECK ("spellcategory_2" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellcategorycooldown_2" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellid_3" INTEGER NOT NULL DEFAULT '0' CHECK ("spellid_3" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spelltrigger_3" SMALLINT NOT NULL DEFAULT '0' CHECK ("spelltrigger_3" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellcharges_3" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellppmrate_3" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellcooldown_3" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellcategory_3" INTEGER NOT NULL DEFAULT '0' CHECK ("spellcategory_3" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellcategorycooldown_3" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellid_4" INTEGER NOT NULL DEFAULT '0' CHECK ("spellid_4" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spelltrigger_4" SMALLINT NOT NULL DEFAULT '0' CHECK ("spelltrigger_4" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellcharges_4" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellppmrate_4" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellcooldown_4" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellcategory_4" INTEGER NOT NULL DEFAULT '0' CHECK ("spellcategory_4" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellcategorycooldown_4" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellid_5" INTEGER NOT NULL DEFAULT '0' CHECK ("spellid_5" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spelltrigger_5" SMALLINT NOT NULL DEFAULT '0' CHECK ("spelltrigger_5" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellcharges_5" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellppmrate_5" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellcooldown_5" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellcategory_5" INTEGER NOT NULL DEFAULT '0' CHECK ("spellcategory_5" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "spellcategorycooldown_5" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "bonding" SMALLINT NOT NULL DEFAULT '0' CHECK ("bonding" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "page_text" BIGINT NOT NULL DEFAULT '0' CHECK ("page_text" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "page_language" SMALLINT NOT NULL DEFAULT '0' CHECK ("page_language" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "page_material" SMALLINT NOT NULL DEFAULT '0' CHECK ("page_material" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "start_quest" BIGINT NOT NULL DEFAULT '0' CHECK ("start_quest" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "lock_id" BIGINT NOT NULL DEFAULT '0' CHECK ("lock_id" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "material" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "sheath" SMALLINT NOT NULL DEFAULT '0' CHECK ("sheath" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "random_property" BIGINT NOT NULL DEFAULT '0' CHECK ("random_property" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "set_id" BIGINT NOT NULL DEFAULT '0' CHECK ("set_id" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "max_durability" INTEGER NOT NULL DEFAULT '0' CHECK ("max_durability" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "area_bound" BIGINT NOT NULL DEFAULT '0' CHECK ("area_bound" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "map_bound" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "duration" BIGINT NOT NULL DEFAULT '0' CHECK ("duration" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "bag_family" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "disenchant_id" BIGINT NOT NULL DEFAULT '0' CHECK ("disenchant_id" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "food_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("food_type" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "min_money_loot" BIGINT NOT NULL DEFAULT '0' CHECK ("min_money_loot" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "max_money_loot" BIGINT NOT NULL DEFAULT '0' CHECK ("max_money_loot" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "wrapped_gift" BIGINT NOT NULL DEFAULT '0' CHECK ("wrapped_gift" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "extra_flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("extra_flags" >= 0);
ALTER TABLE IF EXISTS "item_template" ADD COLUMN IF NOT EXISTS "other_team_entry" BIGINT DEFAULT '1' CHECK ("other_team_entry" >= 0);
CREATE INDEX IF NOT EXISTS idx_item_template_items_index ON "item_template" ("class");

CREATE TABLE IF NOT EXISTS "locales_area" (
    "Entry" INTEGER NOT NULL DEFAULT '0',
    "NameLoc1" VARCHAR(100) NOT NULL DEFAULT '',
    "NameLoc2" VARCHAR(100) NOT NULL DEFAULT '',
    "NameLoc3" VARCHAR(100) NOT NULL DEFAULT '',
    "NameLoc4" VARCHAR(100) NOT NULL DEFAULT '',
    "NameLoc5" VARCHAR(100) NOT NULL DEFAULT '',
    "NameLoc6" VARCHAR(100) NOT NULL DEFAULT '',
    "NameLoc7" VARCHAR(100) NOT NULL DEFAULT '',
    "NameLoc8" VARCHAR(100) NOT NULL DEFAULT '',
    PRIMARY KEY ("Entry")
);
ALTER TABLE IF EXISTS "locales_area" ADD COLUMN IF NOT EXISTS "Entry" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "locales_area" ADD COLUMN IF NOT EXISTS "NameLoc1" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_area" ADD COLUMN IF NOT EXISTS "NameLoc2" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_area" ADD COLUMN IF NOT EXISTS "NameLoc3" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_area" ADD COLUMN IF NOT EXISTS "NameLoc4" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_area" ADD COLUMN IF NOT EXISTS "NameLoc5" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_area" ADD COLUMN IF NOT EXISTS "NameLoc6" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_area" ADD COLUMN IF NOT EXISTS "NameLoc7" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_area" ADD COLUMN IF NOT EXISTS "NameLoc8" VARCHAR(100) NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS "locales_areatrigger" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "message_loc1" VARCHAR(200) NOT NULL DEFAULT '',
    "message_loc2" VARCHAR(200) NOT NULL DEFAULT '',
    "message_loc3" VARCHAR(200) NOT NULL DEFAULT '',
    "message_loc4" VARCHAR(200) NOT NULL DEFAULT '',
    "message_loc5" VARCHAR(200) NOT NULL DEFAULT '',
    "message_loc6" VARCHAR(200) NOT NULL DEFAULT '',
    "message_loc7" VARCHAR(200) NOT NULL DEFAULT '',
    "message_loc8" VARCHAR(200) NOT NULL DEFAULT '',
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "locales_areatrigger" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "locales_areatrigger" ADD COLUMN IF NOT EXISTS "message_loc1" VARCHAR(200) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_areatrigger" ADD COLUMN IF NOT EXISTS "message_loc2" VARCHAR(200) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_areatrigger" ADD COLUMN IF NOT EXISTS "message_loc3" VARCHAR(200) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_areatrigger" ADD COLUMN IF NOT EXISTS "message_loc4" VARCHAR(200) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_areatrigger" ADD COLUMN IF NOT EXISTS "message_loc5" VARCHAR(200) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_areatrigger" ADD COLUMN IF NOT EXISTS "message_loc6" VARCHAR(200) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_areatrigger" ADD COLUMN IF NOT EXISTS "message_loc7" VARCHAR(200) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_areatrigger" ADD COLUMN IF NOT EXISTS "message_loc8" VARCHAR(200) NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS "locales_broadcast_text" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "male_text_loc1" TEXT,
    "male_text_loc2" TEXT,
    "male_text_loc3" TEXT,
    "male_text_loc4" TEXT,
    "male_text_loc5" TEXT,
    "male_text_loc6" TEXT,
    "male_text_loc7" TEXT,
    "male_text_loc8" TEXT,
    "female_text_loc1" TEXT,
    "female_text_loc2" TEXT,
    "female_text_loc3" TEXT,
    "female_text_loc4" TEXT,
    "female_text_loc5" TEXT,
    "female_text_loc6" TEXT,
    "female_text_loc7" TEXT,
    "female_text_loc8" TEXT,
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "locales_broadcast_text" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "locales_broadcast_text" ADD COLUMN IF NOT EXISTS "male_text_loc1" TEXT;
ALTER TABLE IF EXISTS "locales_broadcast_text" ADD COLUMN IF NOT EXISTS "male_text_loc2" TEXT;
ALTER TABLE IF EXISTS "locales_broadcast_text" ADD COLUMN IF NOT EXISTS "male_text_loc3" TEXT;
ALTER TABLE IF EXISTS "locales_broadcast_text" ADD COLUMN IF NOT EXISTS "male_text_loc4" TEXT;
ALTER TABLE IF EXISTS "locales_broadcast_text" ADD COLUMN IF NOT EXISTS "male_text_loc5" TEXT;
ALTER TABLE IF EXISTS "locales_broadcast_text" ADD COLUMN IF NOT EXISTS "male_text_loc6" TEXT;
ALTER TABLE IF EXISTS "locales_broadcast_text" ADD COLUMN IF NOT EXISTS "male_text_loc7" TEXT;
ALTER TABLE IF EXISTS "locales_broadcast_text" ADD COLUMN IF NOT EXISTS "male_text_loc8" TEXT;
ALTER TABLE IF EXISTS "locales_broadcast_text" ADD COLUMN IF NOT EXISTS "female_text_loc1" TEXT;
ALTER TABLE IF EXISTS "locales_broadcast_text" ADD COLUMN IF NOT EXISTS "female_text_loc2" TEXT;
ALTER TABLE IF EXISTS "locales_broadcast_text" ADD COLUMN IF NOT EXISTS "female_text_loc3" TEXT;
ALTER TABLE IF EXISTS "locales_broadcast_text" ADD COLUMN IF NOT EXISTS "female_text_loc4" TEXT;
ALTER TABLE IF EXISTS "locales_broadcast_text" ADD COLUMN IF NOT EXISTS "female_text_loc5" TEXT;
ALTER TABLE IF EXISTS "locales_broadcast_text" ADD COLUMN IF NOT EXISTS "female_text_loc6" TEXT;
ALTER TABLE IF EXISTS "locales_broadcast_text" ADD COLUMN IF NOT EXISTS "female_text_loc7" TEXT;
ALTER TABLE IF EXISTS "locales_broadcast_text" ADD COLUMN IF NOT EXISTS "female_text_loc8" TEXT;

CREATE TABLE IF NOT EXISTS "locales_creature" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "name_loc1" VARCHAR(100) NOT NULL DEFAULT '',
    "name_loc2" VARCHAR(100) NOT NULL DEFAULT '',
    "name_loc3" VARCHAR(100) NOT NULL DEFAULT '',
    "name_loc4" VARCHAR(100) NOT NULL DEFAULT '',
    "name_loc5" VARCHAR(100) NOT NULL DEFAULT '',
    "name_loc6" VARCHAR(100) NOT NULL DEFAULT '',
    "name_loc7" VARCHAR(100) NOT NULL DEFAULT '',
    "name_loc8" VARCHAR(100) NOT NULL DEFAULT '',
    "subname_loc1" VARCHAR(100) DEFAULT NULL,
    "subname_loc2" VARCHAR(100) DEFAULT NULL,
    "subname_loc3" VARCHAR(100) DEFAULT NULL,
    "subname_loc4" VARCHAR(100) DEFAULT NULL,
    "subname_loc5" VARCHAR(100) DEFAULT NULL,
    "subname_loc6" VARCHAR(100) DEFAULT NULL,
    "subname_loc7" VARCHAR(100) DEFAULT NULL,
    "subname_loc8" VARCHAR(100) DEFAULT NULL,
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "locales_creature" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "locales_creature" ADD COLUMN IF NOT EXISTS "name_loc1" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_creature" ADD COLUMN IF NOT EXISTS "name_loc2" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_creature" ADD COLUMN IF NOT EXISTS "name_loc3" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_creature" ADD COLUMN IF NOT EXISTS "name_loc4" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_creature" ADD COLUMN IF NOT EXISTS "name_loc5" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_creature" ADD COLUMN IF NOT EXISTS "name_loc6" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_creature" ADD COLUMN IF NOT EXISTS "name_loc7" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_creature" ADD COLUMN IF NOT EXISTS "name_loc8" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_creature" ADD COLUMN IF NOT EXISTS "subname_loc1" VARCHAR(100) DEFAULT NULL;
ALTER TABLE IF EXISTS "locales_creature" ADD COLUMN IF NOT EXISTS "subname_loc2" VARCHAR(100) DEFAULT NULL;
ALTER TABLE IF EXISTS "locales_creature" ADD COLUMN IF NOT EXISTS "subname_loc3" VARCHAR(100) DEFAULT NULL;
ALTER TABLE IF EXISTS "locales_creature" ADD COLUMN IF NOT EXISTS "subname_loc4" VARCHAR(100) DEFAULT NULL;
ALTER TABLE IF EXISTS "locales_creature" ADD COLUMN IF NOT EXISTS "subname_loc5" VARCHAR(100) DEFAULT NULL;
ALTER TABLE IF EXISTS "locales_creature" ADD COLUMN IF NOT EXISTS "subname_loc6" VARCHAR(100) DEFAULT NULL;
ALTER TABLE IF EXISTS "locales_creature" ADD COLUMN IF NOT EXISTS "subname_loc7" VARCHAR(100) DEFAULT NULL;
ALTER TABLE IF EXISTS "locales_creature" ADD COLUMN IF NOT EXISTS "subname_loc8" VARCHAR(100) DEFAULT NULL;

CREATE TABLE IF NOT EXISTS "locales_faction" (
    "entry" BIGINT NOT NULL CHECK ("entry" >= 0),
    "name_loc1" VARCHAR(256) NOT NULL DEFAULT '',
    "name_loc2" VARCHAR(256) NOT NULL DEFAULT '',
    "name_loc3" VARCHAR(256) NOT NULL DEFAULT '',
    "name_loc4" VARCHAR(256) NOT NULL DEFAULT '',
    "name_loc5" VARCHAR(256) NOT NULL DEFAULT '',
    "name_loc6" VARCHAR(256) NOT NULL DEFAULT '',
    "description_loc1" VARCHAR(512) NOT NULL DEFAULT '',
    "description_loc2" VARCHAR(512) NOT NULL DEFAULT '',
    "description_loc3" VARCHAR(512) NOT NULL DEFAULT '',
    "description_loc4" VARCHAR(512) NOT NULL DEFAULT '',
    "description_loc5" VARCHAR(512) NOT NULL DEFAULT '',
    "description_loc6" VARCHAR(512) NOT NULL DEFAULT '',
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "locales_faction" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "locales_faction" ADD COLUMN IF NOT EXISTS "name_loc1" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_faction" ADD COLUMN IF NOT EXISTS "name_loc2" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_faction" ADD COLUMN IF NOT EXISTS "name_loc3" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_faction" ADD COLUMN IF NOT EXISTS "name_loc4" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_faction" ADD COLUMN IF NOT EXISTS "name_loc5" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_faction" ADD COLUMN IF NOT EXISTS "name_loc6" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_faction" ADD COLUMN IF NOT EXISTS "description_loc1" VARCHAR(512) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_faction" ADD COLUMN IF NOT EXISTS "description_loc2" VARCHAR(512) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_faction" ADD COLUMN IF NOT EXISTS "description_loc3" VARCHAR(512) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_faction" ADD COLUMN IF NOT EXISTS "description_loc4" VARCHAR(512) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_faction" ADD COLUMN IF NOT EXISTS "description_loc5" VARCHAR(512) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_faction" ADD COLUMN IF NOT EXISTS "description_loc6" VARCHAR(512) NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS "locales_gameobject" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "name_loc1" VARCHAR(100) NOT NULL DEFAULT '',
    "name_loc2" VARCHAR(100) NOT NULL DEFAULT '',
    "name_loc3" VARCHAR(100) NOT NULL DEFAULT '',
    "name_loc4" VARCHAR(100) NOT NULL DEFAULT '',
    "name_loc5" VARCHAR(100) NOT NULL DEFAULT '',
    "name_loc6" VARCHAR(100) NOT NULL DEFAULT '',
    "name_loc7" VARCHAR(100) NOT NULL DEFAULT '',
    "name_loc8" VARCHAR(100) NOT NULL DEFAULT '',
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "locales_gameobject" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "locales_gameobject" ADD COLUMN IF NOT EXISTS "name_loc1" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_gameobject" ADD COLUMN IF NOT EXISTS "name_loc2" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_gameobject" ADD COLUMN IF NOT EXISTS "name_loc3" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_gameobject" ADD COLUMN IF NOT EXISTS "name_loc4" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_gameobject" ADD COLUMN IF NOT EXISTS "name_loc5" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_gameobject" ADD COLUMN IF NOT EXISTS "name_loc6" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_gameobject" ADD COLUMN IF NOT EXISTS "name_loc7" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_gameobject" ADD COLUMN IF NOT EXISTS "name_loc8" VARCHAR(100) NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS "locales_gossip_menu_option" (
    "menu_id" INTEGER NOT NULL DEFAULT '0' CHECK ("menu_id" >= 0),
    "id" INTEGER NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "option_text_loc1" TEXT,
    "option_text_loc2" TEXT,
    "option_text_loc3" TEXT,
    "option_text_loc4" TEXT,
    "option_text_loc5" TEXT,
    "option_text_loc6" TEXT,
    "option_text_loc7" TEXT,
    "option_text_loc8" TEXT,
    "box_text_loc1" TEXT,
    "box_text_loc2" TEXT,
    "box_text_loc3" TEXT,
    "box_text_loc4" TEXT,
    "box_text_loc5" TEXT,
    "box_text_loc6" TEXT,
    "box_text_loc7" TEXT,
    "box_text_loc8" TEXT,
    PRIMARY KEY ("menu_id", "id")
);
ALTER TABLE IF EXISTS "locales_gossip_menu_option" ADD COLUMN IF NOT EXISTS "menu_id" INTEGER NOT NULL DEFAULT '0' CHECK ("menu_id" >= 0);
ALTER TABLE IF EXISTS "locales_gossip_menu_option" ADD COLUMN IF NOT EXISTS "id" INTEGER NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "locales_gossip_menu_option" ADD COLUMN IF NOT EXISTS "option_text_loc1" TEXT;
ALTER TABLE IF EXISTS "locales_gossip_menu_option" ADD COLUMN IF NOT EXISTS "option_text_loc2" TEXT;
ALTER TABLE IF EXISTS "locales_gossip_menu_option" ADD COLUMN IF NOT EXISTS "option_text_loc3" TEXT;
ALTER TABLE IF EXISTS "locales_gossip_menu_option" ADD COLUMN IF NOT EXISTS "option_text_loc4" TEXT;
ALTER TABLE IF EXISTS "locales_gossip_menu_option" ADD COLUMN IF NOT EXISTS "option_text_loc5" TEXT;
ALTER TABLE IF EXISTS "locales_gossip_menu_option" ADD COLUMN IF NOT EXISTS "option_text_loc6" TEXT;
ALTER TABLE IF EXISTS "locales_gossip_menu_option" ADD COLUMN IF NOT EXISTS "option_text_loc7" TEXT;
ALTER TABLE IF EXISTS "locales_gossip_menu_option" ADD COLUMN IF NOT EXISTS "option_text_loc8" TEXT;
ALTER TABLE IF EXISTS "locales_gossip_menu_option" ADD COLUMN IF NOT EXISTS "box_text_loc1" TEXT;
ALTER TABLE IF EXISTS "locales_gossip_menu_option" ADD COLUMN IF NOT EXISTS "box_text_loc2" TEXT;
ALTER TABLE IF EXISTS "locales_gossip_menu_option" ADD COLUMN IF NOT EXISTS "box_text_loc3" TEXT;
ALTER TABLE IF EXISTS "locales_gossip_menu_option" ADD COLUMN IF NOT EXISTS "box_text_loc4" TEXT;
ALTER TABLE IF EXISTS "locales_gossip_menu_option" ADD COLUMN IF NOT EXISTS "box_text_loc5" TEXT;
ALTER TABLE IF EXISTS "locales_gossip_menu_option" ADD COLUMN IF NOT EXISTS "box_text_loc6" TEXT;
ALTER TABLE IF EXISTS "locales_gossip_menu_option" ADD COLUMN IF NOT EXISTS "box_text_loc7" TEXT;
ALTER TABLE IF EXISTS "locales_gossip_menu_option" ADD COLUMN IF NOT EXISTS "box_text_loc8" TEXT;

CREATE TABLE IF NOT EXISTS "locales_item" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "name_loc1" VARCHAR(100) NOT NULL DEFAULT '',
    "name_loc2" VARCHAR(100) NOT NULL DEFAULT '',
    "name_loc3" VARCHAR(100) NOT NULL DEFAULT '',
    "name_loc4" VARCHAR(100) NOT NULL DEFAULT '',
    "name_loc5" VARCHAR(100) NOT NULL DEFAULT '',
    "name_loc6" VARCHAR(100) NOT NULL DEFAULT '',
    "name_loc7" VARCHAR(100) NOT NULL DEFAULT '',
    "name_loc8" VARCHAR(100) NOT NULL DEFAULT '',
    "description_loc1" VARCHAR(255) DEFAULT NULL,
    "description_loc2" VARCHAR(255) DEFAULT NULL,
    "description_loc3" VARCHAR(255) DEFAULT NULL,
    "description_loc4" VARCHAR(255) DEFAULT NULL,
    "description_loc5" VARCHAR(255) DEFAULT NULL,
    "description_loc6" VARCHAR(255) DEFAULT NULL,
    "description_loc7" VARCHAR(255) DEFAULT NULL,
    "description_loc8" VARCHAR(255) DEFAULT NULL,
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "locales_item" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "locales_item" ADD COLUMN IF NOT EXISTS "name_loc1" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_item" ADD COLUMN IF NOT EXISTS "name_loc2" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_item" ADD COLUMN IF NOT EXISTS "name_loc3" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_item" ADD COLUMN IF NOT EXISTS "name_loc4" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_item" ADD COLUMN IF NOT EXISTS "name_loc5" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_item" ADD COLUMN IF NOT EXISTS "name_loc6" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_item" ADD COLUMN IF NOT EXISTS "name_loc7" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_item" ADD COLUMN IF NOT EXISTS "name_loc8" VARCHAR(100) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_item" ADD COLUMN IF NOT EXISTS "description_loc1" VARCHAR(255) DEFAULT NULL;
ALTER TABLE IF EXISTS "locales_item" ADD COLUMN IF NOT EXISTS "description_loc2" VARCHAR(255) DEFAULT NULL;
ALTER TABLE IF EXISTS "locales_item" ADD COLUMN IF NOT EXISTS "description_loc3" VARCHAR(255) DEFAULT NULL;
ALTER TABLE IF EXISTS "locales_item" ADD COLUMN IF NOT EXISTS "description_loc4" VARCHAR(255) DEFAULT NULL;
ALTER TABLE IF EXISTS "locales_item" ADD COLUMN IF NOT EXISTS "description_loc5" VARCHAR(255) DEFAULT NULL;
ALTER TABLE IF EXISTS "locales_item" ADD COLUMN IF NOT EXISTS "description_loc6" VARCHAR(255) DEFAULT NULL;
ALTER TABLE IF EXISTS "locales_item" ADD COLUMN IF NOT EXISTS "description_loc7" VARCHAR(255) DEFAULT NULL;
ALTER TABLE IF EXISTS "locales_item" ADD COLUMN IF NOT EXISTS "description_loc8" VARCHAR(255) DEFAULT NULL;

CREATE TABLE IF NOT EXISTS "locales_page_text" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "Text_loc1" TEXT,
    "Text_loc2" TEXT,
    "Text_loc3" TEXT,
    "Text_loc4" TEXT,
    "Text_loc5" TEXT,
    "Text_loc6" TEXT,
    "Text_loc7" TEXT,
    "Text_loc8" TEXT,
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "locales_page_text" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "locales_page_text" ADD COLUMN IF NOT EXISTS "Text_loc1" TEXT;
ALTER TABLE IF EXISTS "locales_page_text" ADD COLUMN IF NOT EXISTS "Text_loc2" TEXT;
ALTER TABLE IF EXISTS "locales_page_text" ADD COLUMN IF NOT EXISTS "Text_loc3" TEXT;
ALTER TABLE IF EXISTS "locales_page_text" ADD COLUMN IF NOT EXISTS "Text_loc4" TEXT;
ALTER TABLE IF EXISTS "locales_page_text" ADD COLUMN IF NOT EXISTS "Text_loc5" TEXT;
ALTER TABLE IF EXISTS "locales_page_text" ADD COLUMN IF NOT EXISTS "Text_loc6" TEXT;
ALTER TABLE IF EXISTS "locales_page_text" ADD COLUMN IF NOT EXISTS "Text_loc7" TEXT;
ALTER TABLE IF EXISTS "locales_page_text" ADD COLUMN IF NOT EXISTS "Text_loc8" TEXT;

CREATE TABLE IF NOT EXISTS "locales_points_of_interest" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "icon_name_loc1" TEXT,
    "icon_name_loc2" TEXT,
    "icon_name_loc3" TEXT,
    "icon_name_loc4" TEXT,
    "icon_name_loc5" TEXT,
    "icon_name_loc6" TEXT,
    "icon_name_loc7" TEXT,
    "icon_name_loc8" TEXT,
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "locales_points_of_interest" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "locales_points_of_interest" ADD COLUMN IF NOT EXISTS "icon_name_loc1" TEXT;
ALTER TABLE IF EXISTS "locales_points_of_interest" ADD COLUMN IF NOT EXISTS "icon_name_loc2" TEXT;
ALTER TABLE IF EXISTS "locales_points_of_interest" ADD COLUMN IF NOT EXISTS "icon_name_loc3" TEXT;
ALTER TABLE IF EXISTS "locales_points_of_interest" ADD COLUMN IF NOT EXISTS "icon_name_loc4" TEXT;
ALTER TABLE IF EXISTS "locales_points_of_interest" ADD COLUMN IF NOT EXISTS "icon_name_loc5" TEXT;
ALTER TABLE IF EXISTS "locales_points_of_interest" ADD COLUMN IF NOT EXISTS "icon_name_loc6" TEXT;
ALTER TABLE IF EXISTS "locales_points_of_interest" ADD COLUMN IF NOT EXISTS "icon_name_loc7" TEXT;
ALTER TABLE IF EXISTS "locales_points_of_interest" ADD COLUMN IF NOT EXISTS "icon_name_loc8" TEXT;

CREATE TABLE IF NOT EXISTS "locales_quest" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "Title_loc1" TEXT,
    "Title_loc2" TEXT,
    "Title_loc3" TEXT,
    "Title_loc4" TEXT,
    "Title_loc5" TEXT,
    "Title_loc6" TEXT,
    "Title_loc7" TEXT,
    "Title_loc8" TEXT,
    "Details_loc1" TEXT,
    "Details_loc2" TEXT,
    "Details_loc3" TEXT,
    "Details_loc4" TEXT,
    "Details_loc5" TEXT,
    "Details_loc6" TEXT,
    "Details_loc7" TEXT,
    "Details_loc8" TEXT,
    "Objectives_loc1" TEXT,
    "Objectives_loc2" TEXT,
    "Objectives_loc3" TEXT,
    "Objectives_loc4" TEXT,
    "Objectives_loc5" TEXT,
    "Objectives_loc6" TEXT,
    "Objectives_loc7" TEXT,
    "Objectives_loc8" TEXT,
    "OfferRewardText_loc1" TEXT,
    "OfferRewardText_loc2" TEXT,
    "OfferRewardText_loc3" TEXT,
    "OfferRewardText_loc4" TEXT,
    "OfferRewardText_loc5" TEXT,
    "OfferRewardText_loc6" TEXT,
    "OfferRewardText_loc7" TEXT,
    "OfferRewardText_loc8" TEXT,
    "RequestItemsText_loc1" TEXT,
    "RequestItemsText_loc2" TEXT,
    "RequestItemsText_loc3" TEXT,
    "RequestItemsText_loc4" TEXT,
    "RequestItemsText_loc5" TEXT,
    "RequestItemsText_loc6" TEXT,
    "RequestItemsText_loc7" TEXT,
    "RequestItemsText_loc8" TEXT,
    "EndText_loc1" TEXT,
    "EndText_loc2" TEXT,
    "EndText_loc3" TEXT,
    "EndText_loc4" TEXT,
    "EndText_loc5" TEXT,
    "EndText_loc6" TEXT,
    "EndText_loc7" TEXT,
    "EndText_loc8" TEXT,
    "ObjectiveText1_loc1" TEXT,
    "ObjectiveText1_loc2" TEXT,
    "ObjectiveText1_loc3" TEXT,
    "ObjectiveText1_loc4" TEXT,
    "ObjectiveText1_loc5" TEXT,
    "ObjectiveText1_loc6" TEXT,
    "ObjectiveText1_loc7" TEXT,
    "ObjectiveText1_loc8" TEXT,
    "ObjectiveText2_loc1" TEXT,
    "ObjectiveText2_loc2" TEXT,
    "ObjectiveText2_loc3" TEXT,
    "ObjectiveText2_loc4" TEXT,
    "ObjectiveText2_loc5" TEXT,
    "ObjectiveText2_loc6" TEXT,
    "ObjectiveText2_loc7" TEXT,
    "ObjectiveText2_loc8" TEXT,
    "ObjectiveText3_loc1" TEXT,
    "ObjectiveText3_loc2" TEXT,
    "ObjectiveText3_loc3" TEXT,
    "ObjectiveText3_loc4" TEXT,
    "ObjectiveText3_loc5" TEXT,
    "ObjectiveText3_loc6" TEXT,
    "ObjectiveText3_loc7" TEXT,
    "ObjectiveText3_loc8" TEXT,
    "ObjectiveText4_loc1" TEXT,
    "ObjectiveText4_loc2" TEXT,
    "ObjectiveText4_loc3" TEXT,
    "ObjectiveText4_loc4" TEXT,
    "ObjectiveText4_loc5" TEXT,
    "ObjectiveText4_loc6" TEXT,
    "ObjectiveText4_loc7" TEXT,
    "ObjectiveText4_loc8" TEXT,
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "Title_loc1" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "Title_loc2" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "Title_loc3" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "Title_loc4" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "Title_loc5" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "Title_loc6" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "Title_loc7" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "Title_loc8" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "Details_loc1" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "Details_loc2" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "Details_loc3" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "Details_loc4" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "Details_loc5" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "Details_loc6" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "Details_loc7" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "Details_loc8" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "Objectives_loc1" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "Objectives_loc2" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "Objectives_loc3" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "Objectives_loc4" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "Objectives_loc5" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "Objectives_loc6" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "Objectives_loc7" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "Objectives_loc8" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "OfferRewardText_loc1" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "OfferRewardText_loc2" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "OfferRewardText_loc3" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "OfferRewardText_loc4" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "OfferRewardText_loc5" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "OfferRewardText_loc6" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "OfferRewardText_loc7" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "OfferRewardText_loc8" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "RequestItemsText_loc1" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "RequestItemsText_loc2" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "RequestItemsText_loc3" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "RequestItemsText_loc4" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "RequestItemsText_loc5" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "RequestItemsText_loc6" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "RequestItemsText_loc7" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "RequestItemsText_loc8" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "EndText_loc1" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "EndText_loc2" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "EndText_loc3" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "EndText_loc4" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "EndText_loc5" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "EndText_loc6" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "EndText_loc7" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "EndText_loc8" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText1_loc1" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText1_loc2" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText1_loc3" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText1_loc4" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText1_loc5" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText1_loc6" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText1_loc7" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText1_loc8" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText2_loc1" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText2_loc2" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText2_loc3" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText2_loc4" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText2_loc5" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText2_loc6" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText2_loc7" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText2_loc8" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText3_loc1" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText3_loc2" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText3_loc3" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText3_loc4" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText3_loc5" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText3_loc6" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText3_loc7" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText3_loc8" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText4_loc1" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText4_loc2" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText4_loc3" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText4_loc4" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText4_loc5" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText4_loc6" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText4_loc7" TEXT;
ALTER TABLE IF EXISTS "locales_quest" ADD COLUMN IF NOT EXISTS "ObjectiveText4_loc8" TEXT;

CREATE TABLE IF NOT EXISTS "locales_spell" (
    "entry" INTEGER NOT NULL CHECK ("entry" >= 0),
    "name_loc1" VARCHAR(256) NOT NULL DEFAULT '',
    "name_loc2" VARCHAR(256) NOT NULL DEFAULT '',
    "name_loc3" VARCHAR(256) NOT NULL DEFAULT '',
    "name_loc4" VARCHAR(256) NOT NULL DEFAULT '',
    "name_loc5" VARCHAR(256) NOT NULL DEFAULT '',
    "name_loc6" VARCHAR(256) NOT NULL DEFAULT '',
    "nameSubtext_loc1" VARCHAR(256) NOT NULL DEFAULT '',
    "nameSubtext_loc2" VARCHAR(256) NOT NULL DEFAULT '',
    "nameSubtext_loc3" VARCHAR(256) NOT NULL DEFAULT '',
    "nameSubtext_loc4" VARCHAR(256) NOT NULL DEFAULT '',
    "nameSubtext_loc5" VARCHAR(256) NOT NULL DEFAULT '',
    "nameSubtext_loc6" VARCHAR(256) NOT NULL DEFAULT '',
    "description_loc1" VARCHAR(1024) NOT NULL DEFAULT '',
    "description_loc2" VARCHAR(1024) NOT NULL DEFAULT '',
    "description_loc3" VARCHAR(1024) NOT NULL DEFAULT '',
    "description_loc4" VARCHAR(1024) NOT NULL DEFAULT '',
    "description_loc5" VARCHAR(1024) NOT NULL DEFAULT '',
    "description_loc6" VARCHAR(1024) NOT NULL DEFAULT '',
    "auraDescription_loc1" VARCHAR(512) NOT NULL DEFAULT '',
    "auraDescription_loc2" VARCHAR(512) NOT NULL DEFAULT '',
    "auraDescription_loc3" VARCHAR(512) NOT NULL DEFAULT '',
    "auraDescription_loc4" VARCHAR(512) NOT NULL DEFAULT '',
    "auraDescription_loc5" VARCHAR(512) NOT NULL DEFAULT '',
    "auraDescription_loc6" VARCHAR(512) NOT NULL DEFAULT '',
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "locales_spell" ADD COLUMN IF NOT EXISTS "entry" INTEGER NOT NULL CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "locales_spell" ADD COLUMN IF NOT EXISTS "name_loc1" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_spell" ADD COLUMN IF NOT EXISTS "name_loc2" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_spell" ADD COLUMN IF NOT EXISTS "name_loc3" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_spell" ADD COLUMN IF NOT EXISTS "name_loc4" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_spell" ADD COLUMN IF NOT EXISTS "name_loc5" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_spell" ADD COLUMN IF NOT EXISTS "name_loc6" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_spell" ADD COLUMN IF NOT EXISTS "nameSubtext_loc1" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_spell" ADD COLUMN IF NOT EXISTS "nameSubtext_loc2" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_spell" ADD COLUMN IF NOT EXISTS "nameSubtext_loc3" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_spell" ADD COLUMN IF NOT EXISTS "nameSubtext_loc4" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_spell" ADD COLUMN IF NOT EXISTS "nameSubtext_loc5" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_spell" ADD COLUMN IF NOT EXISTS "nameSubtext_loc6" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_spell" ADD COLUMN IF NOT EXISTS "description_loc1" VARCHAR(1024) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_spell" ADD COLUMN IF NOT EXISTS "description_loc2" VARCHAR(1024) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_spell" ADD COLUMN IF NOT EXISTS "description_loc3" VARCHAR(1024) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_spell" ADD COLUMN IF NOT EXISTS "description_loc4" VARCHAR(1024) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_spell" ADD COLUMN IF NOT EXISTS "description_loc5" VARCHAR(1024) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_spell" ADD COLUMN IF NOT EXISTS "description_loc6" VARCHAR(1024) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_spell" ADD COLUMN IF NOT EXISTS "auraDescription_loc1" VARCHAR(512) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_spell" ADD COLUMN IF NOT EXISTS "auraDescription_loc2" VARCHAR(512) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_spell" ADD COLUMN IF NOT EXISTS "auraDescription_loc3" VARCHAR(512) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_spell" ADD COLUMN IF NOT EXISTS "auraDescription_loc4" VARCHAR(512) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_spell" ADD COLUMN IF NOT EXISTS "auraDescription_loc5" VARCHAR(512) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_spell" ADD COLUMN IF NOT EXISTS "auraDescription_loc6" VARCHAR(512) NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS "locales_taxi_node" (
    "entry" BIGINT NOT NULL CHECK ("entry" >= 0),
    "name_loc1" VARCHAR(256) NOT NULL DEFAULT '',
    "name_loc2" VARCHAR(256) NOT NULL DEFAULT '',
    "name_loc3" VARCHAR(256) NOT NULL DEFAULT '',
    "name_loc4" VARCHAR(256) NOT NULL DEFAULT '',
    "name_loc5" VARCHAR(256) NOT NULL DEFAULT '',
    "name_loc6" VARCHAR(256) NOT NULL DEFAULT '',
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "locales_taxi_node" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "locales_taxi_node" ADD COLUMN IF NOT EXISTS "name_loc1" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_taxi_node" ADD COLUMN IF NOT EXISTS "name_loc2" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_taxi_node" ADD COLUMN IF NOT EXISTS "name_loc3" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_taxi_node" ADD COLUMN IF NOT EXISTS "name_loc4" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_taxi_node" ADD COLUMN IF NOT EXISTS "name_loc5" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "locales_taxi_node" ADD COLUMN IF NOT EXISTS "name_loc6" VARCHAR(256) NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS "mail_loot_template" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "item" BIGINT NOT NULL DEFAULT '0' CHECK ("item" >= 0),
    "ChanceOrQuestChance" REAL NOT NULL DEFAULT '100',
    "groupid" SMALLINT NOT NULL DEFAULT '0' CHECK ("groupid" >= 0),
    "mincountOrRef" INTEGER NOT NULL DEFAULT '1',
    "maxcount" SMALLINT NOT NULL DEFAULT '1' CHECK ("maxcount" >= 0),
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0),
    PRIMARY KEY ("entry", "item")
);
ALTER TABLE IF EXISTS "mail_loot_template" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "mail_loot_template" ADD COLUMN IF NOT EXISTS "item" BIGINT NOT NULL DEFAULT '0' CHECK ("item" >= 0);
ALTER TABLE IF EXISTS "mail_loot_template" ADD COLUMN IF NOT EXISTS "ChanceOrQuestChance" REAL NOT NULL DEFAULT '100';
ALTER TABLE IF EXISTS "mail_loot_template" ADD COLUMN IF NOT EXISTS "groupid" SMALLINT NOT NULL DEFAULT '0' CHECK ("groupid" >= 0);
ALTER TABLE IF EXISTS "mail_loot_template" ADD COLUMN IF NOT EXISTS "mincountOrRef" INTEGER NOT NULL DEFAULT '1';
ALTER TABLE IF EXISTS "mail_loot_template" ADD COLUMN IF NOT EXISTS "maxcount" SMALLINT NOT NULL DEFAULT '1' CHECK ("maxcount" >= 0);
ALTER TABLE IF EXISTS "mail_loot_template" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);
ALTER TABLE IF EXISTS "mail_loot_template" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);
ALTER TABLE IF EXISTS "mail_loot_template" ADD COLUMN IF NOT EXISTS "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0);

CREATE TABLE IF NOT EXISTS "mail_text_template" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "content_default" TEXT NOT NULL,
    "content_loc1" TEXT,
    "content_loc2" TEXT,
    "content_loc3" TEXT,
    "content_loc4" TEXT,
    "content_loc5" TEXT,
    "content_loc6" TEXT,
    "content_loc7" TEXT,
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "mail_text_template" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "mail_text_template" ADD COLUMN IF NOT EXISTS "content_default" TEXT NOT NULL;
ALTER TABLE IF EXISTS "mail_text_template" ADD COLUMN IF NOT EXISTS "content_loc1" TEXT;
ALTER TABLE IF EXISTS "mail_text_template" ADD COLUMN IF NOT EXISTS "content_loc2" TEXT;
ALTER TABLE IF EXISTS "mail_text_template" ADD COLUMN IF NOT EXISTS "content_loc3" TEXT;
ALTER TABLE IF EXISTS "mail_text_template" ADD COLUMN IF NOT EXISTS "content_loc4" TEXT;
ALTER TABLE IF EXISTS "mail_text_template" ADD COLUMN IF NOT EXISTS "content_loc5" TEXT;
ALTER TABLE IF EXISTS "mail_text_template" ADD COLUMN IF NOT EXISTS "content_loc6" TEXT;
ALTER TABLE IF EXISTS "mail_text_template" ADD COLUMN IF NOT EXISTS "content_loc7" TEXT;

CREATE TABLE IF NOT EXISTS "mangos_string" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "content_default" TEXT NOT NULL,
    "content_loc1" TEXT,
    "content_loc2" TEXT,
    "content_loc3" TEXT,
    "content_loc4" TEXT,
    "content_loc5" TEXT,
    "content_loc6" TEXT,
    "content_loc7" TEXT,
    "content_loc8" TEXT,
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "mangos_string" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "mangos_string" ADD COLUMN IF NOT EXISTS "content_default" TEXT NOT NULL;
ALTER TABLE IF EXISTS "mangos_string" ADD COLUMN IF NOT EXISTS "content_loc1" TEXT;
ALTER TABLE IF EXISTS "mangos_string" ADD COLUMN IF NOT EXISTS "content_loc2" TEXT;
ALTER TABLE IF EXISTS "mangos_string" ADD COLUMN IF NOT EXISTS "content_loc3" TEXT;
ALTER TABLE IF EXISTS "mangos_string" ADD COLUMN IF NOT EXISTS "content_loc4" TEXT;
ALTER TABLE IF EXISTS "mangos_string" ADD COLUMN IF NOT EXISTS "content_loc5" TEXT;
ALTER TABLE IF EXISTS "mangos_string" ADD COLUMN IF NOT EXISTS "content_loc6" TEXT;
ALTER TABLE IF EXISTS "mangos_string" ADD COLUMN IF NOT EXISTS "content_loc7" TEXT;
ALTER TABLE IF EXISTS "mangos_string" ADD COLUMN IF NOT EXISTS "content_loc8" TEXT;

CREATE TABLE IF NOT EXISTS "map_loot_disabled" (
    "map_id" INTEGER NOT NULL DEFAULT '0',
    "comment" VARCHAR(255) DEFAULT NULL,
    PRIMARY KEY ("map_id")
);
ALTER TABLE IF EXISTS "map_loot_disabled" ADD COLUMN IF NOT EXISTS "map_id" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "map_loot_disabled" ADD COLUMN IF NOT EXISTS "comment" VARCHAR(255) DEFAULT NULL;

CREATE TABLE IF NOT EXISTS "map_template" (
    "entry" INTEGER NOT NULL CHECK ("entry" >= 0),
    "patch" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch" >= 0),
    "parent" BIGINT NOT NULL DEFAULT '0' CHECK ("parent" >= 0),
    "map_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("map_type" >= 0),
    "linked_zone" BIGINT NOT NULL DEFAULT '0' CHECK ("linked_zone" >= 0),
    "player_limit" SMALLINT NOT NULL DEFAULT '0' CHECK ("player_limit" >= 0),
    "reset_delay" BIGINT NOT NULL DEFAULT '0' CHECK ("reset_delay" >= 0),
    "ghost_entrance_map" SMALLINT NOT NULL DEFAULT '-1',
    "ghost_entrance_x" REAL NOT NULL DEFAULT '0',
    "ghost_entrance_y" REAL NOT NULL DEFAULT '0',
    "map_name" VARCHAR(128) NOT NULL DEFAULT '',
    "script_name" VARCHAR(128) NOT NULL DEFAULT '',
    PRIMARY KEY ("entry", "patch")
);
ALTER TABLE IF EXISTS "map_template" ADD COLUMN IF NOT EXISTS "entry" INTEGER NOT NULL CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "map_template" ADD COLUMN IF NOT EXISTS "patch" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch" >= 0);
ALTER TABLE IF EXISTS "map_template" ADD COLUMN IF NOT EXISTS "parent" BIGINT NOT NULL DEFAULT '0' CHECK ("parent" >= 0);
ALTER TABLE IF EXISTS "map_template" ADD COLUMN IF NOT EXISTS "map_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("map_type" >= 0);
ALTER TABLE IF EXISTS "map_template" ADD COLUMN IF NOT EXISTS "linked_zone" BIGINT NOT NULL DEFAULT '0' CHECK ("linked_zone" >= 0);
ALTER TABLE IF EXISTS "map_template" ADD COLUMN IF NOT EXISTS "player_limit" SMALLINT NOT NULL DEFAULT '0' CHECK ("player_limit" >= 0);
ALTER TABLE IF EXISTS "map_template" ADD COLUMN IF NOT EXISTS "reset_delay" BIGINT NOT NULL DEFAULT '0' CHECK ("reset_delay" >= 0);
ALTER TABLE IF EXISTS "map_template" ADD COLUMN IF NOT EXISTS "ghost_entrance_map" SMALLINT NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "map_template" ADD COLUMN IF NOT EXISTS "ghost_entrance_x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "map_template" ADD COLUMN IF NOT EXISTS "ghost_entrance_y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "map_template" ADD COLUMN IF NOT EXISTS "map_name" VARCHAR(128) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "map_template" ADD COLUMN IF NOT EXISTS "script_name" VARCHAR(128) NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS "npc_gossip" (
    "npc_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("npc_guid" >= 0),
    "textid" BIGINT NOT NULL DEFAULT '0' CHECK ("textid" >= 0),
    PRIMARY KEY ("npc_guid")
);
ALTER TABLE IF EXISTS "npc_gossip" ADD COLUMN IF NOT EXISTS "npc_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("npc_guid" >= 0);
ALTER TABLE IF EXISTS "npc_gossip" ADD COLUMN IF NOT EXISTS "textid" BIGINT NOT NULL DEFAULT '0' CHECK ("textid" >= 0);

CREATE TABLE IF NOT EXISTS "npc_text" (
    "ID" BIGINT NOT NULL DEFAULT '0' CHECK ("ID" >= 0),
    "BroadcastTextID0" BIGINT NOT NULL DEFAULT '0' CHECK ("BroadcastTextID0" >= 0),
    "Probability0" REAL NOT NULL DEFAULT '0',
    "BroadcastTextID1" BIGINT NOT NULL DEFAULT '0' CHECK ("BroadcastTextID1" >= 0),
    "Probability1" REAL NOT NULL DEFAULT '0',
    "BroadcastTextID2" BIGINT NOT NULL DEFAULT '0' CHECK ("BroadcastTextID2" >= 0),
    "Probability2" REAL NOT NULL DEFAULT '0',
    "BroadcastTextID3" BIGINT NOT NULL DEFAULT '0' CHECK ("BroadcastTextID3" >= 0),
    "Probability3" REAL NOT NULL DEFAULT '0',
    "BroadcastTextID4" BIGINT NOT NULL DEFAULT '0' CHECK ("BroadcastTextID4" >= 0),
    "Probability4" REAL NOT NULL DEFAULT '0',
    "BroadcastTextID5" BIGINT NOT NULL DEFAULT '0' CHECK ("BroadcastTextID5" >= 0),
    "Probability5" REAL NOT NULL DEFAULT '0',
    "BroadcastTextID6" BIGINT NOT NULL DEFAULT '0' CHECK ("BroadcastTextID6" >= 0),
    "Probability6" REAL NOT NULL DEFAULT '0',
    "BroadcastTextID7" BIGINT NOT NULL DEFAULT '0' CHECK ("BroadcastTextID7" >= 0),
    "Probability7" REAL NOT NULL DEFAULT '0',
    PRIMARY KEY ("ID")
);
ALTER TABLE IF EXISTS "npc_text" ADD COLUMN IF NOT EXISTS "ID" BIGINT NOT NULL DEFAULT '0' CHECK ("ID" >= 0);
ALTER TABLE IF EXISTS "npc_text" ADD COLUMN IF NOT EXISTS "BroadcastTextID0" BIGINT NOT NULL DEFAULT '0' CHECK ("BroadcastTextID0" >= 0);
ALTER TABLE IF EXISTS "npc_text" ADD COLUMN IF NOT EXISTS "Probability0" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "npc_text" ADD COLUMN IF NOT EXISTS "BroadcastTextID1" BIGINT NOT NULL DEFAULT '0' CHECK ("BroadcastTextID1" >= 0);
ALTER TABLE IF EXISTS "npc_text" ADD COLUMN IF NOT EXISTS "Probability1" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "npc_text" ADD COLUMN IF NOT EXISTS "BroadcastTextID2" BIGINT NOT NULL DEFAULT '0' CHECK ("BroadcastTextID2" >= 0);
ALTER TABLE IF EXISTS "npc_text" ADD COLUMN IF NOT EXISTS "Probability2" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "npc_text" ADD COLUMN IF NOT EXISTS "BroadcastTextID3" BIGINT NOT NULL DEFAULT '0' CHECK ("BroadcastTextID3" >= 0);
ALTER TABLE IF EXISTS "npc_text" ADD COLUMN IF NOT EXISTS "Probability3" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "npc_text" ADD COLUMN IF NOT EXISTS "BroadcastTextID4" BIGINT NOT NULL DEFAULT '0' CHECK ("BroadcastTextID4" >= 0);
ALTER TABLE IF EXISTS "npc_text" ADD COLUMN IF NOT EXISTS "Probability4" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "npc_text" ADD COLUMN IF NOT EXISTS "BroadcastTextID5" BIGINT NOT NULL DEFAULT '0' CHECK ("BroadcastTextID5" >= 0);
ALTER TABLE IF EXISTS "npc_text" ADD COLUMN IF NOT EXISTS "Probability5" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "npc_text" ADD COLUMN IF NOT EXISTS "BroadcastTextID6" BIGINT NOT NULL DEFAULT '0' CHECK ("BroadcastTextID6" >= 0);
ALTER TABLE IF EXISTS "npc_text" ADD COLUMN IF NOT EXISTS "Probability6" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "npc_text" ADD COLUMN IF NOT EXISTS "BroadcastTextID7" BIGINT NOT NULL DEFAULT '0' CHECK ("BroadcastTextID7" >= 0);
ALTER TABLE IF EXISTS "npc_text" ADD COLUMN IF NOT EXISTS "Probability7" REAL NOT NULL DEFAULT '0';

CREATE TABLE IF NOT EXISTS "npc_trainer" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "spell" INTEGER NOT NULL DEFAULT '0' CHECK ("spell" >= 0),
    "spellcost" BIGINT NOT NULL DEFAULT '0' CHECK ("spellcost" >= 0),
    "reqskill" INTEGER NOT NULL DEFAULT '0' CHECK ("reqskill" >= 0),
    "reqskillvalue" INTEGER NOT NULL DEFAULT '0' CHECK ("reqskillvalue" >= 0),
    "reqlevel" SMALLINT NOT NULL DEFAULT '0' CHECK ("reqlevel" >= 0),
    "build_min" INTEGER NOT NULL DEFAULT '0' CHECK ("build_min" >= 0),
    "build_max" INTEGER NOT NULL DEFAULT '5875' CHECK ("build_max" >= 0),
    UNIQUE ("entry", "spell", "build_max")
);
ALTER TABLE IF EXISTS "npc_trainer" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "npc_trainer" ADD COLUMN IF NOT EXISTS "spell" INTEGER NOT NULL DEFAULT '0' CHECK ("spell" >= 0);
ALTER TABLE IF EXISTS "npc_trainer" ADD COLUMN IF NOT EXISTS "spellcost" BIGINT NOT NULL DEFAULT '0' CHECK ("spellcost" >= 0);
ALTER TABLE IF EXISTS "npc_trainer" ADD COLUMN IF NOT EXISTS "reqskill" INTEGER NOT NULL DEFAULT '0' CHECK ("reqskill" >= 0);
ALTER TABLE IF EXISTS "npc_trainer" ADD COLUMN IF NOT EXISTS "reqskillvalue" INTEGER NOT NULL DEFAULT '0' CHECK ("reqskillvalue" >= 0);
ALTER TABLE IF EXISTS "npc_trainer" ADD COLUMN IF NOT EXISTS "reqlevel" SMALLINT NOT NULL DEFAULT '0' CHECK ("reqlevel" >= 0);
ALTER TABLE IF EXISTS "npc_trainer" ADD COLUMN IF NOT EXISTS "build_min" INTEGER NOT NULL DEFAULT '0' CHECK ("build_min" >= 0);
ALTER TABLE IF EXISTS "npc_trainer" ADD COLUMN IF NOT EXISTS "build_max" INTEGER NOT NULL DEFAULT '5875' CHECK ("build_max" >= 0);
CREATE UNIQUE INDEX IF NOT EXISTS idx_npc_trainer_entry_spell ON "npc_trainer" ("entry", "spell", "build_max");

CREATE TABLE IF NOT EXISTS "npc_trainer_greeting" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "content_default" TEXT NOT NULL,
    "content_loc1" TEXT NOT NULL,
    "content_loc2" TEXT NOT NULL,
    "content_loc3" TEXT NOT NULL,
    "content_loc4" TEXT NOT NULL,
    "content_loc5" TEXT NOT NULL,
    "content_loc6" TEXT NOT NULL,
    "content_loc7" TEXT NOT NULL,
    "content_loc8" TEXT NOT NULL,
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "npc_trainer_greeting" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "npc_trainer_greeting" ADD COLUMN IF NOT EXISTS "content_default" TEXT NOT NULL;
ALTER TABLE IF EXISTS "npc_trainer_greeting" ADD COLUMN IF NOT EXISTS "content_loc1" TEXT NOT NULL;
ALTER TABLE IF EXISTS "npc_trainer_greeting" ADD COLUMN IF NOT EXISTS "content_loc2" TEXT NOT NULL;
ALTER TABLE IF EXISTS "npc_trainer_greeting" ADD COLUMN IF NOT EXISTS "content_loc3" TEXT NOT NULL;
ALTER TABLE IF EXISTS "npc_trainer_greeting" ADD COLUMN IF NOT EXISTS "content_loc4" TEXT NOT NULL;
ALTER TABLE IF EXISTS "npc_trainer_greeting" ADD COLUMN IF NOT EXISTS "content_loc5" TEXT NOT NULL;
ALTER TABLE IF EXISTS "npc_trainer_greeting" ADD COLUMN IF NOT EXISTS "content_loc6" TEXT NOT NULL;
ALTER TABLE IF EXISTS "npc_trainer_greeting" ADD COLUMN IF NOT EXISTS "content_loc7" TEXT NOT NULL;
ALTER TABLE IF EXISTS "npc_trainer_greeting" ADD COLUMN IF NOT EXISTS "content_loc8" TEXT NOT NULL;

CREATE TABLE IF NOT EXISTS "npc_trainer_template" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "spell" INTEGER NOT NULL DEFAULT '0' CHECK ("spell" >= 0),
    "spellcost" BIGINT NOT NULL DEFAULT '0' CHECK ("spellcost" >= 0),
    "reqskill" INTEGER NOT NULL DEFAULT '0' CHECK ("reqskill" >= 0),
    "reqskillvalue" INTEGER NOT NULL DEFAULT '0' CHECK ("reqskillvalue" >= 0),
    "reqlevel" SMALLINT NOT NULL DEFAULT '0' CHECK ("reqlevel" >= 0),
    "build_min" INTEGER NOT NULL DEFAULT '0' CHECK ("build_min" >= 0),
    "build_max" INTEGER NOT NULL DEFAULT '5875' CHECK ("build_max" >= 0),
    UNIQUE ("entry", "spell", "build_max")
);
ALTER TABLE IF EXISTS "npc_trainer_template" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "npc_trainer_template" ADD COLUMN IF NOT EXISTS "spell" INTEGER NOT NULL DEFAULT '0' CHECK ("spell" >= 0);
ALTER TABLE IF EXISTS "npc_trainer_template" ADD COLUMN IF NOT EXISTS "spellcost" BIGINT NOT NULL DEFAULT '0' CHECK ("spellcost" >= 0);
ALTER TABLE IF EXISTS "npc_trainer_template" ADD COLUMN IF NOT EXISTS "reqskill" INTEGER NOT NULL DEFAULT '0' CHECK ("reqskill" >= 0);
ALTER TABLE IF EXISTS "npc_trainer_template" ADD COLUMN IF NOT EXISTS "reqskillvalue" INTEGER NOT NULL DEFAULT '0' CHECK ("reqskillvalue" >= 0);
ALTER TABLE IF EXISTS "npc_trainer_template" ADD COLUMN IF NOT EXISTS "reqlevel" SMALLINT NOT NULL DEFAULT '0' CHECK ("reqlevel" >= 0);
ALTER TABLE IF EXISTS "npc_trainer_template" ADD COLUMN IF NOT EXISTS "build_min" INTEGER NOT NULL DEFAULT '0' CHECK ("build_min" >= 0);
ALTER TABLE IF EXISTS "npc_trainer_template" ADD COLUMN IF NOT EXISTS "build_max" INTEGER NOT NULL DEFAULT '5875' CHECK ("build_max" >= 0);
CREATE UNIQUE INDEX IF NOT EXISTS idx_npc_trainer_template_entry_spell ON "npc_trainer_template" ("entry", "spell", "build_max");

CREATE TABLE IF NOT EXISTS "npc_vendor" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "slot" INTEGER NOT NULL DEFAULT '0' CHECK ("slot" >= 0),
    "item" BIGINT NOT NULL DEFAULT '0' CHECK ("item" >= 0),
    "maxcount" SMALLINT NOT NULL DEFAULT '0' CHECK ("maxcount" >= 0),
    "incrtime" BIGINT NOT NULL DEFAULT '0' CHECK ("incrtime" >= 0),
    "itemflags" BIGINT NOT NULL DEFAULT '0' CHECK ("itemflags" >= 0),
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    PRIMARY KEY ("entry", "item")
);
ALTER TABLE IF EXISTS "npc_vendor" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "npc_vendor" ADD COLUMN IF NOT EXISTS "slot" INTEGER NOT NULL DEFAULT '0' CHECK ("slot" >= 0);
ALTER TABLE IF EXISTS "npc_vendor" ADD COLUMN IF NOT EXISTS "item" BIGINT NOT NULL DEFAULT '0' CHECK ("item" >= 0);
ALTER TABLE IF EXISTS "npc_vendor" ADD COLUMN IF NOT EXISTS "maxcount" SMALLINT NOT NULL DEFAULT '0' CHECK ("maxcount" >= 0);
ALTER TABLE IF EXISTS "npc_vendor" ADD COLUMN IF NOT EXISTS "incrtime" BIGINT NOT NULL DEFAULT '0' CHECK ("incrtime" >= 0);
ALTER TABLE IF EXISTS "npc_vendor" ADD COLUMN IF NOT EXISTS "itemflags" BIGINT NOT NULL DEFAULT '0' CHECK ("itemflags" >= 0);
ALTER TABLE IF EXISTS "npc_vendor" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);

CREATE TABLE IF NOT EXISTS "npc_vendor_template" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "slot" INTEGER NOT NULL DEFAULT '0' CHECK ("slot" >= 0),
    "item" BIGINT NOT NULL DEFAULT '0' CHECK ("item" >= 0),
    "maxcount" SMALLINT NOT NULL DEFAULT '0' CHECK ("maxcount" >= 0),
    "incrtime" BIGINT NOT NULL DEFAULT '0' CHECK ("incrtime" >= 0),
    "itemflags" BIGINT NOT NULL DEFAULT '0' CHECK ("itemflags" >= 0),
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    PRIMARY KEY ("entry", "item")
);
ALTER TABLE IF EXISTS "npc_vendor_template" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "npc_vendor_template" ADD COLUMN IF NOT EXISTS "slot" INTEGER NOT NULL DEFAULT '0' CHECK ("slot" >= 0);
ALTER TABLE IF EXISTS "npc_vendor_template" ADD COLUMN IF NOT EXISTS "item" BIGINT NOT NULL DEFAULT '0' CHECK ("item" >= 0);
ALTER TABLE IF EXISTS "npc_vendor_template" ADD COLUMN IF NOT EXISTS "maxcount" SMALLINT NOT NULL DEFAULT '0' CHECK ("maxcount" >= 0);
ALTER TABLE IF EXISTS "npc_vendor_template" ADD COLUMN IF NOT EXISTS "incrtime" BIGINT NOT NULL DEFAULT '0' CHECK ("incrtime" >= 0);
ALTER TABLE IF EXISTS "npc_vendor_template" ADD COLUMN IF NOT EXISTS "itemflags" BIGINT NOT NULL DEFAULT '0' CHECK ("itemflags" >= 0);
ALTER TABLE IF EXISTS "npc_vendor_template" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);

CREATE TABLE IF NOT EXISTS "page_text" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "text" TEXT NOT NULL,
    "next_page" BIGINT NOT NULL DEFAULT '0' CHECK ("next_page" >= 0),
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "page_text" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "page_text" ADD COLUMN IF NOT EXISTS "text" TEXT NOT NULL;
ALTER TABLE IF EXISTS "page_text" ADD COLUMN IF NOT EXISTS "next_page" BIGINT NOT NULL DEFAULT '0' CHECK ("next_page" >= 0);

CREATE TABLE IF NOT EXISTS "pet_levelstats" (
    "entry" BIGINT NOT NULL CHECK ("entry" >= 0),
    "level" SMALLINT NOT NULL CHECK ("level" >= 0),
    "health" INTEGER NOT NULL DEFAULT '0' CHECK ("health" >= 0),
    "mana" INTEGER NOT NULL DEFAULT '0' CHECK ("mana" >= 0),
    "armor" BIGINT NOT NULL DEFAULT '0' CHECK ("armor" >= 0),
    "dmg_min" REAL NOT NULL DEFAULT '0',
    "dmg_max" REAL NOT NULL DEFAULT '0',
    "strength" INTEGER NOT NULL DEFAULT '0' CHECK ("strength" >= 0),
    "agility" INTEGER NOT NULL DEFAULT '0' CHECK ("agility" >= 0),
    "stamina" INTEGER NOT NULL DEFAULT '0' CHECK ("stamina" >= 0),
    "intellect" INTEGER NOT NULL DEFAULT '0' CHECK ("intellect" >= 0),
    "spirit" INTEGER NOT NULL DEFAULT '0' CHECK ("spirit" >= 0),
    PRIMARY KEY ("entry", "level")
);
ALTER TABLE IF EXISTS "pet_levelstats" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "pet_levelstats" ADD COLUMN IF NOT EXISTS "level" SMALLINT NOT NULL CHECK ("level" >= 0);
ALTER TABLE IF EXISTS "pet_levelstats" ADD COLUMN IF NOT EXISTS "health" INTEGER NOT NULL DEFAULT '0' CHECK ("health" >= 0);
ALTER TABLE IF EXISTS "pet_levelstats" ADD COLUMN IF NOT EXISTS "mana" INTEGER NOT NULL DEFAULT '0' CHECK ("mana" >= 0);
ALTER TABLE IF EXISTS "pet_levelstats" ADD COLUMN IF NOT EXISTS "armor" BIGINT NOT NULL DEFAULT '0' CHECK ("armor" >= 0);
ALTER TABLE IF EXISTS "pet_levelstats" ADD COLUMN IF NOT EXISTS "dmg_min" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "pet_levelstats" ADD COLUMN IF NOT EXISTS "dmg_max" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "pet_levelstats" ADD COLUMN IF NOT EXISTS "strength" INTEGER NOT NULL DEFAULT '0' CHECK ("strength" >= 0);
ALTER TABLE IF EXISTS "pet_levelstats" ADD COLUMN IF NOT EXISTS "agility" INTEGER NOT NULL DEFAULT '0' CHECK ("agility" >= 0);
ALTER TABLE IF EXISTS "pet_levelstats" ADD COLUMN IF NOT EXISTS "stamina" INTEGER NOT NULL DEFAULT '0' CHECK ("stamina" >= 0);
ALTER TABLE IF EXISTS "pet_levelstats" ADD COLUMN IF NOT EXISTS "intellect" INTEGER NOT NULL DEFAULT '0' CHECK ("intellect" >= 0);
ALTER TABLE IF EXISTS "pet_levelstats" ADD COLUMN IF NOT EXISTS "spirit" INTEGER NOT NULL DEFAULT '0' CHECK ("spirit" >= 0);

CREATE TABLE IF NOT EXISTS "pet_name_generation" (
    "id" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("id" >= 0),
    "word" TEXT NOT NULL,
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "half" SMALLINT NOT NULL DEFAULT '0',
    PRIMARY KEY ("id")
);
ALTER TABLE IF EXISTS "pet_name_generation" ADD COLUMN IF NOT EXISTS "id" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "pet_name_generation" ADD COLUMN IF NOT EXISTS "word" TEXT NOT NULL;
ALTER TABLE IF EXISTS "pet_name_generation" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "pet_name_generation" ADD COLUMN IF NOT EXISTS "half" SMALLINT NOT NULL DEFAULT '0';

CREATE TABLE IF NOT EXISTS "pet_spell_data" (
    "entry" BIGINT NOT NULL CHECK ("entry" >= 0),
    "build" INTEGER NOT NULL DEFAULT '0' CHECK ("build" >= 0),
    "spell_id1" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_id1" >= 0),
    "spell_id2" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_id2" >= 0),
    "spell_id3" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_id3" >= 0),
    "spell_id4" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_id4" >= 0),
    PRIMARY KEY ("entry", "build")
);
ALTER TABLE IF EXISTS "pet_spell_data" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "pet_spell_data" ADD COLUMN IF NOT EXISTS "build" INTEGER NOT NULL DEFAULT '0' CHECK ("build" >= 0);
ALTER TABLE IF EXISTS "pet_spell_data" ADD COLUMN IF NOT EXISTS "spell_id1" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_id1" >= 0);
ALTER TABLE IF EXISTS "pet_spell_data" ADD COLUMN IF NOT EXISTS "spell_id2" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_id2" >= 0);
ALTER TABLE IF EXISTS "pet_spell_data" ADD COLUMN IF NOT EXISTS "spell_id3" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_id3" >= 0);
ALTER TABLE IF EXISTS "pet_spell_data" ADD COLUMN IF NOT EXISTS "spell_id4" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_id4" >= 0);

CREATE TABLE IF NOT EXISTS "petcreateinfo_spell" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "spell1" INTEGER NOT NULL DEFAULT '0' CHECK ("spell1" >= 0),
    "spell2" INTEGER NOT NULL DEFAULT '0' CHECK ("spell2" >= 0),
    "spell3" INTEGER NOT NULL DEFAULT '0' CHECK ("spell3" >= 0),
    "spell4" INTEGER NOT NULL DEFAULT '0' CHECK ("spell4" >= 0),
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0),
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "petcreateinfo_spell" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "petcreateinfo_spell" ADD COLUMN IF NOT EXISTS "spell1" INTEGER NOT NULL DEFAULT '0' CHECK ("spell1" >= 0);
ALTER TABLE IF EXISTS "petcreateinfo_spell" ADD COLUMN IF NOT EXISTS "spell2" INTEGER NOT NULL DEFAULT '0' CHECK ("spell2" >= 0);
ALTER TABLE IF EXISTS "petcreateinfo_spell" ADD COLUMN IF NOT EXISTS "spell3" INTEGER NOT NULL DEFAULT '0' CHECK ("spell3" >= 0);
ALTER TABLE IF EXISTS "petcreateinfo_spell" ADD COLUMN IF NOT EXISTS "spell4" INTEGER NOT NULL DEFAULT '0' CHECK ("spell4" >= 0);
ALTER TABLE IF EXISTS "petcreateinfo_spell" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);
ALTER TABLE IF EXISTS "petcreateinfo_spell" ADD COLUMN IF NOT EXISTS "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0);

CREATE TABLE IF NOT EXISTS "pickpocketing_loot_template" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "item" BIGINT NOT NULL DEFAULT '0' CHECK ("item" >= 0),
    "ChanceOrQuestChance" REAL NOT NULL DEFAULT '100',
    "groupid" SMALLINT NOT NULL DEFAULT '0' CHECK ("groupid" >= 0),
    "mincountOrRef" INTEGER NOT NULL DEFAULT '1',
    "maxcount" SMALLINT NOT NULL DEFAULT '1' CHECK ("maxcount" >= 0),
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0),
    PRIMARY KEY ("entry", "item")
);
ALTER TABLE IF EXISTS "pickpocketing_loot_template" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "pickpocketing_loot_template" ADD COLUMN IF NOT EXISTS "item" BIGINT NOT NULL DEFAULT '0' CHECK ("item" >= 0);
ALTER TABLE IF EXISTS "pickpocketing_loot_template" ADD COLUMN IF NOT EXISTS "ChanceOrQuestChance" REAL NOT NULL DEFAULT '100';
ALTER TABLE IF EXISTS "pickpocketing_loot_template" ADD COLUMN IF NOT EXISTS "groupid" SMALLINT NOT NULL DEFAULT '0' CHECK ("groupid" >= 0);
ALTER TABLE IF EXISTS "pickpocketing_loot_template" ADD COLUMN IF NOT EXISTS "mincountOrRef" INTEGER NOT NULL DEFAULT '1';
ALTER TABLE IF EXISTS "pickpocketing_loot_template" ADD COLUMN IF NOT EXISTS "maxcount" SMALLINT NOT NULL DEFAULT '1' CHECK ("maxcount" >= 0);
ALTER TABLE IF EXISTS "pickpocketing_loot_template" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);
ALTER TABLE IF EXISTS "pickpocketing_loot_template" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);
ALTER TABLE IF EXISTS "pickpocketing_loot_template" ADD COLUMN IF NOT EXISTS "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0);

CREATE TABLE IF NOT EXISTS "player_classlevelstats" (
    "class" SMALLINT NOT NULL CHECK ("class" >= 0),
    "level" SMALLINT NOT NULL CHECK ("level" >= 0),
    "basehp" INTEGER NOT NULL CHECK ("basehp" >= 0),
    "basemana" INTEGER NOT NULL CHECK ("basemana" >= 0),
    PRIMARY KEY ("class", "level")
);
ALTER TABLE IF EXISTS "player_classlevelstats" ADD COLUMN IF NOT EXISTS "class" SMALLINT NOT NULL CHECK ("class" >= 0);
ALTER TABLE IF EXISTS "player_classlevelstats" ADD COLUMN IF NOT EXISTS "level" SMALLINT NOT NULL CHECK ("level" >= 0);
ALTER TABLE IF EXISTS "player_classlevelstats" ADD COLUMN IF NOT EXISTS "basehp" INTEGER NOT NULL CHECK ("basehp" >= 0);
ALTER TABLE IF EXISTS "player_classlevelstats" ADD COLUMN IF NOT EXISTS "basemana" INTEGER NOT NULL CHECK ("basemana" >= 0);

CREATE TABLE IF NOT EXISTS "player_factionchange_items" (
    "alliance_id" INTEGER NOT NULL,
    "horde_id" INTEGER NOT NULL,
    "comment" VARCHAR(255) NOT NULL DEFAULT '',
    PRIMARY KEY ("alliance_id", "horde_id")
);
ALTER TABLE IF EXISTS "player_factionchange_items" ADD COLUMN IF NOT EXISTS "alliance_id" INTEGER NOT NULL;
ALTER TABLE IF EXISTS "player_factionchange_items" ADD COLUMN IF NOT EXISTS "horde_id" INTEGER NOT NULL;
ALTER TABLE IF EXISTS "player_factionchange_items" ADD COLUMN IF NOT EXISTS "comment" VARCHAR(255) NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS "player_factionchange_mounts" (
    "RaceId" INTEGER NOT NULL,
    "MountNum" INTEGER NOT NULL,
    "ItemEntry" INTEGER NOT NULL,
    "Comment" VARCHAR(255) NOT NULL DEFAULT '',
    PRIMARY KEY ("RaceId", "MountNum")
);
ALTER TABLE IF EXISTS "player_factionchange_mounts" ADD COLUMN IF NOT EXISTS "RaceId" INTEGER NOT NULL;
ALTER TABLE IF EXISTS "player_factionchange_mounts" ADD COLUMN IF NOT EXISTS "MountNum" INTEGER NOT NULL;
ALTER TABLE IF EXISTS "player_factionchange_mounts" ADD COLUMN IF NOT EXISTS "ItemEntry" INTEGER NOT NULL;
ALTER TABLE IF EXISTS "player_factionchange_mounts" ADD COLUMN IF NOT EXISTS "Comment" VARCHAR(255) NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS "player_factionchange_quests" (
    "alliance_id" INTEGER NOT NULL,
    "horde_id" INTEGER NOT NULL,
    "comment" VARCHAR(255) NOT NULL DEFAULT '',
    PRIMARY KEY ("alliance_id", "horde_id")
);
ALTER TABLE IF EXISTS "player_factionchange_quests" ADD COLUMN IF NOT EXISTS "alliance_id" INTEGER NOT NULL;
ALTER TABLE IF EXISTS "player_factionchange_quests" ADD COLUMN IF NOT EXISTS "horde_id" INTEGER NOT NULL;
ALTER TABLE IF EXISTS "player_factionchange_quests" ADD COLUMN IF NOT EXISTS "comment" VARCHAR(255) NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS "player_factionchange_reputations" (
    "alliance_id" INTEGER NOT NULL,
    "horde_id" INTEGER NOT NULL,
    PRIMARY KEY ("alliance_id", "horde_id")
);
ALTER TABLE IF EXISTS "player_factionchange_reputations" ADD COLUMN IF NOT EXISTS "alliance_id" INTEGER NOT NULL;
ALTER TABLE IF EXISTS "player_factionchange_reputations" ADD COLUMN IF NOT EXISTS "horde_id" INTEGER NOT NULL;

CREATE TABLE IF NOT EXISTS "player_factionchange_spells" (
    "alliance_id" INTEGER NOT NULL CHECK ("alliance_id" >= 0),
    "horde_id" INTEGER NOT NULL CHECK ("horde_id" >= 0),
    "comment" VARCHAR(255) NOT NULL DEFAULT '',
    PRIMARY KEY ("alliance_id", "horde_id")
);
ALTER TABLE IF EXISTS "player_factionchange_spells" ADD COLUMN IF NOT EXISTS "alliance_id" INTEGER NOT NULL CHECK ("alliance_id" >= 0);
ALTER TABLE IF EXISTS "player_factionchange_spells" ADD COLUMN IF NOT EXISTS "horde_id" INTEGER NOT NULL CHECK ("horde_id" >= 0);
ALTER TABLE IF EXISTS "player_factionchange_spells" ADD COLUMN IF NOT EXISTS "comment" VARCHAR(255) NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS "player_levelstats" (
    "race" SMALLINT NOT NULL CHECK ("race" >= 0),
    "class" SMALLINT NOT NULL CHECK ("class" >= 0),
    "level" SMALLINT NOT NULL CHECK ("level" >= 0),
    "str" SMALLINT NOT NULL CHECK ("str" >= 0),
    "agi" SMALLINT NOT NULL CHECK ("agi" >= 0),
    "sta" SMALLINT NOT NULL CHECK ("sta" >= 0),
    "inte" SMALLINT NOT NULL CHECK ("inte" >= 0),
    "spi" SMALLINT NOT NULL CHECK ("spi" >= 0),
    PRIMARY KEY ("race", "class", "level")
);
ALTER TABLE IF EXISTS "player_levelstats" ADD COLUMN IF NOT EXISTS "race" SMALLINT NOT NULL CHECK ("race" >= 0);
ALTER TABLE IF EXISTS "player_levelstats" ADD COLUMN IF NOT EXISTS "class" SMALLINT NOT NULL CHECK ("class" >= 0);
ALTER TABLE IF EXISTS "player_levelstats" ADD COLUMN IF NOT EXISTS "level" SMALLINT NOT NULL CHECK ("level" >= 0);
ALTER TABLE IF EXISTS "player_levelstats" ADD COLUMN IF NOT EXISTS "str" SMALLINT NOT NULL CHECK ("str" >= 0);
ALTER TABLE IF EXISTS "player_levelstats" ADD COLUMN IF NOT EXISTS "agi" SMALLINT NOT NULL CHECK ("agi" >= 0);
ALTER TABLE IF EXISTS "player_levelstats" ADD COLUMN IF NOT EXISTS "sta" SMALLINT NOT NULL CHECK ("sta" >= 0);
ALTER TABLE IF EXISTS "player_levelstats" ADD COLUMN IF NOT EXISTS "inte" SMALLINT NOT NULL CHECK ("inte" >= 0);
ALTER TABLE IF EXISTS "player_levelstats" ADD COLUMN IF NOT EXISTS "spi" SMALLINT NOT NULL CHECK ("spi" >= 0);

CREATE TABLE IF NOT EXISTS "player_premade_item" (
    "entry" BIGINT NOT NULL CHECK ("entry" >= 0),
    "item" BIGINT NOT NULL CHECK ("item" >= 0),
    "enchant" BIGINT NOT NULL DEFAULT '0' CHECK ("enchant" >= 0),
    "team" BIGINT NOT NULL DEFAULT '0' CHECK ("team" >= 0)
);
ALTER TABLE IF EXISTS "player_premade_item" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "player_premade_item" ADD COLUMN IF NOT EXISTS "item" BIGINT NOT NULL CHECK ("item" >= 0);
ALTER TABLE IF EXISTS "player_premade_item" ADD COLUMN IF NOT EXISTS "enchant" BIGINT NOT NULL DEFAULT '0' CHECK ("enchant" >= 0);
ALTER TABLE IF EXISTS "player_premade_item" ADD COLUMN IF NOT EXISTS "team" BIGINT NOT NULL DEFAULT '0' CHECK ("team" >= 0);

CREATE TABLE IF NOT EXISTS "player_premade_item_template" (
    "entry" BIGINT NOT NULL CHECK ("entry" >= 0),
    "class" SMALLINT NOT NULL DEFAULT '0' CHECK ("class" >= 0),
    "level" SMALLINT NOT NULL DEFAULT '60' CHECK ("level" >= 0),
    "role" SMALLINT NOT NULL DEFAULT '0' CHECK ("role" >= 0),
    "name" VARCHAR(50) DEFAULT '',
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "player_premade_item_template" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "player_premade_item_template" ADD COLUMN IF NOT EXISTS "class" SMALLINT NOT NULL DEFAULT '0' CHECK ("class" >= 0);
ALTER TABLE IF EXISTS "player_premade_item_template" ADD COLUMN IF NOT EXISTS "level" SMALLINT NOT NULL DEFAULT '60' CHECK ("level" >= 0);
ALTER TABLE IF EXISTS "player_premade_item_template" ADD COLUMN IF NOT EXISTS "role" SMALLINT NOT NULL DEFAULT '0' CHECK ("role" >= 0);
ALTER TABLE IF EXISTS "player_premade_item_template" ADD COLUMN IF NOT EXISTS "name" VARCHAR(50) DEFAULT '';

CREATE TABLE IF NOT EXISTS "player_premade_spell" (
    "entry" BIGINT NOT NULL CHECK ("entry" >= 0),
    "spell" INTEGER NOT NULL CHECK ("spell" >= 0),
    PRIMARY KEY ("entry", "spell")
);
ALTER TABLE IF EXISTS "player_premade_spell" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "player_premade_spell" ADD COLUMN IF NOT EXISTS "spell" INTEGER NOT NULL CHECK ("spell" >= 0);

CREATE TABLE IF NOT EXISTS "player_premade_spell_template" (
    "entry" BIGINT NOT NULL CHECK ("entry" >= 0),
    "class" SMALLINT NOT NULL DEFAULT '0' CHECK ("class" >= 0),
    "level" SMALLINT NOT NULL DEFAULT '60' CHECK ("level" >= 0),
    "role" SMALLINT NOT NULL DEFAULT '0' CHECK ("role" >= 0),
    "name" VARCHAR(50) DEFAULT '',
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "player_premade_spell_template" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "player_premade_spell_template" ADD COLUMN IF NOT EXISTS "class" SMALLINT NOT NULL DEFAULT '0' CHECK ("class" >= 0);
ALTER TABLE IF EXISTS "player_premade_spell_template" ADD COLUMN IF NOT EXISTS "level" SMALLINT NOT NULL DEFAULT '60' CHECK ("level" >= 0);
ALTER TABLE IF EXISTS "player_premade_spell_template" ADD COLUMN IF NOT EXISTS "role" SMALLINT NOT NULL DEFAULT '0' CHECK ("role" >= 0);
ALTER TABLE IF EXISTS "player_premade_spell_template" ADD COLUMN IF NOT EXISTS "name" VARCHAR(50) DEFAULT '';

CREATE TABLE IF NOT EXISTS "player_xp_for_level" (
    "lvl" BIGINT NOT NULL CHECK ("lvl" >= 0),
    "xp_for_next_level" BIGINT NOT NULL CHECK ("xp_for_next_level" >= 0),
    PRIMARY KEY ("lvl")
);
ALTER TABLE IF EXISTS "player_xp_for_level" ADD COLUMN IF NOT EXISTS "lvl" BIGINT NOT NULL CHECK ("lvl" >= 0);
ALTER TABLE IF EXISTS "player_xp_for_level" ADD COLUMN IF NOT EXISTS "xp_for_next_level" BIGINT NOT NULL CHECK ("xp_for_next_level" >= 0);

CREATE TABLE IF NOT EXISTS "playercreateinfo" (
    "race" SMALLINT NOT NULL DEFAULT '0' CHECK ("race" >= 0),
    "class" SMALLINT NOT NULL DEFAULT '0' CHECK ("class" >= 0),
    "map" INTEGER NOT NULL DEFAULT '0' CHECK ("map" >= 0),
    "zone" BIGINT NOT NULL DEFAULT '0' CHECK ("zone" >= 0),
    "position_x" REAL NOT NULL DEFAULT '0',
    "position_y" REAL NOT NULL DEFAULT '0',
    "position_z" REAL NOT NULL DEFAULT '0',
    "orientation" REAL NOT NULL DEFAULT '0',
    PRIMARY KEY ("race", "class")
);
ALTER TABLE IF EXISTS "playercreateinfo" ADD COLUMN IF NOT EXISTS "race" SMALLINT NOT NULL DEFAULT '0' CHECK ("race" >= 0);
ALTER TABLE IF EXISTS "playercreateinfo" ADD COLUMN IF NOT EXISTS "class" SMALLINT NOT NULL DEFAULT '0' CHECK ("class" >= 0);
ALTER TABLE IF EXISTS "playercreateinfo" ADD COLUMN IF NOT EXISTS "map" INTEGER NOT NULL DEFAULT '0' CHECK ("map" >= 0);
ALTER TABLE IF EXISTS "playercreateinfo" ADD COLUMN IF NOT EXISTS "zone" BIGINT NOT NULL DEFAULT '0' CHECK ("zone" >= 0);
ALTER TABLE IF EXISTS "playercreateinfo" ADD COLUMN IF NOT EXISTS "position_x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "playercreateinfo" ADD COLUMN IF NOT EXISTS "position_y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "playercreateinfo" ADD COLUMN IF NOT EXISTS "position_z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "playercreateinfo" ADD COLUMN IF NOT EXISTS "orientation" REAL NOT NULL DEFAULT '0';

CREATE TABLE IF NOT EXISTS "playercreateinfo_action" (
    "race" SMALLINT NOT NULL DEFAULT '0' CHECK ("race" >= 0),
    "class" SMALLINT NOT NULL DEFAULT '0' CHECK ("class" >= 0),
    "button" INTEGER NOT NULL DEFAULT '0' CHECK ("button" >= 0),
    "action" BIGINT NOT NULL DEFAULT '0' CHECK ("action" >= 0),
    "type" INTEGER NOT NULL DEFAULT '0' CHECK ("type" >= 0),
    PRIMARY KEY ("race", "class", "button")
);
ALTER TABLE IF EXISTS "playercreateinfo_action" ADD COLUMN IF NOT EXISTS "race" SMALLINT NOT NULL DEFAULT '0' CHECK ("race" >= 0);
ALTER TABLE IF EXISTS "playercreateinfo_action" ADD COLUMN IF NOT EXISTS "class" SMALLINT NOT NULL DEFAULT '0' CHECK ("class" >= 0);
ALTER TABLE IF EXISTS "playercreateinfo_action" ADD COLUMN IF NOT EXISTS "button" INTEGER NOT NULL DEFAULT '0' CHECK ("button" >= 0);
ALTER TABLE IF EXISTS "playercreateinfo_action" ADD COLUMN IF NOT EXISTS "action" BIGINT NOT NULL DEFAULT '0' CHECK ("action" >= 0);
ALTER TABLE IF EXISTS "playercreateinfo_action" ADD COLUMN IF NOT EXISTS "type" INTEGER NOT NULL DEFAULT '0' CHECK ("type" >= 0);
CREATE INDEX IF NOT EXISTS idx_playercreateinfo_action_playercreateinfo_race_class_index ON "playercreateinfo_action" ("race", "class");

CREATE TABLE IF NOT EXISTS "playercreateinfo_item" (
    "race" SMALLINT NOT NULL DEFAULT '0' CHECK ("race" >= 0),
    "class" SMALLINT NOT NULL DEFAULT '0' CHECK ("class" >= 0),
    "itemid" BIGINT NOT NULL DEFAULT '0' CHECK ("itemid" >= 0),
    "amount" SMALLINT NOT NULL DEFAULT '1' CHECK ("amount" >= 0)
);
ALTER TABLE IF EXISTS "playercreateinfo_item" ADD COLUMN IF NOT EXISTS "race" SMALLINT NOT NULL DEFAULT '0' CHECK ("race" >= 0);
ALTER TABLE IF EXISTS "playercreateinfo_item" ADD COLUMN IF NOT EXISTS "class" SMALLINT NOT NULL DEFAULT '0' CHECK ("class" >= 0);
ALTER TABLE IF EXISTS "playercreateinfo_item" ADD COLUMN IF NOT EXISTS "itemid" BIGINT NOT NULL DEFAULT '0' CHECK ("itemid" >= 0);
ALTER TABLE IF EXISTS "playercreateinfo_item" ADD COLUMN IF NOT EXISTS "amount" SMALLINT NOT NULL DEFAULT '1' CHECK ("amount" >= 0);
CREATE INDEX IF NOT EXISTS idx_playercreateinfo_item_playercreateinfo_race_class_index ON "playercreateinfo_item" ("race", "class");

CREATE TABLE IF NOT EXISTS "playercreateinfo_spell" (
    "race" SMALLINT NOT NULL DEFAULT '0' CHECK ("race" >= 0),
    "class" SMALLINT NOT NULL DEFAULT '0' CHECK ("class" >= 0),
    "spell" INTEGER NOT NULL DEFAULT '0' CHECK ("spell" >= 0),
    "build_min" INTEGER NOT NULL DEFAULT '0' CHECK ("build_min" >= 0),
    "build_max" INTEGER NOT NULL DEFAULT '5875' CHECK ("build_max" >= 0),
    "note" VARCHAR(255) DEFAULT NULL,
    PRIMARY KEY ("race", "class", "spell")
);
ALTER TABLE IF EXISTS "playercreateinfo_spell" ADD COLUMN IF NOT EXISTS "race" SMALLINT NOT NULL DEFAULT '0' CHECK ("race" >= 0);
ALTER TABLE IF EXISTS "playercreateinfo_spell" ADD COLUMN IF NOT EXISTS "class" SMALLINT NOT NULL DEFAULT '0' CHECK ("class" >= 0);
ALTER TABLE IF EXISTS "playercreateinfo_spell" ADD COLUMN IF NOT EXISTS "spell" INTEGER NOT NULL DEFAULT '0' CHECK ("spell" >= 0);
ALTER TABLE IF EXISTS "playercreateinfo_spell" ADD COLUMN IF NOT EXISTS "build_min" INTEGER NOT NULL DEFAULT '0' CHECK ("build_min" >= 0);
ALTER TABLE IF EXISTS "playercreateinfo_spell" ADD COLUMN IF NOT EXISTS "build_max" INTEGER NOT NULL DEFAULT '5875' CHECK ("build_max" >= 0);
ALTER TABLE IF EXISTS "playercreateinfo_spell" ADD COLUMN IF NOT EXISTS "note" VARCHAR(255) DEFAULT NULL;

CREATE TABLE IF NOT EXISTS "points_of_interest" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "x" REAL NOT NULL DEFAULT '0',
    "y" REAL NOT NULL DEFAULT '0',
    "icon" BIGINT NOT NULL DEFAULT '0' CHECK ("icon" >= 0),
    "flags" BIGINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0),
    "data" BIGINT NOT NULL DEFAULT '0' CHECK ("data" >= 0),
    "icon_name" TEXT NOT NULL,
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "points_of_interest" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "points_of_interest" ADD COLUMN IF NOT EXISTS "x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "points_of_interest" ADD COLUMN IF NOT EXISTS "y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "points_of_interest" ADD COLUMN IF NOT EXISTS "icon" BIGINT NOT NULL DEFAULT '0' CHECK ("icon" >= 0);
ALTER TABLE IF EXISTS "points_of_interest" ADD COLUMN IF NOT EXISTS "flags" BIGINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0);
ALTER TABLE IF EXISTS "points_of_interest" ADD COLUMN IF NOT EXISTS "data" BIGINT NOT NULL DEFAULT '0' CHECK ("data" >= 0);
ALTER TABLE IF EXISTS "points_of_interest" ADD COLUMN IF NOT EXISTS "icon_name" TEXT NOT NULL;

CREATE TABLE IF NOT EXISTS "pool_creature" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "pool_entry" INTEGER NOT NULL DEFAULT '0' CHECK ("pool_entry" >= 0),
    "chance" REAL NOT NULL DEFAULT '0' CHECK ("chance" >= 0),
    "description" VARCHAR(255) NOT NULL,
    "flags" BIGINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0),
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0),
    PRIMARY KEY ("guid")
);
ALTER TABLE IF EXISTS "pool_creature" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "pool_creature" ADD COLUMN IF NOT EXISTS "pool_entry" INTEGER NOT NULL DEFAULT '0' CHECK ("pool_entry" >= 0);
ALTER TABLE IF EXISTS "pool_creature" ADD COLUMN IF NOT EXISTS "chance" REAL NOT NULL DEFAULT '0' CHECK ("chance" >= 0);
ALTER TABLE IF EXISTS "pool_creature" ADD COLUMN IF NOT EXISTS "description" VARCHAR(255) NOT NULL;
ALTER TABLE IF EXISTS "pool_creature" ADD COLUMN IF NOT EXISTS "flags" BIGINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0);
ALTER TABLE IF EXISTS "pool_creature" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);
ALTER TABLE IF EXISTS "pool_creature" ADD COLUMN IF NOT EXISTS "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0);
CREATE INDEX IF NOT EXISTS idx_pool_creature_pool_idx ON "pool_creature" ("pool_entry");

CREATE TABLE IF NOT EXISTS "pool_creature_template" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "pool_entry" INTEGER NOT NULL DEFAULT '0' CHECK ("pool_entry" >= 0),
    "chance" REAL NOT NULL DEFAULT '0' CHECK ("chance" >= 0),
    "description" VARCHAR(255) NOT NULL,
    "flags" BIGINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0),
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0),
    PRIMARY KEY ("id")
);
ALTER TABLE IF EXISTS "pool_creature_template" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "pool_creature_template" ADD COLUMN IF NOT EXISTS "pool_entry" INTEGER NOT NULL DEFAULT '0' CHECK ("pool_entry" >= 0);
ALTER TABLE IF EXISTS "pool_creature_template" ADD COLUMN IF NOT EXISTS "chance" REAL NOT NULL DEFAULT '0' CHECK ("chance" >= 0);
ALTER TABLE IF EXISTS "pool_creature_template" ADD COLUMN IF NOT EXISTS "description" VARCHAR(255) NOT NULL;
ALTER TABLE IF EXISTS "pool_creature_template" ADD COLUMN IF NOT EXISTS "flags" BIGINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0);
ALTER TABLE IF EXISTS "pool_creature_template" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);
ALTER TABLE IF EXISTS "pool_creature_template" ADD COLUMN IF NOT EXISTS "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0);
CREATE INDEX IF NOT EXISTS idx_pool_creature_template_pool_idx ON "pool_creature_template" ("pool_entry");

CREATE TABLE IF NOT EXISTS "pool_gameobject" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "pool_entry" INTEGER NOT NULL DEFAULT '0' CHECK ("pool_entry" >= 0),
    "chance" REAL NOT NULL DEFAULT '0' CHECK ("chance" >= 0),
    "description" VARCHAR(255) NOT NULL,
    "flags" BIGINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0),
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0),
    PRIMARY KEY ("guid")
);
ALTER TABLE IF EXISTS "pool_gameobject" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "pool_gameobject" ADD COLUMN IF NOT EXISTS "pool_entry" INTEGER NOT NULL DEFAULT '0' CHECK ("pool_entry" >= 0);
ALTER TABLE IF EXISTS "pool_gameobject" ADD COLUMN IF NOT EXISTS "chance" REAL NOT NULL DEFAULT '0' CHECK ("chance" >= 0);
ALTER TABLE IF EXISTS "pool_gameobject" ADD COLUMN IF NOT EXISTS "description" VARCHAR(255) NOT NULL;
ALTER TABLE IF EXISTS "pool_gameobject" ADD COLUMN IF NOT EXISTS "flags" BIGINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0);
ALTER TABLE IF EXISTS "pool_gameobject" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);
ALTER TABLE IF EXISTS "pool_gameobject" ADD COLUMN IF NOT EXISTS "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0);
CREATE INDEX IF NOT EXISTS idx_pool_gameobject_pool_idx ON "pool_gameobject" ("pool_entry");

CREATE TABLE IF NOT EXISTS "pool_gameobject_template" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "pool_entry" INTEGER NOT NULL DEFAULT '0' CHECK ("pool_entry" >= 0),
    "chance" REAL NOT NULL DEFAULT '0' CHECK ("chance" >= 0),
    "description" VARCHAR(255) NOT NULL,
    "flags" BIGINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0),
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0),
    PRIMARY KEY ("id")
);
ALTER TABLE IF EXISTS "pool_gameobject_template" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "pool_gameobject_template" ADD COLUMN IF NOT EXISTS "pool_entry" INTEGER NOT NULL DEFAULT '0' CHECK ("pool_entry" >= 0);
ALTER TABLE IF EXISTS "pool_gameobject_template" ADD COLUMN IF NOT EXISTS "chance" REAL NOT NULL DEFAULT '0' CHECK ("chance" >= 0);
ALTER TABLE IF EXISTS "pool_gameobject_template" ADD COLUMN IF NOT EXISTS "description" VARCHAR(255) NOT NULL;
ALTER TABLE IF EXISTS "pool_gameobject_template" ADD COLUMN IF NOT EXISTS "flags" BIGINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0);
ALTER TABLE IF EXISTS "pool_gameobject_template" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);
ALTER TABLE IF EXISTS "pool_gameobject_template" ADD COLUMN IF NOT EXISTS "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0);
CREATE INDEX IF NOT EXISTS idx_pool_gameobject_template_pool_idx ON "pool_gameobject_template" ("pool_entry");

CREATE TABLE IF NOT EXISTS "pool_pool" (
    "pool_id" INTEGER NOT NULL DEFAULT '0' CHECK ("pool_id" >= 0),
    "mother_pool" INTEGER NOT NULL DEFAULT '0' CHECK ("mother_pool" >= 0),
    "chance" REAL NOT NULL DEFAULT '0',
    "description" VARCHAR(255) NOT NULL,
    "flags" BIGINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0),
    PRIMARY KEY ("pool_id")
);
ALTER TABLE IF EXISTS "pool_pool" ADD COLUMN IF NOT EXISTS "pool_id" INTEGER NOT NULL DEFAULT '0' CHECK ("pool_id" >= 0);
ALTER TABLE IF EXISTS "pool_pool" ADD COLUMN IF NOT EXISTS "mother_pool" INTEGER NOT NULL DEFAULT '0' CHECK ("mother_pool" >= 0);
ALTER TABLE IF EXISTS "pool_pool" ADD COLUMN IF NOT EXISTS "chance" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "pool_pool" ADD COLUMN IF NOT EXISTS "description" VARCHAR(255) NOT NULL;
ALTER TABLE IF EXISTS "pool_pool" ADD COLUMN IF NOT EXISTS "flags" BIGINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0);
CREATE INDEX IF NOT EXISTS idx_pool_pool_pool_idx ON "pool_pool" ("mother_pool");

CREATE TABLE IF NOT EXISTS "pool_template" (
    "entry" INTEGER NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "max_limit" BIGINT NOT NULL DEFAULT '0' CHECK ("max_limit" >= 0),
    "description" VARCHAR(255) NOT NULL,
    "flags" BIGINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0),
    "instance" INTEGER NOT NULL DEFAULT '0',
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0),
    PRIMARY KEY ("entry", "patch_min", "patch_max")
);
ALTER TABLE IF EXISTS "pool_template" ADD COLUMN IF NOT EXISTS "entry" INTEGER NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "pool_template" ADD COLUMN IF NOT EXISTS "max_limit" BIGINT NOT NULL DEFAULT '0' CHECK ("max_limit" >= 0);
ALTER TABLE IF EXISTS "pool_template" ADD COLUMN IF NOT EXISTS "description" VARCHAR(255) NOT NULL;
ALTER TABLE IF EXISTS "pool_template" ADD COLUMN IF NOT EXISTS "flags" BIGINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0);
ALTER TABLE IF EXISTS "pool_template" ADD COLUMN IF NOT EXISTS "instance" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "pool_template" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);
ALTER TABLE IF EXISTS "pool_template" ADD COLUMN IF NOT EXISTS "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0);

CREATE TABLE IF NOT EXISTS "quest_end_scripts" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "delay" BIGINT NOT NULL DEFAULT '0' CHECK ("delay" >= 0),
    "priority" SMALLINT NOT NULL DEFAULT '0' CHECK ("priority" >= 0),
    "command" SMALLINT NOT NULL DEFAULT '0' CHECK ("command" >= 0),
    "datalong" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong" >= 0),
    "datalong2" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong2" >= 0),
    "datalong3" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong3" >= 0),
    "datalong4" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong4" >= 0),
    "target_param1" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param1" >= 0),
    "target_param2" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param2" >= 0),
    "target_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("target_type" >= 0),
    "data_flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("data_flags" >= 0),
    "dataint" INTEGER NOT NULL DEFAULT '0',
    "dataint2" INTEGER NOT NULL DEFAULT '0',
    "dataint3" INTEGER NOT NULL DEFAULT '0',
    "dataint4" INTEGER NOT NULL DEFAULT '0',
    "x" REAL NOT NULL DEFAULT '0',
    "y" REAL NOT NULL DEFAULT '0',
    "z" REAL NOT NULL DEFAULT '0',
    "o" REAL NOT NULL DEFAULT '0',
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    "comments" VARCHAR(255) NOT NULL
);
ALTER TABLE IF EXISTS "quest_end_scripts" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "quest_end_scripts" ADD COLUMN IF NOT EXISTS "delay" BIGINT NOT NULL DEFAULT '0' CHECK ("delay" >= 0);
ALTER TABLE IF EXISTS "quest_end_scripts" ADD COLUMN IF NOT EXISTS "priority" SMALLINT NOT NULL DEFAULT '0' CHECK ("priority" >= 0);
ALTER TABLE IF EXISTS "quest_end_scripts" ADD COLUMN IF NOT EXISTS "command" SMALLINT NOT NULL DEFAULT '0' CHECK ("command" >= 0);
ALTER TABLE IF EXISTS "quest_end_scripts" ADD COLUMN IF NOT EXISTS "datalong" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong" >= 0);
ALTER TABLE IF EXISTS "quest_end_scripts" ADD COLUMN IF NOT EXISTS "datalong2" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong2" >= 0);
ALTER TABLE IF EXISTS "quest_end_scripts" ADD COLUMN IF NOT EXISTS "datalong3" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong3" >= 0);
ALTER TABLE IF EXISTS "quest_end_scripts" ADD COLUMN IF NOT EXISTS "datalong4" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong4" >= 0);
ALTER TABLE IF EXISTS "quest_end_scripts" ADD COLUMN IF NOT EXISTS "target_param1" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param1" >= 0);
ALTER TABLE IF EXISTS "quest_end_scripts" ADD COLUMN IF NOT EXISTS "target_param2" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param2" >= 0);
ALTER TABLE IF EXISTS "quest_end_scripts" ADD COLUMN IF NOT EXISTS "target_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("target_type" >= 0);
ALTER TABLE IF EXISTS "quest_end_scripts" ADD COLUMN IF NOT EXISTS "data_flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("data_flags" >= 0);
ALTER TABLE IF EXISTS "quest_end_scripts" ADD COLUMN IF NOT EXISTS "dataint" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_end_scripts" ADD COLUMN IF NOT EXISTS "dataint2" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_end_scripts" ADD COLUMN IF NOT EXISTS "dataint3" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_end_scripts" ADD COLUMN IF NOT EXISTS "dataint4" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_end_scripts" ADD COLUMN IF NOT EXISTS "x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_end_scripts" ADD COLUMN IF NOT EXISTS "y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_end_scripts" ADD COLUMN IF NOT EXISTS "z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_end_scripts" ADD COLUMN IF NOT EXISTS "o" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_end_scripts" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);
ALTER TABLE IF EXISTS "quest_end_scripts" ADD COLUMN IF NOT EXISTS "comments" VARCHAR(255) NOT NULL;

CREATE TABLE IF NOT EXISTS "quest_greeting" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "type" SMALLINT NOT NULL DEFAULT '0' CHECK ("type" >= 0),
    "content_default" TEXT NOT NULL,
    "content_loc1" TEXT,
    "content_loc2" TEXT,
    "content_loc3" TEXT,
    "content_loc4" TEXT,
    "content_loc5" TEXT,
    "content_loc6" TEXT,
    "content_loc7" TEXT,
    "content_loc8" TEXT,
    "emote_id" INTEGER NOT NULL DEFAULT '0' CHECK ("emote_id" >= 0),
    "emote_delay" BIGINT NOT NULL DEFAULT '0' CHECK ("emote_delay" >= 0),
    PRIMARY KEY ("entry", "type")
);
ALTER TABLE IF EXISTS "quest_greeting" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "quest_greeting" ADD COLUMN IF NOT EXISTS "type" SMALLINT NOT NULL DEFAULT '0' CHECK ("type" >= 0);
ALTER TABLE IF EXISTS "quest_greeting" ADD COLUMN IF NOT EXISTS "content_default" TEXT NOT NULL;
ALTER TABLE IF EXISTS "quest_greeting" ADD COLUMN IF NOT EXISTS "content_loc1" TEXT;
ALTER TABLE IF EXISTS "quest_greeting" ADD COLUMN IF NOT EXISTS "content_loc2" TEXT;
ALTER TABLE IF EXISTS "quest_greeting" ADD COLUMN IF NOT EXISTS "content_loc3" TEXT;
ALTER TABLE IF EXISTS "quest_greeting" ADD COLUMN IF NOT EXISTS "content_loc4" TEXT;
ALTER TABLE IF EXISTS "quest_greeting" ADD COLUMN IF NOT EXISTS "content_loc5" TEXT;
ALTER TABLE IF EXISTS "quest_greeting" ADD COLUMN IF NOT EXISTS "content_loc6" TEXT;
ALTER TABLE IF EXISTS "quest_greeting" ADD COLUMN IF NOT EXISTS "content_loc7" TEXT;
ALTER TABLE IF EXISTS "quest_greeting" ADD COLUMN IF NOT EXISTS "content_loc8" TEXT;
ALTER TABLE IF EXISTS "quest_greeting" ADD COLUMN IF NOT EXISTS "emote_id" INTEGER NOT NULL DEFAULT '0' CHECK ("emote_id" >= 0);
ALTER TABLE IF EXISTS "quest_greeting" ADD COLUMN IF NOT EXISTS "emote_delay" BIGINT NOT NULL DEFAULT '0' CHECK ("emote_delay" >= 0);

CREATE TABLE IF NOT EXISTS "quest_start_scripts" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "delay" BIGINT NOT NULL DEFAULT '0' CHECK ("delay" >= 0),
    "priority" SMALLINT NOT NULL DEFAULT '0' CHECK ("priority" >= 0),
    "command" SMALLINT NOT NULL DEFAULT '0' CHECK ("command" >= 0),
    "datalong" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong" >= 0),
    "datalong2" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong2" >= 0),
    "datalong3" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong3" >= 0),
    "datalong4" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong4" >= 0),
    "target_param1" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param1" >= 0),
    "target_param2" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param2" >= 0),
    "target_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("target_type" >= 0),
    "data_flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("data_flags" >= 0),
    "dataint" INTEGER NOT NULL DEFAULT '0',
    "dataint2" INTEGER NOT NULL DEFAULT '0',
    "dataint3" INTEGER NOT NULL DEFAULT '0',
    "dataint4" INTEGER NOT NULL DEFAULT '0',
    "x" REAL NOT NULL DEFAULT '0',
    "y" REAL NOT NULL DEFAULT '0',
    "z" REAL NOT NULL DEFAULT '0',
    "o" REAL NOT NULL DEFAULT '0',
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    "comments" VARCHAR(255) NOT NULL
);
ALTER TABLE IF EXISTS "quest_start_scripts" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "quest_start_scripts" ADD COLUMN IF NOT EXISTS "delay" BIGINT NOT NULL DEFAULT '0' CHECK ("delay" >= 0);
ALTER TABLE IF EXISTS "quest_start_scripts" ADD COLUMN IF NOT EXISTS "priority" SMALLINT NOT NULL DEFAULT '0' CHECK ("priority" >= 0);
ALTER TABLE IF EXISTS "quest_start_scripts" ADD COLUMN IF NOT EXISTS "command" SMALLINT NOT NULL DEFAULT '0' CHECK ("command" >= 0);
ALTER TABLE IF EXISTS "quest_start_scripts" ADD COLUMN IF NOT EXISTS "datalong" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong" >= 0);
ALTER TABLE IF EXISTS "quest_start_scripts" ADD COLUMN IF NOT EXISTS "datalong2" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong2" >= 0);
ALTER TABLE IF EXISTS "quest_start_scripts" ADD COLUMN IF NOT EXISTS "datalong3" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong3" >= 0);
ALTER TABLE IF EXISTS "quest_start_scripts" ADD COLUMN IF NOT EXISTS "datalong4" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong4" >= 0);
ALTER TABLE IF EXISTS "quest_start_scripts" ADD COLUMN IF NOT EXISTS "target_param1" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param1" >= 0);
ALTER TABLE IF EXISTS "quest_start_scripts" ADD COLUMN IF NOT EXISTS "target_param2" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param2" >= 0);
ALTER TABLE IF EXISTS "quest_start_scripts" ADD COLUMN IF NOT EXISTS "target_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("target_type" >= 0);
ALTER TABLE IF EXISTS "quest_start_scripts" ADD COLUMN IF NOT EXISTS "data_flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("data_flags" >= 0);
ALTER TABLE IF EXISTS "quest_start_scripts" ADD COLUMN IF NOT EXISTS "dataint" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_start_scripts" ADD COLUMN IF NOT EXISTS "dataint2" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_start_scripts" ADD COLUMN IF NOT EXISTS "dataint3" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_start_scripts" ADD COLUMN IF NOT EXISTS "dataint4" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_start_scripts" ADD COLUMN IF NOT EXISTS "x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_start_scripts" ADD COLUMN IF NOT EXISTS "y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_start_scripts" ADD COLUMN IF NOT EXISTS "z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_start_scripts" ADD COLUMN IF NOT EXISTS "o" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_start_scripts" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);
ALTER TABLE IF EXISTS "quest_start_scripts" ADD COLUMN IF NOT EXISTS "comments" VARCHAR(255) NOT NULL;

CREATE TABLE IF NOT EXISTS "quest_template" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "patch" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch" >= 0),
    "Method" SMALLINT NOT NULL DEFAULT '2' CHECK ("Method" >= 0),
    "ZoneOrSort" SMALLINT NOT NULL DEFAULT '0',
    "MinLevel" SMALLINT NOT NULL DEFAULT '0' CHECK ("MinLevel" >= 0),
    "MaxLevel" SMALLINT NOT NULL DEFAULT '0' CHECK ("MaxLevel" >= 0),
    "QuestLevel" SMALLINT NOT NULL DEFAULT '0' CHECK ("QuestLevel" >= 0),
    "Type" INTEGER NOT NULL DEFAULT '0' CHECK ("Type" >= 0),
    "RequiredClasses" INTEGER NOT NULL DEFAULT '0' CHECK ("RequiredClasses" >= 0),
    "RequiredRaces" INTEGER NOT NULL DEFAULT '0' CHECK ("RequiredRaces" >= 0),
    "RequiredSkill" INTEGER NOT NULL DEFAULT '0' CHECK ("RequiredSkill" >= 0),
    "RequiredSkillValue" INTEGER NOT NULL DEFAULT '0' CHECK ("RequiredSkillValue" >= 0),
    "RequiredCondition" BIGINT NOT NULL DEFAULT '0' CHECK ("RequiredCondition" >= 0),
    "RepObjectiveFaction" INTEGER NOT NULL DEFAULT '0' CHECK ("RepObjectiveFaction" >= 0),
    "RepObjectiveValue" INTEGER NOT NULL DEFAULT '0',
    "RequiredMinRepFaction" INTEGER NOT NULL DEFAULT '0' CHECK ("RequiredMinRepFaction" >= 0),
    "RequiredMinRepValue" INTEGER NOT NULL DEFAULT '0',
    "RequiredMaxRepFaction" INTEGER NOT NULL DEFAULT '0' CHECK ("RequiredMaxRepFaction" >= 0),
    "RequiredMaxRepValue" INTEGER NOT NULL DEFAULT '0',
    "SuggestedPlayers" SMALLINT NOT NULL DEFAULT '0' CHECK ("SuggestedPlayers" >= 0),
    "LimitTime" BIGINT NOT NULL DEFAULT '0' CHECK ("LimitTime" >= 0),
    "QuestFlags" INTEGER NOT NULL DEFAULT '0' CHECK ("QuestFlags" >= 0),
    "SpecialFlags" SMALLINT NOT NULL DEFAULT '0' CHECK ("SpecialFlags" >= 0),
    "PrevQuestId" INTEGER NOT NULL DEFAULT '0',
    "NextQuestId" INTEGER NOT NULL DEFAULT '0',
    "ExclusiveGroup" INTEGER NOT NULL DEFAULT '0',
    "BreadcrumbForQuestId" BIGINT NOT NULL DEFAULT '0' CHECK ("BreadcrumbForQuestId" >= 0),
    "NextQuestInChain" BIGINT NOT NULL DEFAULT '0' CHECK ("NextQuestInChain" >= 0),
    "SrcItemId" BIGINT NOT NULL DEFAULT '0' CHECK ("SrcItemId" >= 0),
    "SrcItemCount" SMALLINT NOT NULL DEFAULT '0' CHECK ("SrcItemCount" >= 0),
    "SrcSpell" INTEGER NOT NULL DEFAULT '0' CHECK ("SrcSpell" >= 0),
    "Title" TEXT,
    "Details" TEXT,
    "Objectives" TEXT,
    "OfferRewardText" TEXT,
    "RequestItemsText" TEXT,
    "EndText" TEXT,
    "ObjectiveText1" TEXT,
    "ObjectiveText2" TEXT,
    "ObjectiveText3" TEXT,
    "ObjectiveText4" TEXT,
    "ReqItemId1" BIGINT NOT NULL DEFAULT '0' CHECK ("ReqItemId1" >= 0),
    "ReqItemId2" BIGINT NOT NULL DEFAULT '0' CHECK ("ReqItemId2" >= 0),
    "ReqItemId3" BIGINT NOT NULL DEFAULT '0' CHECK ("ReqItemId3" >= 0),
    "ReqItemId4" BIGINT NOT NULL DEFAULT '0' CHECK ("ReqItemId4" >= 0),
    "ReqItemCount1" INTEGER NOT NULL DEFAULT '0' CHECK ("ReqItemCount1" >= 0),
    "ReqItemCount2" INTEGER NOT NULL DEFAULT '0' CHECK ("ReqItemCount2" >= 0),
    "ReqItemCount3" INTEGER NOT NULL DEFAULT '0' CHECK ("ReqItemCount3" >= 0),
    "ReqItemCount4" INTEGER NOT NULL DEFAULT '0' CHECK ("ReqItemCount4" >= 0),
    "ReqSourceId1" BIGINT NOT NULL DEFAULT '0' CHECK ("ReqSourceId1" >= 0),
    "ReqSourceId2" BIGINT NOT NULL DEFAULT '0' CHECK ("ReqSourceId2" >= 0),
    "ReqSourceId3" BIGINT NOT NULL DEFAULT '0' CHECK ("ReqSourceId3" >= 0),
    "ReqSourceId4" BIGINT NOT NULL DEFAULT '0' CHECK ("ReqSourceId4" >= 0),
    "ReqSourceCount1" BIGINT NOT NULL DEFAULT '0' CHECK ("ReqSourceCount1" >= 0),
    "ReqSourceCount2" BIGINT NOT NULL DEFAULT '0' CHECK ("ReqSourceCount2" >= 0),
    "ReqSourceCount3" BIGINT NOT NULL DEFAULT '0' CHECK ("ReqSourceCount3" >= 0),
    "ReqSourceCount4" BIGINT NOT NULL DEFAULT '0' CHECK ("ReqSourceCount4" >= 0),
    "ReqCreatureOrGOId1" INTEGER NOT NULL DEFAULT '0',
    "ReqCreatureOrGOId2" INTEGER NOT NULL DEFAULT '0',
    "ReqCreatureOrGOId3" INTEGER NOT NULL DEFAULT '0',
    "ReqCreatureOrGOId4" INTEGER NOT NULL DEFAULT '0',
    "ReqCreatureOrGOCount1" INTEGER NOT NULL DEFAULT '0' CHECK ("ReqCreatureOrGOCount1" >= 0),
    "ReqCreatureOrGOCount2" INTEGER NOT NULL DEFAULT '0' CHECK ("ReqCreatureOrGOCount2" >= 0),
    "ReqCreatureOrGOCount3" INTEGER NOT NULL DEFAULT '0' CHECK ("ReqCreatureOrGOCount3" >= 0),
    "ReqCreatureOrGOCount4" INTEGER NOT NULL DEFAULT '0' CHECK ("ReqCreatureOrGOCount4" >= 0),
    "ReqSpellCast1" INTEGER NOT NULL DEFAULT '0' CHECK ("ReqSpellCast1" >= 0),
    "ReqSpellCast2" INTEGER NOT NULL DEFAULT '0' CHECK ("ReqSpellCast2" >= 0),
    "ReqSpellCast3" INTEGER NOT NULL DEFAULT '0' CHECK ("ReqSpellCast3" >= 0),
    "ReqSpellCast4" INTEGER NOT NULL DEFAULT '0' CHECK ("ReqSpellCast4" >= 0),
    "RewChoiceItemId1" BIGINT NOT NULL DEFAULT '0' CHECK ("RewChoiceItemId1" >= 0),
    "RewChoiceItemId2" BIGINT NOT NULL DEFAULT '0' CHECK ("RewChoiceItemId2" >= 0),
    "RewChoiceItemId3" BIGINT NOT NULL DEFAULT '0' CHECK ("RewChoiceItemId3" >= 0),
    "RewChoiceItemId4" BIGINT NOT NULL DEFAULT '0' CHECK ("RewChoiceItemId4" >= 0),
    "RewChoiceItemId5" BIGINT NOT NULL DEFAULT '0' CHECK ("RewChoiceItemId5" >= 0),
    "RewChoiceItemId6" BIGINT NOT NULL DEFAULT '0' CHECK ("RewChoiceItemId6" >= 0),
    "RewChoiceItemCount1" INTEGER NOT NULL DEFAULT '0' CHECK ("RewChoiceItemCount1" >= 0),
    "RewChoiceItemCount2" INTEGER NOT NULL DEFAULT '0' CHECK ("RewChoiceItemCount2" >= 0),
    "RewChoiceItemCount3" INTEGER NOT NULL DEFAULT '0' CHECK ("RewChoiceItemCount3" >= 0),
    "RewChoiceItemCount4" INTEGER NOT NULL DEFAULT '0' CHECK ("RewChoiceItemCount4" >= 0),
    "RewChoiceItemCount5" INTEGER NOT NULL DEFAULT '0' CHECK ("RewChoiceItemCount5" >= 0),
    "RewChoiceItemCount6" INTEGER NOT NULL DEFAULT '0' CHECK ("RewChoiceItemCount6" >= 0),
    "RewItemId1" BIGINT NOT NULL DEFAULT '0' CHECK ("RewItemId1" >= 0),
    "RewItemId2" BIGINT NOT NULL DEFAULT '0' CHECK ("RewItemId2" >= 0),
    "RewItemId3" BIGINT NOT NULL DEFAULT '0' CHECK ("RewItemId3" >= 0),
    "RewItemId4" BIGINT NOT NULL DEFAULT '0' CHECK ("RewItemId4" >= 0),
    "RewItemCount1" INTEGER NOT NULL DEFAULT '0' CHECK ("RewItemCount1" >= 0),
    "RewItemCount2" INTEGER NOT NULL DEFAULT '0' CHECK ("RewItemCount2" >= 0),
    "RewItemCount3" INTEGER NOT NULL DEFAULT '0' CHECK ("RewItemCount3" >= 0),
    "RewItemCount4" INTEGER NOT NULL DEFAULT '0' CHECK ("RewItemCount4" >= 0),
    "RewRepFaction1" INTEGER NOT NULL DEFAULT '0' CHECK ("RewRepFaction1" >= 0),
    "RewRepFaction2" INTEGER NOT NULL DEFAULT '0' CHECK ("RewRepFaction2" >= 0),
    "RewRepFaction3" INTEGER NOT NULL DEFAULT '0' CHECK ("RewRepFaction3" >= 0),
    "RewRepFaction4" INTEGER NOT NULL DEFAULT '0' CHECK ("RewRepFaction4" >= 0),
    "RewRepFaction5" INTEGER NOT NULL DEFAULT '0' CHECK ("RewRepFaction5" >= 0),
    "RewRepValue1" INTEGER NOT NULL DEFAULT '0',
    "RewRepValue2" INTEGER NOT NULL DEFAULT '0',
    "RewRepValue3" INTEGER NOT NULL DEFAULT '0',
    "RewRepValue4" INTEGER NOT NULL DEFAULT '0',
    "RewRepValue5" INTEGER NOT NULL DEFAULT '0',
    "RewRepSpilloverMask" SMALLINT NOT NULL DEFAULT '0' CHECK ("RewRepSpilloverMask" >= 0),
    "RewXP" BIGINT NOT NULL DEFAULT '0' CHECK ("RewXP" >= 0),
    "RewOrReqMoney" INTEGER NOT NULL DEFAULT '0',
    "RewMoneyMaxLevel" BIGINT NOT NULL DEFAULT '0' CHECK ("RewMoneyMaxLevel" >= 0),
    "RewSpell" INTEGER NOT NULL DEFAULT '0' CHECK ("RewSpell" >= 0),
    "RewSpellCast" INTEGER NOT NULL DEFAULT '0' CHECK ("RewSpellCast" >= 0),
    "RewMailTemplateId" INTEGER NOT NULL DEFAULT '0',
    "RewMailDelaySecs" BIGINT NOT NULL DEFAULT '0' CHECK ("RewMailDelaySecs" >= 0),
    "RewMailMoney" BIGINT NOT NULL DEFAULT '0' CHECK ("RewMailMoney" >= 0),
    "PointMapId" INTEGER NOT NULL DEFAULT '0' CHECK ("PointMapId" >= 0),
    "PointX" REAL NOT NULL DEFAULT '0',
    "PointY" REAL NOT NULL DEFAULT '0',
    "PointOpt" BIGINT NOT NULL DEFAULT '0' CHECK ("PointOpt" >= 0),
    "DetailsEmote1" INTEGER NOT NULL DEFAULT '0' CHECK ("DetailsEmote1" >= 0),
    "DetailsEmote2" INTEGER NOT NULL DEFAULT '0' CHECK ("DetailsEmote2" >= 0),
    "DetailsEmote3" INTEGER NOT NULL DEFAULT '0' CHECK ("DetailsEmote3" >= 0),
    "DetailsEmote4" INTEGER NOT NULL DEFAULT '0' CHECK ("DetailsEmote4" >= 0),
    "DetailsEmoteDelay1" BIGINT NOT NULL DEFAULT '0' CHECK ("DetailsEmoteDelay1" >= 0),
    "DetailsEmoteDelay2" BIGINT NOT NULL DEFAULT '0' CHECK ("DetailsEmoteDelay2" >= 0),
    "DetailsEmoteDelay3" BIGINT NOT NULL DEFAULT '0' CHECK ("DetailsEmoteDelay3" >= 0),
    "DetailsEmoteDelay4" BIGINT NOT NULL DEFAULT '0' CHECK ("DetailsEmoteDelay4" >= 0),
    "IncompleteEmote" INTEGER NOT NULL DEFAULT '0' CHECK ("IncompleteEmote" >= 0),
    "CompleteEmote" INTEGER NOT NULL DEFAULT '0' CHECK ("CompleteEmote" >= 0),
    "OfferRewardEmote1" INTEGER NOT NULL DEFAULT '0' CHECK ("OfferRewardEmote1" >= 0),
    "OfferRewardEmote2" INTEGER NOT NULL DEFAULT '0' CHECK ("OfferRewardEmote2" >= 0),
    "OfferRewardEmote3" INTEGER NOT NULL DEFAULT '0' CHECK ("OfferRewardEmote3" >= 0),
    "OfferRewardEmote4" INTEGER NOT NULL DEFAULT '0' CHECK ("OfferRewardEmote4" >= 0),
    "OfferRewardEmoteDelay1" BIGINT NOT NULL DEFAULT '0' CHECK ("OfferRewardEmoteDelay1" >= 0),
    "OfferRewardEmoteDelay2" BIGINT NOT NULL DEFAULT '0' CHECK ("OfferRewardEmoteDelay2" >= 0),
    "OfferRewardEmoteDelay3" BIGINT NOT NULL DEFAULT '0' CHECK ("OfferRewardEmoteDelay3" >= 0),
    "OfferRewardEmoteDelay4" BIGINT NOT NULL DEFAULT '0' CHECK ("OfferRewardEmoteDelay4" >= 0),
    "StartScript" BIGINT NOT NULL DEFAULT '0' CHECK ("StartScript" >= 0),
    "CompleteScript" BIGINT NOT NULL DEFAULT '0' CHECK ("CompleteScript" >= 0),
    PRIMARY KEY ("entry", "patch")
);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "patch" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "Method" SMALLINT NOT NULL DEFAULT '2' CHECK ("Method" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ZoneOrSort" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "MinLevel" SMALLINT NOT NULL DEFAULT '0' CHECK ("MinLevel" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "MaxLevel" SMALLINT NOT NULL DEFAULT '0' CHECK ("MaxLevel" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "QuestLevel" SMALLINT NOT NULL DEFAULT '0' CHECK ("QuestLevel" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "Type" INTEGER NOT NULL DEFAULT '0' CHECK ("Type" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RequiredClasses" INTEGER NOT NULL DEFAULT '0' CHECK ("RequiredClasses" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RequiredRaces" INTEGER NOT NULL DEFAULT '0' CHECK ("RequiredRaces" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RequiredSkill" INTEGER NOT NULL DEFAULT '0' CHECK ("RequiredSkill" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RequiredSkillValue" INTEGER NOT NULL DEFAULT '0' CHECK ("RequiredSkillValue" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RequiredCondition" BIGINT NOT NULL DEFAULT '0' CHECK ("RequiredCondition" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RepObjectiveFaction" INTEGER NOT NULL DEFAULT '0' CHECK ("RepObjectiveFaction" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RepObjectiveValue" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RequiredMinRepFaction" INTEGER NOT NULL DEFAULT '0' CHECK ("RequiredMinRepFaction" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RequiredMinRepValue" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RequiredMaxRepFaction" INTEGER NOT NULL DEFAULT '0' CHECK ("RequiredMaxRepFaction" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RequiredMaxRepValue" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "SuggestedPlayers" SMALLINT NOT NULL DEFAULT '0' CHECK ("SuggestedPlayers" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "LimitTime" BIGINT NOT NULL DEFAULT '0' CHECK ("LimitTime" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "QuestFlags" INTEGER NOT NULL DEFAULT '0' CHECK ("QuestFlags" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "SpecialFlags" SMALLINT NOT NULL DEFAULT '0' CHECK ("SpecialFlags" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "PrevQuestId" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "NextQuestId" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ExclusiveGroup" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "BreadcrumbForQuestId" BIGINT NOT NULL DEFAULT '0' CHECK ("BreadcrumbForQuestId" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "NextQuestInChain" BIGINT NOT NULL DEFAULT '0' CHECK ("NextQuestInChain" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "SrcItemId" BIGINT NOT NULL DEFAULT '0' CHECK ("SrcItemId" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "SrcItemCount" SMALLINT NOT NULL DEFAULT '0' CHECK ("SrcItemCount" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "SrcSpell" INTEGER NOT NULL DEFAULT '0' CHECK ("SrcSpell" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "Title" TEXT;
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "Details" TEXT;
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "Objectives" TEXT;
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "OfferRewardText" TEXT;
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RequestItemsText" TEXT;
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "EndText" TEXT;
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ObjectiveText1" TEXT;
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ObjectiveText2" TEXT;
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ObjectiveText3" TEXT;
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ObjectiveText4" TEXT;
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqItemId1" BIGINT NOT NULL DEFAULT '0' CHECK ("ReqItemId1" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqItemId2" BIGINT NOT NULL DEFAULT '0' CHECK ("ReqItemId2" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqItemId3" BIGINT NOT NULL DEFAULT '0' CHECK ("ReqItemId3" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqItemId4" BIGINT NOT NULL DEFAULT '0' CHECK ("ReqItemId4" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqItemCount1" INTEGER NOT NULL DEFAULT '0' CHECK ("ReqItemCount1" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqItemCount2" INTEGER NOT NULL DEFAULT '0' CHECK ("ReqItemCount2" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqItemCount3" INTEGER NOT NULL DEFAULT '0' CHECK ("ReqItemCount3" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqItemCount4" INTEGER NOT NULL DEFAULT '0' CHECK ("ReqItemCount4" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqSourceId1" BIGINT NOT NULL DEFAULT '0' CHECK ("ReqSourceId1" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqSourceId2" BIGINT NOT NULL DEFAULT '0' CHECK ("ReqSourceId2" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqSourceId3" BIGINT NOT NULL DEFAULT '0' CHECK ("ReqSourceId3" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqSourceId4" BIGINT NOT NULL DEFAULT '0' CHECK ("ReqSourceId4" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqSourceCount1" BIGINT NOT NULL DEFAULT '0' CHECK ("ReqSourceCount1" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqSourceCount2" BIGINT NOT NULL DEFAULT '0' CHECK ("ReqSourceCount2" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqSourceCount3" BIGINT NOT NULL DEFAULT '0' CHECK ("ReqSourceCount3" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqSourceCount4" BIGINT NOT NULL DEFAULT '0' CHECK ("ReqSourceCount4" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqCreatureOrGOId1" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqCreatureOrGOId2" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqCreatureOrGOId3" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqCreatureOrGOId4" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqCreatureOrGOCount1" INTEGER NOT NULL DEFAULT '0' CHECK ("ReqCreatureOrGOCount1" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqCreatureOrGOCount2" INTEGER NOT NULL DEFAULT '0' CHECK ("ReqCreatureOrGOCount2" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqCreatureOrGOCount3" INTEGER NOT NULL DEFAULT '0' CHECK ("ReqCreatureOrGOCount3" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqCreatureOrGOCount4" INTEGER NOT NULL DEFAULT '0' CHECK ("ReqCreatureOrGOCount4" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqSpellCast1" INTEGER NOT NULL DEFAULT '0' CHECK ("ReqSpellCast1" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqSpellCast2" INTEGER NOT NULL DEFAULT '0' CHECK ("ReqSpellCast2" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqSpellCast3" INTEGER NOT NULL DEFAULT '0' CHECK ("ReqSpellCast3" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "ReqSpellCast4" INTEGER NOT NULL DEFAULT '0' CHECK ("ReqSpellCast4" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewChoiceItemId1" BIGINT NOT NULL DEFAULT '0' CHECK ("RewChoiceItemId1" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewChoiceItemId2" BIGINT NOT NULL DEFAULT '0' CHECK ("RewChoiceItemId2" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewChoiceItemId3" BIGINT NOT NULL DEFAULT '0' CHECK ("RewChoiceItemId3" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewChoiceItemId4" BIGINT NOT NULL DEFAULT '0' CHECK ("RewChoiceItemId4" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewChoiceItemId5" BIGINT NOT NULL DEFAULT '0' CHECK ("RewChoiceItemId5" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewChoiceItemId6" BIGINT NOT NULL DEFAULT '0' CHECK ("RewChoiceItemId6" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewChoiceItemCount1" INTEGER NOT NULL DEFAULT '0' CHECK ("RewChoiceItemCount1" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewChoiceItemCount2" INTEGER NOT NULL DEFAULT '0' CHECK ("RewChoiceItemCount2" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewChoiceItemCount3" INTEGER NOT NULL DEFAULT '0' CHECK ("RewChoiceItemCount3" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewChoiceItemCount4" INTEGER NOT NULL DEFAULT '0' CHECK ("RewChoiceItemCount4" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewChoiceItemCount5" INTEGER NOT NULL DEFAULT '0' CHECK ("RewChoiceItemCount5" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewChoiceItemCount6" INTEGER NOT NULL DEFAULT '0' CHECK ("RewChoiceItemCount6" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewItemId1" BIGINT NOT NULL DEFAULT '0' CHECK ("RewItemId1" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewItemId2" BIGINT NOT NULL DEFAULT '0' CHECK ("RewItemId2" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewItemId3" BIGINT NOT NULL DEFAULT '0' CHECK ("RewItemId3" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewItemId4" BIGINT NOT NULL DEFAULT '0' CHECK ("RewItemId4" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewItemCount1" INTEGER NOT NULL DEFAULT '0' CHECK ("RewItemCount1" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewItemCount2" INTEGER NOT NULL DEFAULT '0' CHECK ("RewItemCount2" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewItemCount3" INTEGER NOT NULL DEFAULT '0' CHECK ("RewItemCount3" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewItemCount4" INTEGER NOT NULL DEFAULT '0' CHECK ("RewItemCount4" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewRepFaction1" INTEGER NOT NULL DEFAULT '0' CHECK ("RewRepFaction1" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewRepFaction2" INTEGER NOT NULL DEFAULT '0' CHECK ("RewRepFaction2" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewRepFaction3" INTEGER NOT NULL DEFAULT '0' CHECK ("RewRepFaction3" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewRepFaction4" INTEGER NOT NULL DEFAULT '0' CHECK ("RewRepFaction4" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewRepFaction5" INTEGER NOT NULL DEFAULT '0' CHECK ("RewRepFaction5" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewRepValue1" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewRepValue2" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewRepValue3" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewRepValue4" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewRepValue5" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewRepSpilloverMask" SMALLINT NOT NULL DEFAULT '0' CHECK ("RewRepSpilloverMask" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewXP" BIGINT NOT NULL DEFAULT '0' CHECK ("RewXP" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewOrReqMoney" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewMoneyMaxLevel" BIGINT NOT NULL DEFAULT '0' CHECK ("RewMoneyMaxLevel" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewSpell" INTEGER NOT NULL DEFAULT '0' CHECK ("RewSpell" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewSpellCast" INTEGER NOT NULL DEFAULT '0' CHECK ("RewSpellCast" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewMailTemplateId" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewMailDelaySecs" BIGINT NOT NULL DEFAULT '0' CHECK ("RewMailDelaySecs" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "RewMailMoney" BIGINT NOT NULL DEFAULT '0' CHECK ("RewMailMoney" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "PointMapId" INTEGER NOT NULL DEFAULT '0' CHECK ("PointMapId" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "PointX" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "PointY" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "PointOpt" BIGINT NOT NULL DEFAULT '0' CHECK ("PointOpt" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "DetailsEmote1" INTEGER NOT NULL DEFAULT '0' CHECK ("DetailsEmote1" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "DetailsEmote2" INTEGER NOT NULL DEFAULT '0' CHECK ("DetailsEmote2" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "DetailsEmote3" INTEGER NOT NULL DEFAULT '0' CHECK ("DetailsEmote3" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "DetailsEmote4" INTEGER NOT NULL DEFAULT '0' CHECK ("DetailsEmote4" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "DetailsEmoteDelay1" BIGINT NOT NULL DEFAULT '0' CHECK ("DetailsEmoteDelay1" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "DetailsEmoteDelay2" BIGINT NOT NULL DEFAULT '0' CHECK ("DetailsEmoteDelay2" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "DetailsEmoteDelay3" BIGINT NOT NULL DEFAULT '0' CHECK ("DetailsEmoteDelay3" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "DetailsEmoteDelay4" BIGINT NOT NULL DEFAULT '0' CHECK ("DetailsEmoteDelay4" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "IncompleteEmote" INTEGER NOT NULL DEFAULT '0' CHECK ("IncompleteEmote" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "CompleteEmote" INTEGER NOT NULL DEFAULT '0' CHECK ("CompleteEmote" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "OfferRewardEmote1" INTEGER NOT NULL DEFAULT '0' CHECK ("OfferRewardEmote1" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "OfferRewardEmote2" INTEGER NOT NULL DEFAULT '0' CHECK ("OfferRewardEmote2" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "OfferRewardEmote3" INTEGER NOT NULL DEFAULT '0' CHECK ("OfferRewardEmote3" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "OfferRewardEmote4" INTEGER NOT NULL DEFAULT '0' CHECK ("OfferRewardEmote4" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "OfferRewardEmoteDelay1" BIGINT NOT NULL DEFAULT '0' CHECK ("OfferRewardEmoteDelay1" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "OfferRewardEmoteDelay2" BIGINT NOT NULL DEFAULT '0' CHECK ("OfferRewardEmoteDelay2" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "OfferRewardEmoteDelay3" BIGINT NOT NULL DEFAULT '0' CHECK ("OfferRewardEmoteDelay3" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "OfferRewardEmoteDelay4" BIGINT NOT NULL DEFAULT '0' CHECK ("OfferRewardEmoteDelay4" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "StartScript" BIGINT NOT NULL DEFAULT '0' CHECK ("StartScript" >= 0);
ALTER TABLE IF EXISTS "quest_template" ADD COLUMN IF NOT EXISTS "CompleteScript" BIGINT NOT NULL DEFAULT '0' CHECK ("CompleteScript" >= 0);

CREATE TABLE IF NOT EXISTS "reference_loot_template" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "item" BIGINT NOT NULL DEFAULT '0' CHECK ("item" >= 0),
    "ChanceOrQuestChance" REAL NOT NULL DEFAULT '100',
    "groupid" SMALLINT NOT NULL DEFAULT '0' CHECK ("groupid" >= 0),
    "mincountOrRef" INTEGER NOT NULL DEFAULT '1',
    "maxcount" SMALLINT NOT NULL DEFAULT '1' CHECK ("maxcount" >= 0),
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0),
    PRIMARY KEY ("entry", "item", "patch_min", "patch_max")
);
ALTER TABLE IF EXISTS "reference_loot_template" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "reference_loot_template" ADD COLUMN IF NOT EXISTS "item" BIGINT NOT NULL DEFAULT '0' CHECK ("item" >= 0);
ALTER TABLE IF EXISTS "reference_loot_template" ADD COLUMN IF NOT EXISTS "ChanceOrQuestChance" REAL NOT NULL DEFAULT '100';
ALTER TABLE IF EXISTS "reference_loot_template" ADD COLUMN IF NOT EXISTS "groupid" SMALLINT NOT NULL DEFAULT '0' CHECK ("groupid" >= 0);
ALTER TABLE IF EXISTS "reference_loot_template" ADD COLUMN IF NOT EXISTS "mincountOrRef" INTEGER NOT NULL DEFAULT '1';
ALTER TABLE IF EXISTS "reference_loot_template" ADD COLUMN IF NOT EXISTS "maxcount" SMALLINT NOT NULL DEFAULT '1' CHECK ("maxcount" >= 0);
ALTER TABLE IF EXISTS "reference_loot_template" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);
ALTER TABLE IF EXISTS "reference_loot_template" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);
ALTER TABLE IF EXISTS "reference_loot_template" ADD COLUMN IF NOT EXISTS "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0);

CREATE TABLE IF NOT EXISTS "reputation_reward_rate" (
    "faction" BIGINT NOT NULL DEFAULT '0' CHECK ("faction" >= 0),
    "quest_rate" REAL NOT NULL DEFAULT '1',
    "creature_rate" REAL NOT NULL DEFAULT '1',
    "spell_rate" REAL NOT NULL DEFAULT '1',
    PRIMARY KEY ("faction")
);
ALTER TABLE IF EXISTS "reputation_reward_rate" ADD COLUMN IF NOT EXISTS "faction" BIGINT NOT NULL DEFAULT '0' CHECK ("faction" >= 0);
ALTER TABLE IF EXISTS "reputation_reward_rate" ADD COLUMN IF NOT EXISTS "quest_rate" REAL NOT NULL DEFAULT '1';
ALTER TABLE IF EXISTS "reputation_reward_rate" ADD COLUMN IF NOT EXISTS "creature_rate" REAL NOT NULL DEFAULT '1';
ALTER TABLE IF EXISTS "reputation_reward_rate" ADD COLUMN IF NOT EXISTS "spell_rate" REAL NOT NULL DEFAULT '1';

CREATE TABLE IF NOT EXISTS "reputation_spillover_template" (
    "faction" INTEGER NOT NULL DEFAULT '0' CHECK ("faction" >= 0),
    "faction1" INTEGER NOT NULL DEFAULT '0' CHECK ("faction1" >= 0),
    "rate_1" REAL NOT NULL DEFAULT '0',
    "rank_1" SMALLINT NOT NULL DEFAULT '0' CHECK ("rank_1" >= 0),
    "faction2" INTEGER NOT NULL DEFAULT '0' CHECK ("faction2" >= 0),
    "rate_2" REAL NOT NULL DEFAULT '0',
    "rank_2" SMALLINT NOT NULL DEFAULT '0' CHECK ("rank_2" >= 0),
    "faction3" INTEGER NOT NULL DEFAULT '0' CHECK ("faction3" >= 0),
    "rate_3" REAL NOT NULL DEFAULT '0',
    "rank_3" SMALLINT NOT NULL DEFAULT '0' CHECK ("rank_3" >= 0),
    "faction4" INTEGER NOT NULL DEFAULT '0' CHECK ("faction4" >= 0),
    "rate_4" REAL NOT NULL DEFAULT '0',
    "rank_4" SMALLINT NOT NULL DEFAULT '0' CHECK ("rank_4" >= 0),
    PRIMARY KEY ("faction")
);
ALTER TABLE IF EXISTS "reputation_spillover_template" ADD COLUMN IF NOT EXISTS "faction" INTEGER NOT NULL DEFAULT '0' CHECK ("faction" >= 0);
ALTER TABLE IF EXISTS "reputation_spillover_template" ADD COLUMN IF NOT EXISTS "faction1" INTEGER NOT NULL DEFAULT '0' CHECK ("faction1" >= 0);
ALTER TABLE IF EXISTS "reputation_spillover_template" ADD COLUMN IF NOT EXISTS "rate_1" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "reputation_spillover_template" ADD COLUMN IF NOT EXISTS "rank_1" SMALLINT NOT NULL DEFAULT '0' CHECK ("rank_1" >= 0);
ALTER TABLE IF EXISTS "reputation_spillover_template" ADD COLUMN IF NOT EXISTS "faction2" INTEGER NOT NULL DEFAULT '0' CHECK ("faction2" >= 0);
ALTER TABLE IF EXISTS "reputation_spillover_template" ADD COLUMN IF NOT EXISTS "rate_2" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "reputation_spillover_template" ADD COLUMN IF NOT EXISTS "rank_2" SMALLINT NOT NULL DEFAULT '0' CHECK ("rank_2" >= 0);
ALTER TABLE IF EXISTS "reputation_spillover_template" ADD COLUMN IF NOT EXISTS "faction3" INTEGER NOT NULL DEFAULT '0' CHECK ("faction3" >= 0);
ALTER TABLE IF EXISTS "reputation_spillover_template" ADD COLUMN IF NOT EXISTS "rate_3" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "reputation_spillover_template" ADD COLUMN IF NOT EXISTS "rank_3" SMALLINT NOT NULL DEFAULT '0' CHECK ("rank_3" >= 0);
ALTER TABLE IF EXISTS "reputation_spillover_template" ADD COLUMN IF NOT EXISTS "faction4" INTEGER NOT NULL DEFAULT '0' CHECK ("faction4" >= 0);
ALTER TABLE IF EXISTS "reputation_spillover_template" ADD COLUMN IF NOT EXISTS "rate_4" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "reputation_spillover_template" ADD COLUMN IF NOT EXISTS "rank_4" SMALLINT NOT NULL DEFAULT '0' CHECK ("rank_4" >= 0);

CREATE TABLE IF NOT EXISTS "reserved_name" (
    "name" VARCHAR(12) NOT NULL DEFAULT '',
    PRIMARY KEY ("name")
);
ALTER TABLE IF EXISTS "reserved_name" ADD COLUMN IF NOT EXISTS "name" VARCHAR(12) NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS "script_escort_data" (
    "creature_id" INTEGER DEFAULT NULL,
    "quest" INTEGER DEFAULT NULL,
    "escort_faction" INTEGER DEFAULT NULL,
    UNIQUE ("creature_id")
);
ALTER TABLE IF EXISTS "script_escort_data" ADD COLUMN IF NOT EXISTS "creature_id" INTEGER DEFAULT NULL;
ALTER TABLE IF EXISTS "script_escort_data" ADD COLUMN IF NOT EXISTS "quest" INTEGER DEFAULT NULL;
ALTER TABLE IF EXISTS "script_escort_data" ADD COLUMN IF NOT EXISTS "escort_faction" INTEGER DEFAULT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_script_escort_data_creature_id ON "script_escort_data" ("creature_id");

CREATE TABLE IF NOT EXISTS "script_texts" (
    "entry" INTEGER NOT NULL,
    "content_default" TEXT NOT NULL,
    "content_loc1" TEXT,
    "content_loc2" TEXT,
    "content_loc3" TEXT,
    "content_loc4" TEXT,
    "content_loc5" TEXT,
    "content_loc6" TEXT,
    "content_loc7" TEXT,
    "content_loc8" TEXT,
    "sound" BIGINT NOT NULL DEFAULT '0' CHECK ("sound" >= 0),
    "type" SMALLINT NOT NULL DEFAULT '0' CHECK ("type" >= 0),
    "language" SMALLINT NOT NULL DEFAULT '0' CHECK ("language" >= 0),
    "emote" INTEGER NOT NULL DEFAULT '0' CHECK ("emote" >= 0),
    "comment" TEXT,
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "script_texts" ADD COLUMN IF NOT EXISTS "entry" INTEGER NOT NULL;
ALTER TABLE IF EXISTS "script_texts" ADD COLUMN IF NOT EXISTS "content_default" TEXT NOT NULL;
ALTER TABLE IF EXISTS "script_texts" ADD COLUMN IF NOT EXISTS "content_loc1" TEXT;
ALTER TABLE IF EXISTS "script_texts" ADD COLUMN IF NOT EXISTS "content_loc2" TEXT;
ALTER TABLE IF EXISTS "script_texts" ADD COLUMN IF NOT EXISTS "content_loc3" TEXT;
ALTER TABLE IF EXISTS "script_texts" ADD COLUMN IF NOT EXISTS "content_loc4" TEXT;
ALTER TABLE IF EXISTS "script_texts" ADD COLUMN IF NOT EXISTS "content_loc5" TEXT;
ALTER TABLE IF EXISTS "script_texts" ADD COLUMN IF NOT EXISTS "content_loc6" TEXT;
ALTER TABLE IF EXISTS "script_texts" ADD COLUMN IF NOT EXISTS "content_loc7" TEXT;
ALTER TABLE IF EXISTS "script_texts" ADD COLUMN IF NOT EXISTS "content_loc8" TEXT;
ALTER TABLE IF EXISTS "script_texts" ADD COLUMN IF NOT EXISTS "sound" BIGINT NOT NULL DEFAULT '0' CHECK ("sound" >= 0);
ALTER TABLE IF EXISTS "script_texts" ADD COLUMN IF NOT EXISTS "type" SMALLINT NOT NULL DEFAULT '0' CHECK ("type" >= 0);
ALTER TABLE IF EXISTS "script_texts" ADD COLUMN IF NOT EXISTS "language" SMALLINT NOT NULL DEFAULT '0' CHECK ("language" >= 0);
ALTER TABLE IF EXISTS "script_texts" ADD COLUMN IF NOT EXISTS "emote" INTEGER NOT NULL DEFAULT '0' CHECK ("emote" >= 0);
ALTER TABLE IF EXISTS "script_texts" ADD COLUMN IF NOT EXISTS "comment" TEXT;

CREATE TABLE IF NOT EXISTS "script_waypoint" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "pointid" BIGINT NOT NULL DEFAULT '0' CHECK ("pointid" >= 0),
    "location_x" REAL NOT NULL DEFAULT '0',
    "location_y" REAL NOT NULL DEFAULT '0',
    "location_z" REAL NOT NULL DEFAULT '0',
    "waittime" BIGINT NOT NULL DEFAULT '0' CHECK ("waittime" >= 0),
    "point_comment" TEXT,
    PRIMARY KEY ("entry", "pointid")
);
ALTER TABLE IF EXISTS "script_waypoint" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "script_waypoint" ADD COLUMN IF NOT EXISTS "pointid" BIGINT NOT NULL DEFAULT '0' CHECK ("pointid" >= 0);
ALTER TABLE IF EXISTS "script_waypoint" ADD COLUMN IF NOT EXISTS "location_x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "script_waypoint" ADD COLUMN IF NOT EXISTS "location_y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "script_waypoint" ADD COLUMN IF NOT EXISTS "location_z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "script_waypoint" ADD COLUMN IF NOT EXISTS "waittime" BIGINT NOT NULL DEFAULT '0' CHECK ("waittime" >= 0);
ALTER TABLE IF EXISTS "script_waypoint" ADD COLUMN IF NOT EXISTS "point_comment" TEXT;

CREATE TABLE IF NOT EXISTS "scripted_event_id" (
    "id" INTEGER NOT NULL,
    "script_name" CHAR(64) NOT NULL,
    PRIMARY KEY ("id")
);
ALTER TABLE IF EXISTS "scripted_event_id" ADD COLUMN IF NOT EXISTS "id" INTEGER NOT NULL;
ALTER TABLE IF EXISTS "scripted_event_id" ADD COLUMN IF NOT EXISTS "script_name" CHAR(64) NOT NULL;

CREATE TABLE IF NOT EXISTS "skill_fishing_base_level" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "skill" SMALLINT NOT NULL DEFAULT '0',
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "skill_fishing_base_level" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "skill_fishing_base_level" ADD COLUMN IF NOT EXISTS "skill" SMALLINT NOT NULL DEFAULT '0';

CREATE TABLE IF NOT EXISTS "skill_line_ability" (
    "id" INTEGER NOT NULL CHECK ("id" >= 0),
    "build" INTEGER NOT NULL CHECK ("build" >= 0),
    "skill_id" BIGINT NOT NULL DEFAULT '0' CHECK ("skill_id" >= 0),
    "spell_id" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_id" >= 0),
    "race_mask" BIGINT NOT NULL DEFAULT '0' CHECK ("race_mask" >= 0),
    "class_mask" BIGINT NOT NULL DEFAULT '0' CHECK ("class_mask" >= 0),
    "req_skill_value" BIGINT NOT NULL DEFAULT '0' CHECK ("req_skill_value" >= 0),
    "superseded_by_spell" INTEGER NOT NULL DEFAULT '0' CHECK ("superseded_by_spell" >= 0),
    "learn_on_get_skill" BIGINT NOT NULL DEFAULT '0' CHECK ("learn_on_get_skill" >= 0),
    "max_value" BIGINT NOT NULL DEFAULT '0' CHECK ("max_value" >= 0),
    "min_value" BIGINT NOT NULL DEFAULT '0' CHECK ("min_value" >= 0),
    "req_train_points" BIGINT NOT NULL DEFAULT '0' CHECK ("req_train_points" >= 0),
    PRIMARY KEY ("id", "build")
);
ALTER TABLE IF EXISTS "skill_line_ability" ADD COLUMN IF NOT EXISTS "id" INTEGER NOT NULL CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "skill_line_ability" ADD COLUMN IF NOT EXISTS "build" INTEGER NOT NULL CHECK ("build" >= 0);
ALTER TABLE IF EXISTS "skill_line_ability" ADD COLUMN IF NOT EXISTS "skill_id" BIGINT NOT NULL DEFAULT '0' CHECK ("skill_id" >= 0);
ALTER TABLE IF EXISTS "skill_line_ability" ADD COLUMN IF NOT EXISTS "spell_id" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_id" >= 0);
ALTER TABLE IF EXISTS "skill_line_ability" ADD COLUMN IF NOT EXISTS "race_mask" BIGINT NOT NULL DEFAULT '0' CHECK ("race_mask" >= 0);
ALTER TABLE IF EXISTS "skill_line_ability" ADD COLUMN IF NOT EXISTS "class_mask" BIGINT NOT NULL DEFAULT '0' CHECK ("class_mask" >= 0);
ALTER TABLE IF EXISTS "skill_line_ability" ADD COLUMN IF NOT EXISTS "req_skill_value" BIGINT NOT NULL DEFAULT '0' CHECK ("req_skill_value" >= 0);
ALTER TABLE IF EXISTS "skill_line_ability" ADD COLUMN IF NOT EXISTS "superseded_by_spell" INTEGER NOT NULL DEFAULT '0' CHECK ("superseded_by_spell" >= 0);
ALTER TABLE IF EXISTS "skill_line_ability" ADD COLUMN IF NOT EXISTS "learn_on_get_skill" BIGINT NOT NULL DEFAULT '0' CHECK ("learn_on_get_skill" >= 0);
ALTER TABLE IF EXISTS "skill_line_ability" ADD COLUMN IF NOT EXISTS "max_value" BIGINT NOT NULL DEFAULT '0' CHECK ("max_value" >= 0);
ALTER TABLE IF EXISTS "skill_line_ability" ADD COLUMN IF NOT EXISTS "min_value" BIGINT NOT NULL DEFAULT '0' CHECK ("min_value" >= 0);
ALTER TABLE IF EXISTS "skill_line_ability" ADD COLUMN IF NOT EXISTS "req_train_points" BIGINT NOT NULL DEFAULT '0' CHECK ("req_train_points" >= 0);

CREATE TABLE IF NOT EXISTS "skinning_loot_template" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "item" BIGINT NOT NULL DEFAULT '0' CHECK ("item" >= 0),
    "ChanceOrQuestChance" REAL NOT NULL DEFAULT '100',
    "groupid" SMALLINT NOT NULL DEFAULT '0' CHECK ("groupid" >= 0),
    "mincountOrRef" INTEGER NOT NULL DEFAULT '1',
    "maxcount" SMALLINT NOT NULL DEFAULT '1' CHECK ("maxcount" >= 0),
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0),
    "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0),
    PRIMARY KEY ("entry", "item", "patch_min", "patch_max")
);
ALTER TABLE IF EXISTS "skinning_loot_template" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "skinning_loot_template" ADD COLUMN IF NOT EXISTS "item" BIGINT NOT NULL DEFAULT '0' CHECK ("item" >= 0);
ALTER TABLE IF EXISTS "skinning_loot_template" ADD COLUMN IF NOT EXISTS "ChanceOrQuestChance" REAL NOT NULL DEFAULT '100';
ALTER TABLE IF EXISTS "skinning_loot_template" ADD COLUMN IF NOT EXISTS "groupid" SMALLINT NOT NULL DEFAULT '0' CHECK ("groupid" >= 0);
ALTER TABLE IF EXISTS "skinning_loot_template" ADD COLUMN IF NOT EXISTS "mincountOrRef" INTEGER NOT NULL DEFAULT '1';
ALTER TABLE IF EXISTS "skinning_loot_template" ADD COLUMN IF NOT EXISTS "maxcount" SMALLINT NOT NULL DEFAULT '1' CHECK ("maxcount" >= 0);
ALTER TABLE IF EXISTS "skinning_loot_template" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);
ALTER TABLE IF EXISTS "skinning_loot_template" ADD COLUMN IF NOT EXISTS "patch_min" SMALLINT NOT NULL DEFAULT '0' CHECK ("patch_min" >= 0);
ALTER TABLE IF EXISTS "skinning_loot_template" ADD COLUMN IF NOT EXISTS "patch_max" SMALLINT NOT NULL DEFAULT '10' CHECK ("patch_max" >= 0);

CREATE TABLE IF NOT EXISTS "sound_entries" (
    "id" SMALLINT NOT NULL DEFAULT '0',
    "name" VARCHAR(128) NOT NULL DEFAULT '',
    PRIMARY KEY ("id"),
    UNIQUE ("id")
);
ALTER TABLE IF EXISTS "sound_entries" ADD COLUMN IF NOT EXISTS "id" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "sound_entries" ADD COLUMN IF NOT EXISTS "name" VARCHAR(128) NOT NULL DEFAULT '';
CREATE UNIQUE INDEX IF NOT EXISTS idx_sound_entries_id ON "sound_entries" ("id");

CREATE TABLE IF NOT EXISTS "spell_area" (
    "spell" INTEGER NOT NULL DEFAULT '0' CHECK ("spell" >= 0),
    "area" BIGINT NOT NULL DEFAULT '0' CHECK ("area" >= 0),
    "quest_start" BIGINT NOT NULL DEFAULT '0' CHECK ("quest_start" >= 0),
    "quest_start_active" SMALLINT NOT NULL DEFAULT '0' CHECK ("quest_start_active" >= 0),
    "quest_end" BIGINT NOT NULL DEFAULT '0' CHECK ("quest_end" >= 0),
    "aura_spell" SMALLINT NOT NULL DEFAULT '0',
    "racemask" BIGINT NOT NULL DEFAULT '0' CHECK ("racemask" >= 0),
    "gender" SMALLINT NOT NULL DEFAULT '2' CHECK ("gender" >= 0),
    "autocast" SMALLINT NOT NULL DEFAULT '0' CHECK ("autocast" >= 0),
    PRIMARY KEY ("spell", "area", "quest_start", "quest_start_active", "aura_spell", "racemask", "gender")
);
ALTER TABLE IF EXISTS "spell_area" ADD COLUMN IF NOT EXISTS "spell" INTEGER NOT NULL DEFAULT '0' CHECK ("spell" >= 0);
ALTER TABLE IF EXISTS "spell_area" ADD COLUMN IF NOT EXISTS "area" BIGINT NOT NULL DEFAULT '0' CHECK ("area" >= 0);
ALTER TABLE IF EXISTS "spell_area" ADD COLUMN IF NOT EXISTS "quest_start" BIGINT NOT NULL DEFAULT '0' CHECK ("quest_start" >= 0);
ALTER TABLE IF EXISTS "spell_area" ADD COLUMN IF NOT EXISTS "quest_start_active" SMALLINT NOT NULL DEFAULT '0' CHECK ("quest_start_active" >= 0);
ALTER TABLE IF EXISTS "spell_area" ADD COLUMN IF NOT EXISTS "quest_end" BIGINT NOT NULL DEFAULT '0' CHECK ("quest_end" >= 0);
ALTER TABLE IF EXISTS "spell_area" ADD COLUMN IF NOT EXISTS "aura_spell" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_area" ADD COLUMN IF NOT EXISTS "racemask" BIGINT NOT NULL DEFAULT '0' CHECK ("racemask" >= 0);
ALTER TABLE IF EXISTS "spell_area" ADD COLUMN IF NOT EXISTS "gender" SMALLINT NOT NULL DEFAULT '2' CHECK ("gender" >= 0);
ALTER TABLE IF EXISTS "spell_area" ADD COLUMN IF NOT EXISTS "autocast" SMALLINT NOT NULL DEFAULT '0' CHECK ("autocast" >= 0);

CREATE TABLE IF NOT EXISTS "spell_chain" (
    "spell_id" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_id" >= 0),
    "prev_spell" INTEGER NOT NULL DEFAULT '0' CHECK ("prev_spell" >= 0),
    "first_spell" INTEGER NOT NULL DEFAULT '0' CHECK ("first_spell" >= 0),
    "rank" SMALLINT NOT NULL DEFAULT '0',
    "req_spell" INTEGER NOT NULL DEFAULT '0' CHECK ("req_spell" >= 0),
    "build_min" SMALLINT NOT NULL DEFAULT '0',
    "build_max" SMALLINT NOT NULL DEFAULT '5875',
    PRIMARY KEY ("spell_id", "build_min", "build_max")
);
ALTER TABLE IF EXISTS "spell_chain" ADD COLUMN IF NOT EXISTS "spell_id" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_id" >= 0);
ALTER TABLE IF EXISTS "spell_chain" ADD COLUMN IF NOT EXISTS "prev_spell" INTEGER NOT NULL DEFAULT '0' CHECK ("prev_spell" >= 0);
ALTER TABLE IF EXISTS "spell_chain" ADD COLUMN IF NOT EXISTS "first_spell" INTEGER NOT NULL DEFAULT '0' CHECK ("first_spell" >= 0);
ALTER TABLE IF EXISTS "spell_chain" ADD COLUMN IF NOT EXISTS "rank" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_chain" ADD COLUMN IF NOT EXISTS "req_spell" INTEGER NOT NULL DEFAULT '0' CHECK ("req_spell" >= 0);
ALTER TABLE IF EXISTS "spell_chain" ADD COLUMN IF NOT EXISTS "build_min" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_chain" ADD COLUMN IF NOT EXISTS "build_max" SMALLINT NOT NULL DEFAULT '5875';

CREATE TABLE IF NOT EXISTS "spell_check" (
    "spellid" INTEGER NOT NULL DEFAULT '0' CHECK ("spellid" >= 0),
    "SpellFamilyName" SMALLINT NOT NULL DEFAULT '-1',
    "SpellFamilyMask" BIGINT NOT NULL DEFAULT '-1',
    "SpellIcon" INTEGER NOT NULL DEFAULT '-1',
    "SpellVisual" INTEGER NOT NULL DEFAULT '-1',
    "SpellCategory" INTEGER NOT NULL DEFAULT '-1',
    "EffectType" INTEGER NOT NULL DEFAULT '-1',
    "EffectAura" INTEGER NOT NULL DEFAULT '-1',
    "EffectIdx" SMALLINT NOT NULL DEFAULT '-1',
    "Name" VARCHAR(40) NOT NULL DEFAULT '',
    "Code" VARCHAR(40) NOT NULL DEFAULT '',
    PRIMARY KEY ("spellid", "SpellFamilyName", "SpellFamilyMask", "SpellIcon", "SpellVisual", "SpellCategory", "Code")
);
ALTER TABLE IF EXISTS "spell_check" ADD COLUMN IF NOT EXISTS "spellid" INTEGER NOT NULL DEFAULT '0' CHECK ("spellid" >= 0);
ALTER TABLE IF EXISTS "spell_check" ADD COLUMN IF NOT EXISTS "SpellFamilyName" SMALLINT NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_check" ADD COLUMN IF NOT EXISTS "SpellFamilyMask" BIGINT NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_check" ADD COLUMN IF NOT EXISTS "SpellIcon" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_check" ADD COLUMN IF NOT EXISTS "SpellVisual" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_check" ADD COLUMN IF NOT EXISTS "SpellCategory" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_check" ADD COLUMN IF NOT EXISTS "EffectType" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_check" ADD COLUMN IF NOT EXISTS "EffectAura" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_check" ADD COLUMN IF NOT EXISTS "EffectIdx" SMALLINT NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_check" ADD COLUMN IF NOT EXISTS "Name" VARCHAR(40) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "spell_check" ADD COLUMN IF NOT EXISTS "Code" VARCHAR(40) NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS "spell_disabled" (
    "entry" INTEGER NOT NULL CHECK ("entry" >= 0),
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "spell_disabled" ADD COLUMN IF NOT EXISTS "entry" INTEGER NOT NULL CHECK ("entry" >= 0);

CREATE TABLE IF NOT EXISTS "spell_effect_mod" (
    "Id" INTEGER NOT NULL DEFAULT '0' CHECK ("Id" >= 0),
    "EffectIndex" BIGINT NOT NULL DEFAULT '0' CHECK ("EffectIndex" >= 0),
    "Effect" INTEGER NOT NULL DEFAULT '-1',
    "EffectDieSides" INTEGER NOT NULL DEFAULT '-1',
    "EffectBaseDice" INTEGER NOT NULL DEFAULT '-1',
    "EffectDicePerLevel" REAL NOT NULL DEFAULT '-1',
    "EffectRealPointsPerLevel" REAL NOT NULL DEFAULT '-1',
    "EffectBasePoints" INTEGER NOT NULL DEFAULT '-1',
    "EffectAmplitude" INTEGER NOT NULL DEFAULT '-1',
    "EffectPointsPerComboPoint" REAL NOT NULL DEFAULT '-1',
    "EffectChainTarget" INTEGER NOT NULL DEFAULT '-1',
    "EffectMultipleValue" REAL NOT NULL DEFAULT '-1',
    "EffectMechanic" INTEGER NOT NULL DEFAULT '-1',
    "EffectImplicitTargetA" INTEGER NOT NULL DEFAULT '-1',
    "EffectImplicitTargetB" INTEGER NOT NULL DEFAULT '-1',
    "EffectRadiusIndex" INTEGER NOT NULL DEFAULT '-1',
    "EffectApplyAuraName" INTEGER NOT NULL DEFAULT '-1',
    "EffectItemType" BIGINT NOT NULL DEFAULT '-1',
    "EffectMiscValue" INTEGER NOT NULL DEFAULT '-1',
    "EffectTriggerSpell" INTEGER NOT NULL DEFAULT '-1',
    "Comment" VARCHAR(255) DEFAULT NULL,
    PRIMARY KEY ("Id", "EffectIndex")
);
ALTER TABLE IF EXISTS "spell_effect_mod" ADD COLUMN IF NOT EXISTS "Id" INTEGER NOT NULL DEFAULT '0' CHECK ("Id" >= 0);
ALTER TABLE IF EXISTS "spell_effect_mod" ADD COLUMN IF NOT EXISTS "EffectIndex" BIGINT NOT NULL DEFAULT '0' CHECK ("EffectIndex" >= 0);
ALTER TABLE IF EXISTS "spell_effect_mod" ADD COLUMN IF NOT EXISTS "Effect" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_effect_mod" ADD COLUMN IF NOT EXISTS "EffectDieSides" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_effect_mod" ADD COLUMN IF NOT EXISTS "EffectBaseDice" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_effect_mod" ADD COLUMN IF NOT EXISTS "EffectDicePerLevel" REAL NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_effect_mod" ADD COLUMN IF NOT EXISTS "EffectRealPointsPerLevel" REAL NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_effect_mod" ADD COLUMN IF NOT EXISTS "EffectBasePoints" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_effect_mod" ADD COLUMN IF NOT EXISTS "EffectAmplitude" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_effect_mod" ADD COLUMN IF NOT EXISTS "EffectPointsPerComboPoint" REAL NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_effect_mod" ADD COLUMN IF NOT EXISTS "EffectChainTarget" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_effect_mod" ADD COLUMN IF NOT EXISTS "EffectMultipleValue" REAL NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_effect_mod" ADD COLUMN IF NOT EXISTS "EffectMechanic" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_effect_mod" ADD COLUMN IF NOT EXISTS "EffectImplicitTargetA" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_effect_mod" ADD COLUMN IF NOT EXISTS "EffectImplicitTargetB" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_effect_mod" ADD COLUMN IF NOT EXISTS "EffectRadiusIndex" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_effect_mod" ADD COLUMN IF NOT EXISTS "EffectApplyAuraName" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_effect_mod" ADD COLUMN IF NOT EXISTS "EffectItemType" BIGINT NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_effect_mod" ADD COLUMN IF NOT EXISTS "EffectMiscValue" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_effect_mod" ADD COLUMN IF NOT EXISTS "EffectTriggerSpell" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_effect_mod" ADD COLUMN IF NOT EXISTS "Comment" VARCHAR(255) DEFAULT NULL;

CREATE TABLE IF NOT EXISTS "spell_elixir" (
    "entry" INTEGER NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "mask" SMALLINT NOT NULL DEFAULT '0' CHECK ("mask" >= 0),
    "build_min" INTEGER NOT NULL DEFAULT '0' CHECK ("build_min" >= 0),
    "build_max" INTEGER NOT NULL DEFAULT '5875' CHECK ("build_max" >= 0),
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "spell_elixir" ADD COLUMN IF NOT EXISTS "entry" INTEGER NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "spell_elixir" ADD COLUMN IF NOT EXISTS "mask" SMALLINT NOT NULL DEFAULT '0' CHECK ("mask" >= 0);
ALTER TABLE IF EXISTS "spell_elixir" ADD COLUMN IF NOT EXISTS "build_min" INTEGER NOT NULL DEFAULT '0' CHECK ("build_min" >= 0);
ALTER TABLE IF EXISTS "spell_elixir" ADD COLUMN IF NOT EXISTS "build_max" INTEGER NOT NULL DEFAULT '5875' CHECK ("build_max" >= 0);

CREATE TABLE IF NOT EXISTS "spell_enchant_charges" (
    "entry" INTEGER NOT NULL CHECK ("entry" >= 0),
    "charges" BIGINT NOT NULL DEFAULT '0' CHECK ("charges" >= 0),
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "spell_enchant_charges" ADD COLUMN IF NOT EXISTS "entry" INTEGER NOT NULL CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "spell_enchant_charges" ADD COLUMN IF NOT EXISTS "charges" BIGINT NOT NULL DEFAULT '0' CHECK ("charges" >= 0);

CREATE TABLE IF NOT EXISTS "spell_group" (
    "group_id" BIGINT NOT NULL DEFAULT '0' CHECK ("group_id" >= 0),
    "group_spell_id" BIGINT NOT NULL DEFAULT '0' CHECK ("group_spell_id" >= 0),
    "spell_id" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_id" >= 0),
    "build_min" INTEGER NOT NULL DEFAULT '0' CHECK ("build_min" >= 0),
    "build_max" INTEGER NOT NULL DEFAULT '5875' CHECK ("build_max" >= 0),
    PRIMARY KEY ("group_id", "group_spell_id", "spell_id"),
    UNIQUE ("group_id", "group_spell_id")
);
ALTER TABLE IF EXISTS "spell_group" ADD COLUMN IF NOT EXISTS "group_id" BIGINT NOT NULL DEFAULT '0' CHECK ("group_id" >= 0);
ALTER TABLE IF EXISTS "spell_group" ADD COLUMN IF NOT EXISTS "group_spell_id" BIGINT NOT NULL DEFAULT '0' CHECK ("group_spell_id" >= 0);
ALTER TABLE IF EXISTS "spell_group" ADD COLUMN IF NOT EXISTS "spell_id" INTEGER NOT NULL DEFAULT '0' CHECK ("spell_id" >= 0);
ALTER TABLE IF EXISTS "spell_group" ADD COLUMN IF NOT EXISTS "build_min" INTEGER NOT NULL DEFAULT '0' CHECK ("build_min" >= 0);
ALTER TABLE IF EXISTS "spell_group" ADD COLUMN IF NOT EXISTS "build_max" INTEGER NOT NULL DEFAULT '5875' CHECK ("build_max" >= 0);
CREATE UNIQUE INDEX IF NOT EXISTS idx_spell_group_group_id ON "spell_group" ("group_id", "group_spell_id");

CREATE TABLE IF NOT EXISTS "spell_group_stack_rules" (
    "group_id" BIGINT NOT NULL DEFAULT '0' CHECK ("group_id" >= 0),
    "build" INTEGER NOT NULL DEFAULT '0' CHECK ("build" >= 0),
    "stack_rule" SMALLINT NOT NULL DEFAULT '1',
    PRIMARY KEY ("group_id", "build")
);
ALTER TABLE IF EXISTS "spell_group_stack_rules" ADD COLUMN IF NOT EXISTS "group_id" BIGINT NOT NULL DEFAULT '0' CHECK ("group_id" >= 0);
ALTER TABLE IF EXISTS "spell_group_stack_rules" ADD COLUMN IF NOT EXISTS "build" INTEGER NOT NULL DEFAULT '0' CHECK ("build" >= 0);
ALTER TABLE IF EXISTS "spell_group_stack_rules" ADD COLUMN IF NOT EXISTS "stack_rule" SMALLINT NOT NULL DEFAULT '1';

CREATE TABLE IF NOT EXISTS "spell_learn_spell" (
    "entry" INTEGER NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "SpellID" INTEGER NOT NULL DEFAULT '0' CHECK ("SpellID" >= 0),
    "Active" SMALLINT NOT NULL DEFAULT '1' CHECK ("Active" >= 0),
    "build_min" INTEGER NOT NULL DEFAULT '0' CHECK ("build_min" >= 0),
    "build_max" INTEGER NOT NULL DEFAULT '5875' CHECK ("build_max" >= 0),
    PRIMARY KEY ("entry", "SpellID")
);
ALTER TABLE IF EXISTS "spell_learn_spell" ADD COLUMN IF NOT EXISTS "entry" INTEGER NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "spell_learn_spell" ADD COLUMN IF NOT EXISTS "SpellID" INTEGER NOT NULL DEFAULT '0' CHECK ("SpellID" >= 0);
ALTER TABLE IF EXISTS "spell_learn_spell" ADD COLUMN IF NOT EXISTS "Active" SMALLINT NOT NULL DEFAULT '1' CHECK ("Active" >= 0);
ALTER TABLE IF EXISTS "spell_learn_spell" ADD COLUMN IF NOT EXISTS "build_min" INTEGER NOT NULL DEFAULT '0' CHECK ("build_min" >= 0);
ALTER TABLE IF EXISTS "spell_learn_spell" ADD COLUMN IF NOT EXISTS "build_max" INTEGER NOT NULL DEFAULT '5875' CHECK ("build_max" >= 0);

CREATE TABLE IF NOT EXISTS "spell_mod" (
    "Id" INTEGER NOT NULL DEFAULT '0' CHECK ("Id" >= 0),
    "procChance" INTEGER DEFAULT '-1',
    "procFlags" INTEGER DEFAULT '-1',
    "procCharges" INTEGER DEFAULT '-1',
    "DurationIndex" INTEGER DEFAULT '-1',
    "Category" INTEGER DEFAULT '-1',
    "CastingTimeIndex" INTEGER DEFAULT '-1',
    "StackAmount" INTEGER DEFAULT '-1',
    "SpellIconID" INTEGER DEFAULT '-1',
    "activeIconID" INTEGER DEFAULT '-1',
    "manaCost" INTEGER DEFAULT '-1',
    "Attributes" INTEGER DEFAULT '-1',
    "AttributesEx" INTEGER DEFAULT '-1',
    "AttributesEx2" INTEGER DEFAULT '-1',
    "AttributesEx3" INTEGER DEFAULT '-1',
    "AttributesEx4" INTEGER DEFAULT '-1',
    "Custom" INTEGER DEFAULT '0',
    "InterruptFlags" INTEGER DEFAULT '-1',
    "AuraInterruptFlags" INTEGER DEFAULT '-1',
    "ChannelInterruptFlags" INTEGER DEFAULT '-1',
    "Dispel" INTEGER NOT NULL DEFAULT '-1',
    "Stances" INTEGER DEFAULT '-1',
    "StancesNot" INTEGER DEFAULT '-1',
    "SpellVisual" INTEGER DEFAULT '-1',
    "ManaCostPercentage" INTEGER DEFAULT '-1',
    "StartRecoveryCategory" INTEGER DEFAULT '-1',
    "StartRecoveryTime" INTEGER DEFAULT '-1',
    "MaxAffectedTargets" INTEGER DEFAULT '-1',
    "MaxTargetLevel" INTEGER DEFAULT '-1',
    "DmgClass" INTEGER DEFAULT '-1',
    "rangeIndex" INTEGER DEFAULT '-1',
    "RecoveryTime" INTEGER NOT NULL DEFAULT '-1',
    "CategoryRecoveryTime" INTEGER NOT NULL DEFAULT '-1',
    "SpellFamilyName" INTEGER NOT NULL DEFAULT '-1',
    "SpellFamilyFlags" NUMERIC(20,0) DEFAULT '0' CHECK ("SpellFamilyFlags" >= 0),
    "Mechanic" INTEGER DEFAULT '-1',
    "EquippedItemClass" INTEGER DEFAULT '-1',
    "Comment" VARCHAR(255) DEFAULT NULL,
    PRIMARY KEY ("Id")
);
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "Id" INTEGER NOT NULL DEFAULT '0' CHECK ("Id" >= 0);
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "procChance" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "procFlags" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "procCharges" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "DurationIndex" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "Category" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "CastingTimeIndex" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "StackAmount" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "SpellIconID" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "activeIconID" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "manaCost" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "Attributes" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "AttributesEx" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "AttributesEx2" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "AttributesEx3" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "AttributesEx4" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "Custom" INTEGER DEFAULT '0';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "InterruptFlags" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "AuraInterruptFlags" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "ChannelInterruptFlags" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "Dispel" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "Stances" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "StancesNot" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "SpellVisual" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "ManaCostPercentage" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "StartRecoveryCategory" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "StartRecoveryTime" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "MaxAffectedTargets" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "MaxTargetLevel" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "DmgClass" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "rangeIndex" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "RecoveryTime" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "CategoryRecoveryTime" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "SpellFamilyName" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "SpellFamilyFlags" NUMERIC(20,0) DEFAULT '0' CHECK ("SpellFamilyFlags" >= 0);
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "Mechanic" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "EquippedItemClass" INTEGER DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_mod" ADD COLUMN IF NOT EXISTS "Comment" VARCHAR(255) DEFAULT NULL;

CREATE TABLE IF NOT EXISTS "spell_pet_auras" (
    "spell" INTEGER NOT NULL CHECK ("spell" >= 0),
    "pet" BIGINT NOT NULL DEFAULT '0' CHECK ("pet" >= 0),
    "aura" BIGINT NOT NULL CHECK ("aura" >= 0),
    PRIMARY KEY ("spell", "pet")
);
ALTER TABLE IF EXISTS "spell_pet_auras" ADD COLUMN IF NOT EXISTS "spell" INTEGER NOT NULL CHECK ("spell" >= 0);
ALTER TABLE IF EXISTS "spell_pet_auras" ADD COLUMN IF NOT EXISTS "pet" BIGINT NOT NULL DEFAULT '0' CHECK ("pet" >= 0);
ALTER TABLE IF EXISTS "spell_pet_auras" ADD COLUMN IF NOT EXISTS "aura" BIGINT NOT NULL CHECK ("aura" >= 0);

CREATE TABLE IF NOT EXISTS "spell_proc_event" (
    "entry" INTEGER NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "SchoolMask" SMALLINT NOT NULL DEFAULT '0' CHECK ("SchoolMask" >= 0),
    "SpellFamilyName" INTEGER NOT NULL DEFAULT '0' CHECK ("SpellFamilyName" >= 0),
    "SpellFamilyMask0" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("SpellFamilyMask0" >= 0),
    "SpellFamilyMask1" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("SpellFamilyMask1" >= 0),
    "SpellFamilyMask2" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("SpellFamilyMask2" >= 0),
    "procFlags" BIGINT NOT NULL DEFAULT '0' CHECK ("procFlags" >= 0),
    "procEx" BIGINT NOT NULL DEFAULT '0' CHECK ("procEx" >= 0),
    "ppmRate" REAL NOT NULL DEFAULT '0',
    "CustomChance" REAL NOT NULL DEFAULT '0',
    "Cooldown" BIGINT NOT NULL DEFAULT '0' CHECK ("Cooldown" >= 0),
    "build_min" INTEGER NOT NULL DEFAULT '0' CHECK ("build_min" >= 0),
    "build_max" INTEGER NOT NULL DEFAULT '5875' CHECK ("build_max" >= 0),
    PRIMARY KEY ("entry", "build_min", "build_max")
);
ALTER TABLE IF EXISTS "spell_proc_event" ADD COLUMN IF NOT EXISTS "entry" INTEGER NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "spell_proc_event" ADD COLUMN IF NOT EXISTS "SchoolMask" SMALLINT NOT NULL DEFAULT '0' CHECK ("SchoolMask" >= 0);
ALTER TABLE IF EXISTS "spell_proc_event" ADD COLUMN IF NOT EXISTS "SpellFamilyName" INTEGER NOT NULL DEFAULT '0' CHECK ("SpellFamilyName" >= 0);
ALTER TABLE IF EXISTS "spell_proc_event" ADD COLUMN IF NOT EXISTS "SpellFamilyMask0" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("SpellFamilyMask0" >= 0);
ALTER TABLE IF EXISTS "spell_proc_event" ADD COLUMN IF NOT EXISTS "SpellFamilyMask1" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("SpellFamilyMask1" >= 0);
ALTER TABLE IF EXISTS "spell_proc_event" ADD COLUMN IF NOT EXISTS "SpellFamilyMask2" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("SpellFamilyMask2" >= 0);
ALTER TABLE IF EXISTS "spell_proc_event" ADD COLUMN IF NOT EXISTS "procFlags" BIGINT NOT NULL DEFAULT '0' CHECK ("procFlags" >= 0);
ALTER TABLE IF EXISTS "spell_proc_event" ADD COLUMN IF NOT EXISTS "procEx" BIGINT NOT NULL DEFAULT '0' CHECK ("procEx" >= 0);
ALTER TABLE IF EXISTS "spell_proc_event" ADD COLUMN IF NOT EXISTS "ppmRate" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_proc_event" ADD COLUMN IF NOT EXISTS "CustomChance" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_proc_event" ADD COLUMN IF NOT EXISTS "Cooldown" BIGINT NOT NULL DEFAULT '0' CHECK ("Cooldown" >= 0);
ALTER TABLE IF EXISTS "spell_proc_event" ADD COLUMN IF NOT EXISTS "build_min" INTEGER NOT NULL DEFAULT '0' CHECK ("build_min" >= 0);
ALTER TABLE IF EXISTS "spell_proc_event" ADD COLUMN IF NOT EXISTS "build_max" INTEGER NOT NULL DEFAULT '5875' CHECK ("build_max" >= 0);

CREATE TABLE IF NOT EXISTS "spell_proc_item_enchant" (
    "entry" INTEGER NOT NULL CHECK ("entry" >= 0),
    "ppmRate" REAL NOT NULL DEFAULT '0',
    PRIMARY KEY ("entry")
);
ALTER TABLE IF EXISTS "spell_proc_item_enchant" ADD COLUMN IF NOT EXISTS "entry" INTEGER NOT NULL CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "spell_proc_item_enchant" ADD COLUMN IF NOT EXISTS "ppmRate" REAL NOT NULL DEFAULT '0';

CREATE TABLE IF NOT EXISTS "spell_script_target" (
    "entry" INTEGER NOT NULL CHECK ("entry" >= 0),
    "type" SMALLINT NOT NULL DEFAULT '0' CHECK ("type" >= 0),
    "targetEntry" BIGINT NOT NULL DEFAULT '0' CHECK ("targetEntry" >= 0),
    "conditionId" BIGINT NOT NULL DEFAULT '0' CHECK ("conditionId" >= 0),
    "inverseEffectMask" BIGINT NOT NULL DEFAULT '0' CHECK ("inverseEffectMask" >= 0),
    "build_min" INTEGER NOT NULL DEFAULT '0' CHECK ("build_min" >= 0),
    "build_max" INTEGER NOT NULL DEFAULT '5875' CHECK ("build_max" >= 0),
    UNIQUE ("entry", "type", "targetEntry")
);
ALTER TABLE IF EXISTS "spell_script_target" ADD COLUMN IF NOT EXISTS "entry" INTEGER NOT NULL CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "spell_script_target" ADD COLUMN IF NOT EXISTS "type" SMALLINT NOT NULL DEFAULT '0' CHECK ("type" >= 0);
ALTER TABLE IF EXISTS "spell_script_target" ADD COLUMN IF NOT EXISTS "targetEntry" BIGINT NOT NULL DEFAULT '0' CHECK ("targetEntry" >= 0);
ALTER TABLE IF EXISTS "spell_script_target" ADD COLUMN IF NOT EXISTS "conditionId" BIGINT NOT NULL DEFAULT '0' CHECK ("conditionId" >= 0);
ALTER TABLE IF EXISTS "spell_script_target" ADD COLUMN IF NOT EXISTS "inverseEffectMask" BIGINT NOT NULL DEFAULT '0' CHECK ("inverseEffectMask" >= 0);
ALTER TABLE IF EXISTS "spell_script_target" ADD COLUMN IF NOT EXISTS "build_min" INTEGER NOT NULL DEFAULT '0' CHECK ("build_min" >= 0);
ALTER TABLE IF EXISTS "spell_script_target" ADD COLUMN IF NOT EXISTS "build_max" INTEGER NOT NULL DEFAULT '5875' CHECK ("build_max" >= 0);
CREATE UNIQUE INDEX IF NOT EXISTS idx_spell_script_target_entry_type_target ON "spell_script_target" ("entry", "type", "targetEntry");

CREATE TABLE IF NOT EXISTS "spell_scripts" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "delay" BIGINT NOT NULL DEFAULT '0' CHECK ("delay" >= 0),
    "priority" SMALLINT NOT NULL DEFAULT '0' CHECK ("priority" >= 0),
    "command" SMALLINT NOT NULL DEFAULT '0' CHECK ("command" >= 0),
    "datalong" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong" >= 0),
    "datalong2" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong2" >= 0),
    "datalong3" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong3" >= 0),
    "datalong4" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong4" >= 0),
    "target_param1" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param1" >= 0),
    "target_param2" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param2" >= 0),
    "target_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("target_type" >= 0),
    "data_flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("data_flags" >= 0),
    "dataint" INTEGER NOT NULL DEFAULT '0',
    "dataint2" INTEGER NOT NULL DEFAULT '0',
    "dataint3" INTEGER NOT NULL DEFAULT '0',
    "dataint4" INTEGER NOT NULL DEFAULT '0',
    "x" REAL NOT NULL DEFAULT '0',
    "y" REAL NOT NULL DEFAULT '0',
    "z" REAL NOT NULL DEFAULT '0',
    "o" REAL NOT NULL DEFAULT '0',
    "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0),
    "comments" VARCHAR(255) NOT NULL
);
ALTER TABLE IF EXISTS "spell_scripts" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "spell_scripts" ADD COLUMN IF NOT EXISTS "delay" BIGINT NOT NULL DEFAULT '0' CHECK ("delay" >= 0);
ALTER TABLE IF EXISTS "spell_scripts" ADD COLUMN IF NOT EXISTS "priority" SMALLINT NOT NULL DEFAULT '0' CHECK ("priority" >= 0);
ALTER TABLE IF EXISTS "spell_scripts" ADD COLUMN IF NOT EXISTS "command" SMALLINT NOT NULL DEFAULT '0' CHECK ("command" >= 0);
ALTER TABLE IF EXISTS "spell_scripts" ADD COLUMN IF NOT EXISTS "datalong" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong" >= 0);
ALTER TABLE IF EXISTS "spell_scripts" ADD COLUMN IF NOT EXISTS "datalong2" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong2" >= 0);
ALTER TABLE IF EXISTS "spell_scripts" ADD COLUMN IF NOT EXISTS "datalong3" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong3" >= 0);
ALTER TABLE IF EXISTS "spell_scripts" ADD COLUMN IF NOT EXISTS "datalong4" BIGINT NOT NULL DEFAULT '0' CHECK ("datalong4" >= 0);
ALTER TABLE IF EXISTS "spell_scripts" ADD COLUMN IF NOT EXISTS "target_param1" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param1" >= 0);
ALTER TABLE IF EXISTS "spell_scripts" ADD COLUMN IF NOT EXISTS "target_param2" BIGINT NOT NULL DEFAULT '0' CHECK ("target_param2" >= 0);
ALTER TABLE IF EXISTS "spell_scripts" ADD COLUMN IF NOT EXISTS "target_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("target_type" >= 0);
ALTER TABLE IF EXISTS "spell_scripts" ADD COLUMN IF NOT EXISTS "data_flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("data_flags" >= 0);
ALTER TABLE IF EXISTS "spell_scripts" ADD COLUMN IF NOT EXISTS "dataint" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_scripts" ADD COLUMN IF NOT EXISTS "dataint2" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_scripts" ADD COLUMN IF NOT EXISTS "dataint3" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_scripts" ADD COLUMN IF NOT EXISTS "dataint4" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_scripts" ADD COLUMN IF NOT EXISTS "x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_scripts" ADD COLUMN IF NOT EXISTS "y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_scripts" ADD COLUMN IF NOT EXISTS "z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_scripts" ADD COLUMN IF NOT EXISTS "o" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_scripts" ADD COLUMN IF NOT EXISTS "condition_id" BIGINT NOT NULL DEFAULT '0' CHECK ("condition_id" >= 0);
ALTER TABLE IF EXISTS "spell_scripts" ADD COLUMN IF NOT EXISTS "comments" VARCHAR(255) NOT NULL;

CREATE TABLE IF NOT EXISTS "spell_target_position" (
    "id" INTEGER NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "target_map" INTEGER NOT NULL DEFAULT '0' CHECK ("target_map" >= 0),
    "target_position_x" REAL NOT NULL DEFAULT '0',
    "target_position_y" REAL NOT NULL DEFAULT '0',
    "target_position_z" REAL NOT NULL DEFAULT '0',
    "target_orientation" REAL NOT NULL DEFAULT '0',
    "build_min" INTEGER NOT NULL DEFAULT '0' CHECK ("build_min" >= 0),
    "build_max" INTEGER NOT NULL DEFAULT '5875' CHECK ("build_max" >= 0),
    PRIMARY KEY ("id", "target_map")
);
ALTER TABLE IF EXISTS "spell_target_position" ADD COLUMN IF NOT EXISTS "id" INTEGER NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "spell_target_position" ADD COLUMN IF NOT EXISTS "target_map" INTEGER NOT NULL DEFAULT '0' CHECK ("target_map" >= 0);
ALTER TABLE IF EXISTS "spell_target_position" ADD COLUMN IF NOT EXISTS "target_position_x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_target_position" ADD COLUMN IF NOT EXISTS "target_position_y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_target_position" ADD COLUMN IF NOT EXISTS "target_position_z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_target_position" ADD COLUMN IF NOT EXISTS "target_orientation" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_target_position" ADD COLUMN IF NOT EXISTS "build_min" INTEGER NOT NULL DEFAULT '0' CHECK ("build_min" >= 0);
ALTER TABLE IF EXISTS "spell_target_position" ADD COLUMN IF NOT EXISTS "build_max" INTEGER NOT NULL DEFAULT '5875' CHECK ("build_max" >= 0);

CREATE TABLE IF NOT EXISTS "spell_template" (
    "entry" INTEGER NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "build" INTEGER NOT NULL DEFAULT '5875' CHECK ("build" >= 0),
    "school" BIGINT NOT NULL DEFAULT '0' CHECK ("school" >= 0),
    "category" BIGINT NOT NULL DEFAULT '0' CHECK ("category" >= 0),
    "castUI" BIGINT NOT NULL DEFAULT '0' CHECK ("castUI" >= 0),
    "dispel" BIGINT NOT NULL DEFAULT '0' CHECK ("dispel" >= 0),
    "mechanic" BIGINT NOT NULL DEFAULT '0' CHECK ("mechanic" >= 0),
    "attributes" BIGINT NOT NULL DEFAULT '0' CHECK ("attributes" >= 0),
    "attributesEx" BIGINT NOT NULL DEFAULT '0' CHECK ("attributesEx" >= 0),
    "attributesEx2" BIGINT NOT NULL DEFAULT '0' CHECK ("attributesEx2" >= 0),
    "attributesEx3" BIGINT NOT NULL DEFAULT '0' CHECK ("attributesEx3" >= 0),
    "attributesEx4" BIGINT NOT NULL DEFAULT '0' CHECK ("attributesEx4" >= 0),
    "stances" BIGINT NOT NULL DEFAULT '0' CHECK ("stances" >= 0),
    "stancesNot" BIGINT NOT NULL DEFAULT '0' CHECK ("stancesNot" >= 0),
    "targets" BIGINT NOT NULL DEFAULT '0' CHECK ("targets" >= 0),
    "targetCreatureType" BIGINT NOT NULL DEFAULT '0' CHECK ("targetCreatureType" >= 0),
    "requiresSpellFocus" BIGINT NOT NULL DEFAULT '0' CHECK ("requiresSpellFocus" >= 0),
    "casterAuraState" BIGINT NOT NULL DEFAULT '0' CHECK ("casterAuraState" >= 0),
    "targetAuraState" BIGINT NOT NULL DEFAULT '0' CHECK ("targetAuraState" >= 0),
    "castingTimeIndex" BIGINT NOT NULL DEFAULT '0' CHECK ("castingTimeIndex" >= 0),
    "recoveryTime" BIGINT NOT NULL DEFAULT '0' CHECK ("recoveryTime" >= 0),
    "categoryRecoveryTime" BIGINT NOT NULL DEFAULT '0' CHECK ("categoryRecoveryTime" >= 0),
    "interruptFlags" BIGINT NOT NULL DEFAULT '0' CHECK ("interruptFlags" >= 0),
    "auraInterruptFlags" BIGINT NOT NULL DEFAULT '0' CHECK ("auraInterruptFlags" >= 0),
    "channelInterruptFlags" BIGINT NOT NULL DEFAULT '0' CHECK ("channelInterruptFlags" >= 0),
    "procFlags" BIGINT NOT NULL DEFAULT '0' CHECK ("procFlags" >= 0),
    "procChance" BIGINT NOT NULL DEFAULT '0' CHECK ("procChance" >= 0),
    "procCharges" BIGINT NOT NULL DEFAULT '0' CHECK ("procCharges" >= 0),
    "maxLevel" BIGINT NOT NULL DEFAULT '0' CHECK ("maxLevel" >= 0),
    "baseLevel" BIGINT NOT NULL DEFAULT '0' CHECK ("baseLevel" >= 0),
    "spellLevel" BIGINT NOT NULL DEFAULT '0' CHECK ("spellLevel" >= 0),
    "durationIndex" BIGINT NOT NULL DEFAULT '0' CHECK ("durationIndex" >= 0),
    "powerType" BIGINT NOT NULL DEFAULT '0' CHECK ("powerType" >= 0),
    "manaCost" BIGINT NOT NULL DEFAULT '0' CHECK ("manaCost" >= 0),
    "manCostPerLevel" BIGINT NOT NULL DEFAULT '0' CHECK ("manCostPerLevel" >= 0),
    "manaPerSecond" BIGINT NOT NULL DEFAULT '0' CHECK ("manaPerSecond" >= 0),
    "manaPerSecondPerLevel" BIGINT NOT NULL DEFAULT '0' CHECK ("manaPerSecondPerLevel" >= 0),
    "rangeIndex" BIGINT NOT NULL DEFAULT '0' CHECK ("rangeIndex" >= 0),
    "speed" REAL NOT NULL DEFAULT '0',
    "modelNextSpell" BIGINT NOT NULL DEFAULT '0' CHECK ("modelNextSpell" >= 0),
    "stackAmount" BIGINT NOT NULL DEFAULT '0' CHECK ("stackAmount" >= 0),
    "totem1" BIGINT NOT NULL DEFAULT '0' CHECK ("totem1" >= 0),
    "totem2" BIGINT NOT NULL DEFAULT '0' CHECK ("totem2" >= 0),
    "reagent1" INTEGER NOT NULL DEFAULT '0',
    "reagent2" INTEGER NOT NULL DEFAULT '0',
    "reagent3" INTEGER NOT NULL DEFAULT '0',
    "reagent4" INTEGER NOT NULL DEFAULT '0',
    "reagent5" INTEGER NOT NULL DEFAULT '0',
    "reagent6" INTEGER NOT NULL DEFAULT '0',
    "reagent7" INTEGER NOT NULL DEFAULT '0',
    "reagent8" INTEGER NOT NULL DEFAULT '0',
    "reagentCount1" BIGINT NOT NULL DEFAULT '0' CHECK ("reagentCount1" >= 0),
    "reagentCount2" BIGINT NOT NULL DEFAULT '0' CHECK ("reagentCount2" >= 0),
    "reagentCount3" BIGINT NOT NULL DEFAULT '0' CHECK ("reagentCount3" >= 0),
    "reagentCount4" BIGINT NOT NULL DEFAULT '0' CHECK ("reagentCount4" >= 0),
    "reagentCount5" BIGINT NOT NULL DEFAULT '0' CHECK ("reagentCount5" >= 0),
    "reagentCount6" BIGINT NOT NULL DEFAULT '0' CHECK ("reagentCount6" >= 0),
    "reagentCount7" BIGINT NOT NULL DEFAULT '0' CHECK ("reagentCount7" >= 0),
    "reagentCount8" BIGINT NOT NULL DEFAULT '0' CHECK ("reagentCount8" >= 0),
    "equippedItemClass" INTEGER NOT NULL DEFAULT '0',
    "equippedItemSubClassMask" INTEGER NOT NULL DEFAULT '0',
    "equippedItemInventoryTypeMask" INTEGER NOT NULL DEFAULT '0',
    "effect1" BIGINT NOT NULL DEFAULT '0' CHECK ("effect1" >= 0),
    "effect2" BIGINT NOT NULL DEFAULT '0' CHECK ("effect2" >= 0),
    "effect3" BIGINT NOT NULL DEFAULT '0' CHECK ("effect3" >= 0),
    "effectDieSides1" INTEGER NOT NULL DEFAULT '0',
    "effectDieSides2" INTEGER NOT NULL DEFAULT '0',
    "effectDieSides3" INTEGER NOT NULL DEFAULT '0',
    "effectBaseDice1" BIGINT NOT NULL DEFAULT '0' CHECK ("effectBaseDice1" >= 0),
    "effectBaseDice2" BIGINT NOT NULL DEFAULT '0' CHECK ("effectBaseDice2" >= 0),
    "effectBaseDice3" BIGINT NOT NULL DEFAULT '0' CHECK ("effectBaseDice3" >= 0),
    "effectDicePerLevel1" REAL NOT NULL DEFAULT '0',
    "effectDicePerLevel2" REAL NOT NULL DEFAULT '0',
    "effectDicePerLevel3" REAL NOT NULL DEFAULT '0',
    "effectRealPointsPerLevel1" REAL NOT NULL DEFAULT '0',
    "effectRealPointsPerLevel2" REAL NOT NULL DEFAULT '0',
    "effectRealPointsPerLevel3" REAL NOT NULL DEFAULT '0',
    "effectBasePoints1" INTEGER NOT NULL DEFAULT '0',
    "effectBasePoints2" INTEGER NOT NULL DEFAULT '0',
    "effectBasePoints3" INTEGER NOT NULL DEFAULT '0',
    "effectBonusCoefficient1" REAL NOT NULL DEFAULT '-1',
    "effectBonusCoefficient2" REAL NOT NULL DEFAULT '-1',
    "effectBonusCoefficient3" REAL NOT NULL DEFAULT '-1',
    "effectMechanic1" BIGINT NOT NULL DEFAULT '0' CHECK ("effectMechanic1" >= 0),
    "effectMechanic2" BIGINT NOT NULL DEFAULT '0' CHECK ("effectMechanic2" >= 0),
    "effectMechanic3" BIGINT NOT NULL DEFAULT '0' CHECK ("effectMechanic3" >= 0),
    "effectImplicitTargetA1" BIGINT NOT NULL DEFAULT '0' CHECK ("effectImplicitTargetA1" >= 0),
    "effectImplicitTargetA2" BIGINT NOT NULL DEFAULT '0' CHECK ("effectImplicitTargetA2" >= 0),
    "effectImplicitTargetA3" BIGINT NOT NULL DEFAULT '0' CHECK ("effectImplicitTargetA3" >= 0),
    "effectImplicitTargetB1" BIGINT NOT NULL DEFAULT '0' CHECK ("effectImplicitTargetB1" >= 0),
    "effectImplicitTargetB2" BIGINT NOT NULL DEFAULT '0' CHECK ("effectImplicitTargetB2" >= 0),
    "effectImplicitTargetB3" BIGINT NOT NULL DEFAULT '0' CHECK ("effectImplicitTargetB3" >= 0),
    "effectRadiusIndex1" BIGINT NOT NULL DEFAULT '0' CHECK ("effectRadiusIndex1" >= 0),
    "effectRadiusIndex2" BIGINT NOT NULL DEFAULT '0' CHECK ("effectRadiusIndex2" >= 0),
    "effectRadiusIndex3" BIGINT NOT NULL DEFAULT '0' CHECK ("effectRadiusIndex3" >= 0),
    "effectApplyAuraName1" BIGINT NOT NULL DEFAULT '0' CHECK ("effectApplyAuraName1" >= 0),
    "effectApplyAuraName2" BIGINT NOT NULL DEFAULT '0' CHECK ("effectApplyAuraName2" >= 0),
    "effectApplyAuraName3" BIGINT NOT NULL DEFAULT '0' CHECK ("effectApplyAuraName3" >= 0),
    "effectAmplitude1" BIGINT NOT NULL DEFAULT '0' CHECK ("effectAmplitude1" >= 0),
    "effectAmplitude2" BIGINT NOT NULL DEFAULT '0' CHECK ("effectAmplitude2" >= 0),
    "effectAmplitude3" BIGINT NOT NULL DEFAULT '0' CHECK ("effectAmplitude3" >= 0),
    "effectMultipleValue1" REAL NOT NULL DEFAULT '0',
    "effectMultipleValue2" REAL NOT NULL DEFAULT '0',
    "effectMultipleValue3" REAL NOT NULL DEFAULT '0',
    "effectChainTarget1" BIGINT NOT NULL DEFAULT '0' CHECK ("effectChainTarget1" >= 0),
    "effectChainTarget2" BIGINT NOT NULL DEFAULT '0' CHECK ("effectChainTarget2" >= 0),
    "effectChainTarget3" BIGINT NOT NULL DEFAULT '0' CHECK ("effectChainTarget3" >= 0),
    "effectItemType1" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("effectItemType1" >= 0),
    "effectItemType2" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("effectItemType2" >= 0),
    "effectItemType3" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("effectItemType3" >= 0),
    "effectMiscValue1" INTEGER NOT NULL DEFAULT '0',
    "effectMiscValue2" INTEGER NOT NULL DEFAULT '0',
    "effectMiscValue3" INTEGER NOT NULL DEFAULT '0',
    "effectTriggerSpell1" BIGINT NOT NULL DEFAULT '0' CHECK ("effectTriggerSpell1" >= 0),
    "effectTriggerSpell2" BIGINT NOT NULL DEFAULT '0' CHECK ("effectTriggerSpell2" >= 0),
    "effectTriggerSpell3" BIGINT NOT NULL DEFAULT '0' CHECK ("effectTriggerSpell3" >= 0),
    "effectPointsPerComboPoint1" REAL NOT NULL DEFAULT '0',
    "effectPointsPerComboPoint2" REAL NOT NULL DEFAULT '0',
    "effectPointsPerComboPoint3" REAL NOT NULL DEFAULT '0',
    "spellVisual1" BIGINT NOT NULL DEFAULT '0' CHECK ("spellVisual1" >= 0),
    "spellVisual2" BIGINT NOT NULL DEFAULT '0' CHECK ("spellVisual2" >= 0),
    "spellIconId" BIGINT NOT NULL DEFAULT '0' CHECK ("spellIconId" >= 0),
    "activeIconId" BIGINT NOT NULL DEFAULT '0' CHECK ("activeIconId" >= 0),
    "spellPriority" BIGINT NOT NULL DEFAULT '0' CHECK ("spellPriority" >= 0),
    "name" VARCHAR(256) NOT NULL DEFAULT '',
    "nameFlags" BIGINT NOT NULL DEFAULT '0' CHECK ("nameFlags" >= 0),
    "nameSubtext" VARCHAR(256) NOT NULL DEFAULT '',
    "nameSubtextFlags" BIGINT NOT NULL DEFAULT '0' CHECK ("nameSubtextFlags" >= 0),
    "description" VARCHAR(1024) NOT NULL DEFAULT '',
    "descriptionFlags" BIGINT NOT NULL DEFAULT '0' CHECK ("descriptionFlags" >= 0),
    "auraDescription" VARCHAR(512) NOT NULL DEFAULT '',
    "auraDescriptionFlags" BIGINT NOT NULL DEFAULT '0' CHECK ("auraDescriptionFlags" >= 0),
    "manaCostPercentage" BIGINT NOT NULL DEFAULT '0' CHECK ("manaCostPercentage" >= 0),
    "startRecoveryCategory" BIGINT NOT NULL DEFAULT '0' CHECK ("startRecoveryCategory" >= 0),
    "startRecoveryTime" BIGINT NOT NULL DEFAULT '0' CHECK ("startRecoveryTime" >= 0),
    "minTargetLevel" BIGINT NOT NULL DEFAULT '0' CHECK ("minTargetLevel" >= 0),
    "maxTargetLevel" BIGINT NOT NULL DEFAULT '0' CHECK ("maxTargetLevel" >= 0),
    "spellFamilyName" BIGINT NOT NULL DEFAULT '0' CHECK ("spellFamilyName" >= 0),
    "spellFamilyFlags" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("spellFamilyFlags" >= 0),
    "maxAffectedTargets" BIGINT NOT NULL DEFAULT '0' CHECK ("maxAffectedTargets" >= 0),
    "dmgClass" BIGINT NOT NULL DEFAULT '0' CHECK ("dmgClass" >= 0),
    "preventionType" BIGINT NOT NULL DEFAULT '0' CHECK ("preventionType" >= 0),
    "stanceBarOrder" INTEGER NOT NULL DEFAULT '0',
    "dmgMultiplier1" REAL NOT NULL DEFAULT '0',
    "dmgMultiplier2" REAL NOT NULL DEFAULT '0',
    "dmgMultiplier3" REAL NOT NULL DEFAULT '0',
    "minFactionId" BIGINT NOT NULL DEFAULT '0' CHECK ("minFactionId" >= 0),
    "minReputation" BIGINT NOT NULL DEFAULT '0' CHECK ("minReputation" >= 0),
    "requiredAuraVision" BIGINT NOT NULL DEFAULT '0' CHECK ("requiredAuraVision" >= 0),
    "customFlags" BIGINT NOT NULL DEFAULT '0' CHECK ("customFlags" >= 0),
    "script_name" VARCHAR(64) NOT NULL DEFAULT '',
    PRIMARY KEY ("entry", "build")
);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "entry" INTEGER NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "build" INTEGER NOT NULL DEFAULT '5875' CHECK ("build" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "school" BIGINT NOT NULL DEFAULT '0' CHECK ("school" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "category" BIGINT NOT NULL DEFAULT '0' CHECK ("category" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "castUI" BIGINT NOT NULL DEFAULT '0' CHECK ("castUI" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "dispel" BIGINT NOT NULL DEFAULT '0' CHECK ("dispel" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "mechanic" BIGINT NOT NULL DEFAULT '0' CHECK ("mechanic" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "attributes" BIGINT NOT NULL DEFAULT '0' CHECK ("attributes" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "attributesEx" BIGINT NOT NULL DEFAULT '0' CHECK ("attributesEx" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "attributesEx2" BIGINT NOT NULL DEFAULT '0' CHECK ("attributesEx2" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "attributesEx3" BIGINT NOT NULL DEFAULT '0' CHECK ("attributesEx3" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "attributesEx4" BIGINT NOT NULL DEFAULT '0' CHECK ("attributesEx4" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "stances" BIGINT NOT NULL DEFAULT '0' CHECK ("stances" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "stancesNot" BIGINT NOT NULL DEFAULT '0' CHECK ("stancesNot" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "targets" BIGINT NOT NULL DEFAULT '0' CHECK ("targets" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "targetCreatureType" BIGINT NOT NULL DEFAULT '0' CHECK ("targetCreatureType" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "requiresSpellFocus" BIGINT NOT NULL DEFAULT '0' CHECK ("requiresSpellFocus" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "casterAuraState" BIGINT NOT NULL DEFAULT '0' CHECK ("casterAuraState" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "targetAuraState" BIGINT NOT NULL DEFAULT '0' CHECK ("targetAuraState" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "castingTimeIndex" BIGINT NOT NULL DEFAULT '0' CHECK ("castingTimeIndex" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "recoveryTime" BIGINT NOT NULL DEFAULT '0' CHECK ("recoveryTime" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "categoryRecoveryTime" BIGINT NOT NULL DEFAULT '0' CHECK ("categoryRecoveryTime" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "interruptFlags" BIGINT NOT NULL DEFAULT '0' CHECK ("interruptFlags" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "auraInterruptFlags" BIGINT NOT NULL DEFAULT '0' CHECK ("auraInterruptFlags" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "channelInterruptFlags" BIGINT NOT NULL DEFAULT '0' CHECK ("channelInterruptFlags" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "procFlags" BIGINT NOT NULL DEFAULT '0' CHECK ("procFlags" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "procChance" BIGINT NOT NULL DEFAULT '0' CHECK ("procChance" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "procCharges" BIGINT NOT NULL DEFAULT '0' CHECK ("procCharges" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "maxLevel" BIGINT NOT NULL DEFAULT '0' CHECK ("maxLevel" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "baseLevel" BIGINT NOT NULL DEFAULT '0' CHECK ("baseLevel" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "spellLevel" BIGINT NOT NULL DEFAULT '0' CHECK ("spellLevel" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "durationIndex" BIGINT NOT NULL DEFAULT '0' CHECK ("durationIndex" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "powerType" BIGINT NOT NULL DEFAULT '0' CHECK ("powerType" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "manaCost" BIGINT NOT NULL DEFAULT '0' CHECK ("manaCost" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "manCostPerLevel" BIGINT NOT NULL DEFAULT '0' CHECK ("manCostPerLevel" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "manaPerSecond" BIGINT NOT NULL DEFAULT '0' CHECK ("manaPerSecond" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "manaPerSecondPerLevel" BIGINT NOT NULL DEFAULT '0' CHECK ("manaPerSecondPerLevel" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "rangeIndex" BIGINT NOT NULL DEFAULT '0' CHECK ("rangeIndex" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "speed" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "modelNextSpell" BIGINT NOT NULL DEFAULT '0' CHECK ("modelNextSpell" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "stackAmount" BIGINT NOT NULL DEFAULT '0' CHECK ("stackAmount" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "totem1" BIGINT NOT NULL DEFAULT '0' CHECK ("totem1" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "totem2" BIGINT NOT NULL DEFAULT '0' CHECK ("totem2" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "reagent1" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "reagent2" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "reagent3" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "reagent4" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "reagent5" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "reagent6" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "reagent7" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "reagent8" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "reagentCount1" BIGINT NOT NULL DEFAULT '0' CHECK ("reagentCount1" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "reagentCount2" BIGINT NOT NULL DEFAULT '0' CHECK ("reagentCount2" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "reagentCount3" BIGINT NOT NULL DEFAULT '0' CHECK ("reagentCount3" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "reagentCount4" BIGINT NOT NULL DEFAULT '0' CHECK ("reagentCount4" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "reagentCount5" BIGINT NOT NULL DEFAULT '0' CHECK ("reagentCount5" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "reagentCount6" BIGINT NOT NULL DEFAULT '0' CHECK ("reagentCount6" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "reagentCount7" BIGINT NOT NULL DEFAULT '0' CHECK ("reagentCount7" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "reagentCount8" BIGINT NOT NULL DEFAULT '0' CHECK ("reagentCount8" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "equippedItemClass" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "equippedItemSubClassMask" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "equippedItemInventoryTypeMask" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effect1" BIGINT NOT NULL DEFAULT '0' CHECK ("effect1" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effect2" BIGINT NOT NULL DEFAULT '0' CHECK ("effect2" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effect3" BIGINT NOT NULL DEFAULT '0' CHECK ("effect3" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectDieSides1" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectDieSides2" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectDieSides3" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectBaseDice1" BIGINT NOT NULL DEFAULT '0' CHECK ("effectBaseDice1" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectBaseDice2" BIGINT NOT NULL DEFAULT '0' CHECK ("effectBaseDice2" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectBaseDice3" BIGINT NOT NULL DEFAULT '0' CHECK ("effectBaseDice3" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectDicePerLevel1" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectDicePerLevel2" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectDicePerLevel3" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectRealPointsPerLevel1" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectRealPointsPerLevel2" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectRealPointsPerLevel3" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectBasePoints1" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectBasePoints2" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectBasePoints3" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectBonusCoefficient1" REAL NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectBonusCoefficient2" REAL NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectBonusCoefficient3" REAL NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectMechanic1" BIGINT NOT NULL DEFAULT '0' CHECK ("effectMechanic1" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectMechanic2" BIGINT NOT NULL DEFAULT '0' CHECK ("effectMechanic2" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectMechanic3" BIGINT NOT NULL DEFAULT '0' CHECK ("effectMechanic3" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectImplicitTargetA1" BIGINT NOT NULL DEFAULT '0' CHECK ("effectImplicitTargetA1" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectImplicitTargetA2" BIGINT NOT NULL DEFAULT '0' CHECK ("effectImplicitTargetA2" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectImplicitTargetA3" BIGINT NOT NULL DEFAULT '0' CHECK ("effectImplicitTargetA3" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectImplicitTargetB1" BIGINT NOT NULL DEFAULT '0' CHECK ("effectImplicitTargetB1" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectImplicitTargetB2" BIGINT NOT NULL DEFAULT '0' CHECK ("effectImplicitTargetB2" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectImplicitTargetB3" BIGINT NOT NULL DEFAULT '0' CHECK ("effectImplicitTargetB3" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectRadiusIndex1" BIGINT NOT NULL DEFAULT '0' CHECK ("effectRadiusIndex1" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectRadiusIndex2" BIGINT NOT NULL DEFAULT '0' CHECK ("effectRadiusIndex2" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectRadiusIndex3" BIGINT NOT NULL DEFAULT '0' CHECK ("effectRadiusIndex3" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectApplyAuraName1" BIGINT NOT NULL DEFAULT '0' CHECK ("effectApplyAuraName1" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectApplyAuraName2" BIGINT NOT NULL DEFAULT '0' CHECK ("effectApplyAuraName2" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectApplyAuraName3" BIGINT NOT NULL DEFAULT '0' CHECK ("effectApplyAuraName3" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectAmplitude1" BIGINT NOT NULL DEFAULT '0' CHECK ("effectAmplitude1" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectAmplitude2" BIGINT NOT NULL DEFAULT '0' CHECK ("effectAmplitude2" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectAmplitude3" BIGINT NOT NULL DEFAULT '0' CHECK ("effectAmplitude3" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectMultipleValue1" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectMultipleValue2" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectMultipleValue3" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectChainTarget1" BIGINT NOT NULL DEFAULT '0' CHECK ("effectChainTarget1" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectChainTarget2" BIGINT NOT NULL DEFAULT '0' CHECK ("effectChainTarget2" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectChainTarget3" BIGINT NOT NULL DEFAULT '0' CHECK ("effectChainTarget3" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectItemType1" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("effectItemType1" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectItemType2" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("effectItemType2" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectItemType3" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("effectItemType3" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectMiscValue1" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectMiscValue2" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectMiscValue3" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectTriggerSpell1" BIGINT NOT NULL DEFAULT '0' CHECK ("effectTriggerSpell1" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectTriggerSpell2" BIGINT NOT NULL DEFAULT '0' CHECK ("effectTriggerSpell2" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectTriggerSpell3" BIGINT NOT NULL DEFAULT '0' CHECK ("effectTriggerSpell3" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectPointsPerComboPoint1" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectPointsPerComboPoint2" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "effectPointsPerComboPoint3" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "spellVisual1" BIGINT NOT NULL DEFAULT '0' CHECK ("spellVisual1" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "spellVisual2" BIGINT NOT NULL DEFAULT '0' CHECK ("spellVisual2" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "spellIconId" BIGINT NOT NULL DEFAULT '0' CHECK ("spellIconId" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "activeIconId" BIGINT NOT NULL DEFAULT '0' CHECK ("activeIconId" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "spellPriority" BIGINT NOT NULL DEFAULT '0' CHECK ("spellPriority" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "name" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "nameFlags" BIGINT NOT NULL DEFAULT '0' CHECK ("nameFlags" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "nameSubtext" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "nameSubtextFlags" BIGINT NOT NULL DEFAULT '0' CHECK ("nameSubtextFlags" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "description" VARCHAR(1024) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "descriptionFlags" BIGINT NOT NULL DEFAULT '0' CHECK ("descriptionFlags" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "auraDescription" VARCHAR(512) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "auraDescriptionFlags" BIGINT NOT NULL DEFAULT '0' CHECK ("auraDescriptionFlags" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "manaCostPercentage" BIGINT NOT NULL DEFAULT '0' CHECK ("manaCostPercentage" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "startRecoveryCategory" BIGINT NOT NULL DEFAULT '0' CHECK ("startRecoveryCategory" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "startRecoveryTime" BIGINT NOT NULL DEFAULT '0' CHECK ("startRecoveryTime" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "minTargetLevel" BIGINT NOT NULL DEFAULT '0' CHECK ("minTargetLevel" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "maxTargetLevel" BIGINT NOT NULL DEFAULT '0' CHECK ("maxTargetLevel" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "spellFamilyName" BIGINT NOT NULL DEFAULT '0' CHECK ("spellFamilyName" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "spellFamilyFlags" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("spellFamilyFlags" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "maxAffectedTargets" BIGINT NOT NULL DEFAULT '0' CHECK ("maxAffectedTargets" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "dmgClass" BIGINT NOT NULL DEFAULT '0' CHECK ("dmgClass" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "preventionType" BIGINT NOT NULL DEFAULT '0' CHECK ("preventionType" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "stanceBarOrder" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "dmgMultiplier1" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "dmgMultiplier2" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "dmgMultiplier3" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "minFactionId" BIGINT NOT NULL DEFAULT '0' CHECK ("minFactionId" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "minReputation" BIGINT NOT NULL DEFAULT '0' CHECK ("minReputation" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "requiredAuraVision" BIGINT NOT NULL DEFAULT '0' CHECK ("requiredAuraVision" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "customFlags" BIGINT NOT NULL DEFAULT '0' CHECK ("customFlags" >= 0);
ALTER TABLE IF EXISTS "spell_template" ADD COLUMN IF NOT EXISTS "script_name" VARCHAR(64) NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS "spell_threat" (
    "entry" INTEGER NOT NULL CHECK ("entry" >= 0),
    "Threat" REAL NOT NULL DEFAULT '0',
    "multiplier" REAL NOT NULL DEFAULT '1',
    "ap_bonus" REAL NOT NULL DEFAULT '0',
    "build_min" INTEGER NOT NULL DEFAULT '0' CHECK ("build_min" >= 0),
    "build_max" INTEGER NOT NULL DEFAULT '5875' CHECK ("build_max" >= 0),
    PRIMARY KEY ("entry", "build_min", "build_max")
);
ALTER TABLE IF EXISTS "spell_threat" ADD COLUMN IF NOT EXISTS "entry" INTEGER NOT NULL CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "spell_threat" ADD COLUMN IF NOT EXISTS "Threat" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_threat" ADD COLUMN IF NOT EXISTS "multiplier" REAL NOT NULL DEFAULT '1';
ALTER TABLE IF EXISTS "spell_threat" ADD COLUMN IF NOT EXISTS "ap_bonus" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "spell_threat" ADD COLUMN IF NOT EXISTS "build_min" INTEGER NOT NULL DEFAULT '0' CHECK ("build_min" >= 0);
ALTER TABLE IF EXISTS "spell_threat" ADD COLUMN IF NOT EXISTS "build_max" INTEGER NOT NULL DEFAULT '5875' CHECK ("build_max" >= 0);

CREATE TABLE IF NOT EXISTS "taxi_nodes" (
    "id" INTEGER NOT NULL CHECK ("id" >= 0),
    "build" INTEGER NOT NULL CHECK ("build" >= 0),
    "map_id" BIGINT NOT NULL DEFAULT '0' CHECK ("map_id" >= 0),
    "x" REAL NOT NULL DEFAULT '0',
    "y" REAL NOT NULL DEFAULT '0',
    "z" REAL NOT NULL DEFAULT '0',
    "name" VARCHAR(256) NOT NULL DEFAULT '',
    "mount_creature_id1" INTEGER NOT NULL DEFAULT '0' CHECK ("mount_creature_id1" >= 0),
    "mount_creature_id2" INTEGER NOT NULL DEFAULT '0' CHECK ("mount_creature_id2" >= 0),
    PRIMARY KEY ("id", "build")
);
ALTER TABLE IF EXISTS "taxi_nodes" ADD COLUMN IF NOT EXISTS "id" INTEGER NOT NULL CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "taxi_nodes" ADD COLUMN IF NOT EXISTS "build" INTEGER NOT NULL CHECK ("build" >= 0);
ALTER TABLE IF EXISTS "taxi_nodes" ADD COLUMN IF NOT EXISTS "map_id" BIGINT NOT NULL DEFAULT '0' CHECK ("map_id" >= 0);
ALTER TABLE IF EXISTS "taxi_nodes" ADD COLUMN IF NOT EXISTS "x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "taxi_nodes" ADD COLUMN IF NOT EXISTS "y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "taxi_nodes" ADD COLUMN IF NOT EXISTS "z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "taxi_nodes" ADD COLUMN IF NOT EXISTS "name" VARCHAR(256) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "taxi_nodes" ADD COLUMN IF NOT EXISTS "mount_creature_id1" INTEGER NOT NULL DEFAULT '0' CHECK ("mount_creature_id1" >= 0);
ALTER TABLE IF EXISTS "taxi_nodes" ADD COLUMN IF NOT EXISTS "mount_creature_id2" INTEGER NOT NULL DEFAULT '0' CHECK ("mount_creature_id2" >= 0);

CREATE TABLE IF NOT EXISTS "taxi_path_transitions" (
    "in_path" BIGINT NOT NULL DEFAULT '0' CHECK ("in_path" >= 0),
    "out_path" BIGINT NOT NULL DEFAULT '0' CHECK ("out_path" >= 0),
    "in_node" BIGINT NOT NULL DEFAULT '0' CHECK ("in_node" >= 0),
    "out_node" BIGINT NOT NULL DEFAULT '0' CHECK ("out_node" >= 0),
    "comment" TEXT,
    "build_min" INTEGER NOT NULL DEFAULT '0' CHECK ("build_min" >= 0),
    PRIMARY KEY ("in_path", "out_path")
);
ALTER TABLE IF EXISTS "taxi_path_transitions" ADD COLUMN IF NOT EXISTS "in_path" BIGINT NOT NULL DEFAULT '0' CHECK ("in_path" >= 0);
ALTER TABLE IF EXISTS "taxi_path_transitions" ADD COLUMN IF NOT EXISTS "out_path" BIGINT NOT NULL DEFAULT '0' CHECK ("out_path" >= 0);
ALTER TABLE IF EXISTS "taxi_path_transitions" ADD COLUMN IF NOT EXISTS "in_node" BIGINT NOT NULL DEFAULT '0' CHECK ("in_node" >= 0);
ALTER TABLE IF EXISTS "taxi_path_transitions" ADD COLUMN IF NOT EXISTS "out_node" BIGINT NOT NULL DEFAULT '0' CHECK ("out_node" >= 0);
ALTER TABLE IF EXISTS "taxi_path_transitions" ADD COLUMN IF NOT EXISTS "comment" TEXT;
ALTER TABLE IF EXISTS "taxi_path_transitions" ADD COLUMN IF NOT EXISTS "build_min" INTEGER NOT NULL DEFAULT '0' CHECK ("build_min" >= 0);

CREATE TABLE IF NOT EXISTS "transports" (
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "build" INTEGER NOT NULL DEFAULT '0' CHECK ("build" >= 0),
    "name" TEXT,
    "period" BIGINT NOT NULL DEFAULT '0' CHECK ("period" >= 0),
    PRIMARY KEY ("entry", "build")
);
ALTER TABLE IF EXISTS "transports" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "transports" ADD COLUMN IF NOT EXISTS "build" INTEGER NOT NULL DEFAULT '0' CHECK ("build" >= 0);
ALTER TABLE IF EXISTS "transports" ADD COLUMN IF NOT EXISTS "name" TEXT;
ALTER TABLE IF EXISTS "transports" ADD COLUMN IF NOT EXISTS "period" BIGINT NOT NULL DEFAULT '0' CHECK ("period" >= 0);

CREATE TABLE IF NOT EXISTS "variables" (
    "index" BIGINT NOT NULL DEFAULT '0' CHECK ("index" >= 0),
    "value" BIGINT NOT NULL DEFAULT '0' CHECK ("value" >= 0),
    PRIMARY KEY ("index")
);
ALTER TABLE IF EXISTS "variables" ADD COLUMN IF NOT EXISTS "index" BIGINT NOT NULL DEFAULT '0' CHECK ("index" >= 0);
ALTER TABLE IF EXISTS "variables" ADD COLUMN IF NOT EXISTS "value" BIGINT NOT NULL DEFAULT '0' CHECK ("value" >= 0);

CREATE TABLE IF NOT EXISTS "warden_scans" (
    "id" INTEGER GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("id" >= 0),
    "type" INTEGER DEFAULT '0',
    "str" TEXT,
    "data" TEXT,
    "address" INTEGER DEFAULT '0',
    "length" INTEGER DEFAULT '0',
    "result" TEXT NOT NULL,
    "flags" BIGINT NOT NULL CHECK ("flags" >= 0),
    "penalty" SMALLINT NOT NULL DEFAULT '-1',
    "build_min" INTEGER NOT NULL DEFAULT '5875' CHECK ("build_min" >= 0),
    "build_max" INTEGER NOT NULL DEFAULT '5875' CHECK ("build_max" >= 0),
    "comment" TEXT NOT NULL,
    UNIQUE ("id")
);
ALTER TABLE IF EXISTS "warden_scans" ADD COLUMN IF NOT EXISTS "id" INTEGER GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "warden_scans" ADD COLUMN IF NOT EXISTS "type" INTEGER DEFAULT '0';
ALTER TABLE IF EXISTS "warden_scans" ADD COLUMN IF NOT EXISTS "str" TEXT;
ALTER TABLE IF EXISTS "warden_scans" ADD COLUMN IF NOT EXISTS "data" TEXT;
ALTER TABLE IF EXISTS "warden_scans" ADD COLUMN IF NOT EXISTS "address" INTEGER DEFAULT '0';
ALTER TABLE IF EXISTS "warden_scans" ADD COLUMN IF NOT EXISTS "length" INTEGER DEFAULT '0';
ALTER TABLE IF EXISTS "warden_scans" ADD COLUMN IF NOT EXISTS "result" TEXT NOT NULL;
ALTER TABLE IF EXISTS "warden_scans" ADD COLUMN IF NOT EXISTS "flags" BIGINT NOT NULL CHECK ("flags" >= 0);
ALTER TABLE IF EXISTS "warden_scans" ADD COLUMN IF NOT EXISTS "penalty" SMALLINT NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "warden_scans" ADD COLUMN IF NOT EXISTS "build_min" INTEGER NOT NULL DEFAULT '5875' CHECK ("build_min" >= 0);
ALTER TABLE IF EXISTS "warden_scans" ADD COLUMN IF NOT EXISTS "build_max" INTEGER NOT NULL DEFAULT '5875' CHECK ("build_max" >= 0);
ALTER TABLE IF EXISTS "warden_scans" ADD COLUMN IF NOT EXISTS "comment" TEXT NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_warden_scans_id ON "warden_scans" ("id");

CREATE TABLE IF NOT EXISTS "world_safe_locs_facing" (
    "id" BIGINT NOT NULL CHECK ("id" >= 0),
    "orientation" REAL NOT NULL DEFAULT '0',
    PRIMARY KEY ("id")
);
ALTER TABLE IF EXISTS "world_safe_locs_facing" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "world_safe_locs_facing" ADD COLUMN IF NOT EXISTS "orientation" REAL NOT NULL DEFAULT '0';
