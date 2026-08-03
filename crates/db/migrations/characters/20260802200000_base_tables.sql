-- PostgreSQL migration: characters / base_tables
-- Consolidated table setup for the characters schema.
-- Fresh canonical character persistence surface. Account ownership intentionally has no
-- foreign key because auth remains independently migratable during the staged cutover.

CREATE TABLE characters (
    guid BIGINT PRIMARY KEY CHECK (guid >= 0),
    account BIGINT NOT NULL CHECK (account >= 0),
    name VARCHAR(12) NOT NULL UNIQUE,
    race SMALLINT NOT NULL CHECK (race >= 0),
    class SMALLINT NOT NULL CHECK (class >= 0),
    gender SMALLINT NOT NULL CHECK (gender >= 0),
    skin SMALLINT NOT NULL CHECK (skin >= 0),
    face SMALLINT NOT NULL CHECK (face >= 0),
    hair_style SMALLINT NOT NULL CHECK (hair_style >= 0),
    hair_color SMALLINT NOT NULL CHECK (hair_color >= 0),
    facial_hair SMALLINT NOT NULL CHECK (facial_hair >= 0),
    level SMALLINT NOT NULL DEFAULT 1 CHECK (level >= 1),
    xp BIGINT NOT NULL DEFAULT 0 CHECK (xp >= 0),
    money BIGINT NOT NULL DEFAULT 0 CHECK (money >= 0),
    character_flags BIGINT NOT NULL DEFAULT 0 CHECK (character_flags >= 0),
    zone BIGINT NOT NULL DEFAULT 0 CHECK (zone >= 0),
    map BIGINT NOT NULL DEFAULT 0 CHECK (map >= 0),
    instance BIGINT NOT NULL DEFAULT 0 CHECK (instance >= 0),
    position_x REAL NOT NULL DEFAULT 0,
    position_y REAL NOT NULL DEFAULT 0,
    position_z REAL NOT NULL DEFAULT 0,
    orientation REAL NOT NULL DEFAULT 0,
    transport_guid BIGINT NOT NULL DEFAULT 0 CHECK (transport_guid >= 0),
    transport_x REAL NOT NULL DEFAULT 0,
    transport_y REAL NOT NULL DEFAULT 0,
    transport_z REAL NOT NULL DEFAULT 0,
    transport_o REAL NOT NULL DEFAULT 0,
    known_taxi_mask TEXT,
    current_taxi_path TEXT,
    online BOOLEAN NOT NULL DEFAULT FALSE,
    played_time_total BIGINT NOT NULL DEFAULT 0 CHECK (played_time_total >= 0),
    played_time_level BIGINT NOT NULL DEFAULT 0 CHECK (played_time_level >= 0),
    health BIGINT NOT NULL DEFAULT 0 CHECK (health >= 0),
    power1 BIGINT NOT NULL DEFAULT 0 CHECK (power1 >= 0),
    power2 BIGINT NOT NULL DEFAULT 0 CHECK (power2 >= 0),
    power3 BIGINT NOT NULL DEFAULT 0 CHECK (power3 >= 0),
    power4 BIGINT NOT NULL DEFAULT 0 CHECK (power4 >= 0),
    power5 BIGINT NOT NULL DEFAULT 0 CHECK (power5 >= 0),
    create_time TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    logout_time TIMESTAMPTZ,
    rest_bonus REAL NOT NULL DEFAULT 0,
    reset_talents_multiplier BIGINT NOT NULL DEFAULT 0 CHECK (reset_talents_multiplier >= 0),
    reset_talents_time BIGINT NOT NULL DEFAULT 0 CHECK (reset_talents_time >= 0),
    death_expire_time BIGINT NOT NULL DEFAULT 0 CHECK (death_expire_time >= 0),
    stable_slots SMALLINT NOT NULL DEFAULT 0 CHECK (stable_slots >= 0),
    bank_bag_slots SMALLINT NOT NULL DEFAULT 0 CHECK (bank_bag_slots >= 0),
    extra_flags BIGINT NOT NULL DEFAULT 0 CHECK (extra_flags >= 0),
    honor_rank_points REAL NOT NULL DEFAULT 0,
    honor_highest_rank BIGINT NOT NULL DEFAULT 0 CHECK (honor_highest_rank >= 0),
    honor_standing BIGINT NOT NULL DEFAULT 0 CHECK (honor_standing >= 0),
    honor_last_week_hk BIGINT NOT NULL DEFAULT 0 CHECK (honor_last_week_hk >= 0),
    honor_last_week_cp REAL NOT NULL DEFAULT 0,
    honor_stored_hk INTEGER NOT NULL DEFAULT 0,
    honor_stored_dk INTEGER NOT NULL DEFAULT 0,
    watched_faction INTEGER NOT NULL DEFAULT -1,
    drunk INTEGER NOT NULL DEFAULT 0 CHECK (drunk >= 0),
    explored_zones TEXT,
    equipment_cache TEXT,
    ammo_id BIGINT NOT NULL DEFAULT 0 CHECK (ammo_id >= 0),
    action_bars SMALLINT NOT NULL DEFAULT 0 CHECK (action_bars >= 0),
    deleted_account BIGINT CHECK (deleted_account >= 0),
    deleted_name VARCHAR(12),
    deleted_time BIGINT,
    world_phase_mask INTEGER DEFAULT 0
);
CREATE INDEX idx_characters_account ON characters (account);
CREATE INDEX idx_characters_online ON characters (online);
CREATE INDEX idx_characters_instance ON characters (instance);

CREATE TABLE item_instance (
    guid BIGINT PRIMARY KEY CHECK (guid >= 0),
    item_id BIGINT NOT NULL CHECK (item_id >= 0),
    owner_guid BIGINT NOT NULL CHECK (owner_guid >= 0),
    creator_guid BIGINT NOT NULL DEFAULT 0 CHECK (creator_guid >= 0),
    gift_creator_guid BIGINT NOT NULL DEFAULT 0 CHECK (gift_creator_guid >= 0),
    count BIGINT NOT NULL DEFAULT 1 CHECK (count >= 0),
    duration INTEGER NOT NULL DEFAULT 0,
    charges TEXT,
    flags BIGINT NOT NULL DEFAULT 0 CHECK (flags >= 0),
    enchantments TEXT NOT NULL DEFAULT '',
    random_property_id SMALLINT NOT NULL DEFAULT 0,
    durability INTEGER NOT NULL DEFAULT 0 CHECK (durability >= 0),
    text BIGINT NOT NULL DEFAULT 0 CHECK (text >= 0),
    generated_loot BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE INDEX idx_item_instance_owner_guid ON item_instance (owner_guid);
CREATE INDEX idx_item_instance_item_id ON item_instance (item_id);

CREATE TABLE character_inventory (
    item_guid BIGINT PRIMARY KEY REFERENCES item_instance (guid) ON DELETE CASCADE,
    guid BIGINT NOT NULL REFERENCES characters (guid) ON DELETE CASCADE,
    bag BIGINT NOT NULL DEFAULT 0 CHECK (bag >= 0),
    slot SMALLINT NOT NULL DEFAULT 0 CHECK (slot >= 0),
    item_id BIGINT NOT NULL CHECK (item_id >= 0),
    UNIQUE (guid, bag, slot)
);
CREATE INDEX idx_character_inventory_guid ON character_inventory (guid);

CREATE TABLE character_queststatus (
    guid BIGINT NOT NULL REFERENCES characters (guid) ON DELETE CASCADE,
    quest BIGINT NOT NULL CHECK (quest >= 0),
    status SMALLINT NOT NULL DEFAULT 0 CHECK (status >= 0),
    rewarded BOOLEAN NOT NULL DEFAULT FALSE,
    explored BOOLEAN NOT NULL DEFAULT FALSE,
    timer BIGINT NOT NULL DEFAULT 0 CHECK (timer >= 0),
    mob_count1 BIGINT NOT NULL DEFAULT 0 CHECK (mob_count1 >= 0),
    mob_count2 BIGINT NOT NULL DEFAULT 0 CHECK (mob_count2 >= 0),
    mob_count3 BIGINT NOT NULL DEFAULT 0 CHECK (mob_count3 >= 0),
    mob_count4 BIGINT NOT NULL DEFAULT 0 CHECK (mob_count4 >= 0),
    item_count1 BIGINT NOT NULL DEFAULT 0 CHECK (item_count1 >= 0),
    item_count2 BIGINT NOT NULL DEFAULT 0 CHECK (item_count2 >= 0),
    item_count3 BIGINT NOT NULL DEFAULT 0 CHECK (item_count3 >= 0),
    item_count4 BIGINT NOT NULL DEFAULT 0 CHECK (item_count4 >= 0),
    reward_choice BIGINT NOT NULL DEFAULT 0 CHECK (reward_choice >= 0),
    PRIMARY KEY (guid, quest)
);

CREATE TABLE character_reputation (
    guid BIGINT NOT NULL REFERENCES characters (guid) ON DELETE CASCADE,
    faction BIGINT NOT NULL CHECK (faction >= 0),
    standing INTEGER NOT NULL DEFAULT 0,
    flags INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (guid, faction)
);
-- Consolidated from the previous mail_auction_social_groups setup migration

CREATE TABLE item_text (
    id BIGINT GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY CHECK (id >= 0),
    text TEXT
);

CREATE TABLE mail (
    id BIGINT GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY CHECK (id >= 0),
    message_type SMALLINT NOT NULL DEFAULT 0 CHECK (message_type >= 0),
    stationery SMALLINT NOT NULL DEFAULT 41,
    mail_template_id BIGINT NOT NULL DEFAULT 0 CHECK (mail_template_id >= 0),
    sender_guid BIGINT NOT NULL DEFAULT 0 CHECK (sender_guid >= 0),
    receiver_guid BIGINT NOT NULL DEFAULT 0 CHECK (receiver_guid >= 0),
    subject TEXT,
    item_text_id BIGINT NOT NULL DEFAULT 0 CHECK (item_text_id >= 0),
    has_items SMALLINT NOT NULL DEFAULT 0 CHECK (has_items >= 0),
    expire_time BIGINT NOT NULL DEFAULT 0,
    deliver_time BIGINT NOT NULL DEFAULT 0,
    money BIGINT NOT NULL DEFAULT 0 CHECK (money >= 0),
    cod BIGINT NOT NULL DEFAULT 0 CHECK (cod >= 0),
    checked SMALLINT NOT NULL DEFAULT 0 CHECK (checked >= 0)
);
CREATE INDEX idx_mail_receiver_guid ON mail (receiver_guid);

CREATE TABLE mail_items (
    mail_id BIGINT NOT NULL REFERENCES mail (id) ON DELETE CASCADE,
    item_guid BIGINT NOT NULL REFERENCES item_instance (guid) ON DELETE CASCADE,
    item_id BIGINT NOT NULL CHECK (item_id >= 0),
    receiver_guid BIGINT NOT NULL CHECK (receiver_guid >= 0),
    PRIMARY KEY (mail_id, item_guid)
);
CREATE INDEX idx_mail_items_receiver_guid ON mail_items (receiver_guid);
CREATE INDEX idx_mail_items_item_guid ON mail_items (item_guid);

CREATE TABLE auction (
    id BIGINT PRIMARY KEY CHECK (id >= 0),
    house_id BIGINT NOT NULL DEFAULT 0 CHECK (house_id >= 0),
    item_guid BIGINT NOT NULL UNIQUE REFERENCES item_instance (guid) ON DELETE CASCADE,
    item_id BIGINT NOT NULL DEFAULT 0 CHECK (item_id >= 0),
    seller_guid BIGINT NOT NULL DEFAULT 0 CHECK (seller_guid >= 0),
    buyout_price INTEGER NOT NULL DEFAULT 0,
    expire_time BIGINT NOT NULL DEFAULT 0,
    buyer_guid BIGINT NOT NULL DEFAULT 0 CHECK (buyer_guid >= 0),
    last_bid INTEGER NOT NULL DEFAULT 0,
    start_bid INTEGER NOT NULL DEFAULT 0,
    deposit INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_auction_house_id ON auction (house_id);
CREATE INDEX idx_auction_seller_guid ON auction (seller_guid);
CREATE INDEX idx_auction_buyer_guid ON auction (buyer_guid);
CREATE INDEX idx_auction_expire_time ON auction (expire_time);

CREATE TABLE character_social (
    guid BIGINT NOT NULL REFERENCES characters (guid) ON DELETE CASCADE,
    friend BIGINT NOT NULL REFERENCES characters (guid) ON DELETE CASCADE,
    flags SMALLINT NOT NULL DEFAULT 0 CHECK (flags >= 0),
    PRIMARY KEY (guid, friend)
);
CREATE INDEX idx_character_social_friend ON character_social (friend);
CREATE INDEX idx_character_social_guid_flags ON character_social (guid, flags);
CREATE INDEX idx_character_social_friend_flags ON character_social (friend, flags);

CREATE TABLE groups (
    group_id BIGINT PRIMARY KEY CHECK (group_id >= 0),
    leader_guid BIGINT NOT NULL UNIQUE REFERENCES characters (guid),
    main_tank_guid BIGINT NOT NULL CHECK (main_tank_guid >= 0),
    main_assistant_guid BIGINT NOT NULL CHECK (main_assistant_guid >= 0),
    loot_method SMALLINT NOT NULL CHECK (loot_method >= 0),
    loot_threshold SMALLINT NOT NULL CHECK (loot_threshold >= 0),
    looter_guid BIGINT NOT NULL CHECK (looter_guid >= 0),
    icon1 BIGINT NOT NULL CHECK (icon1 >= 0),
    icon2 BIGINT NOT NULL CHECK (icon2 >= 0),
    icon3 BIGINT NOT NULL CHECK (icon3 >= 0),
    icon4 BIGINT NOT NULL CHECK (icon4 >= 0),
    icon5 BIGINT NOT NULL CHECK (icon5 >= 0),
    icon6 BIGINT NOT NULL CHECK (icon6 >= 0),
    icon7 BIGINT NOT NULL CHECK (icon7 >= 0),
    icon8 BIGINT NOT NULL CHECK (icon8 >= 0),
    is_raid SMALLINT NOT NULL CHECK (is_raid >= 0)
);

CREATE TABLE group_member (
    group_id BIGINT NOT NULL REFERENCES groups (group_id) ON DELETE CASCADE,
    member_guid BIGINT NOT NULL REFERENCES characters (guid) ON DELETE CASCADE,
    assistant SMALLINT NOT NULL CHECK (assistant >= 0),
    subgroup SMALLINT NOT NULL CHECK (subgroup >= 0),
    PRIMARY KEY (group_id, member_guid)
);
CREATE INDEX idx_group_member_member_guid ON group_member (member_guid);

CREATE TABLE group_instance (
    leader_guid BIGINT NOT NULL REFERENCES groups (leader_guid) ON DELETE CASCADE,
    instance BIGINT NOT NULL CHECK (instance >= 0),
    permanent SMALLINT NOT NULL DEFAULT 0 CHECK (permanent >= 0),
    PRIMARY KEY (leader_guid, instance)
);
CREATE INDEX idx_group_instance_instance ON group_instance (instance);
-- Consolidated from the previous guilds_support_instances_honor setup migration.

CREATE TABLE guild (
    guild_id BIGINT PRIMARY KEY CHECK (guild_id >= 0),
    name TEXT NOT NULL UNIQUE,
    leader_guid BIGINT NOT NULL UNIQUE REFERENCES characters (guid),
    emblem_style INTEGER NOT NULL DEFAULT 0, emblem_color INTEGER NOT NULL DEFAULT 0,
    border_style INTEGER NOT NULL DEFAULT 0, border_color INTEGER NOT NULL DEFAULT 0,
    background_color INTEGER NOT NULL DEFAULT 0, info TEXT NOT NULL DEFAULT '',
    motd TEXT NOT NULL DEFAULT '', create_date BIGINT NOT NULL DEFAULT 0,
    bank_money BIGINT NOT NULL DEFAULT 0 CHECK (bank_money >= 0)
);
CREATE TABLE guild_rank (
    guild_id BIGINT NOT NULL REFERENCES guild (guild_id) ON DELETE CASCADE,
    id BIGINT NOT NULL CHECK (id >= 0), name TEXT NOT NULL DEFAULT '',
    rights BIGINT NOT NULL DEFAULT 0 CHECK (rights >= 0), PRIMARY KEY (guild_id, id)
);
CREATE TABLE guild_member (
    guild_id BIGINT NOT NULL REFERENCES guild (guild_id) ON DELETE CASCADE,
    guid BIGINT NOT NULL UNIQUE REFERENCES characters (guid) ON DELETE CASCADE,
    rank SMALLINT NOT NULL DEFAULT 0 CHECK (rank >= 0), player_note TEXT NOT NULL DEFAULT '',
    officer_note TEXT NOT NULL DEFAULT '', PRIMARY KEY (guild_id, guid)
);
CREATE INDEX idx_guild_member_guild_rank ON guild_member (guild_id, rank);
CREATE TABLE guild_bank_tab (
    guild_id BIGINT NOT NULL REFERENCES guild (guild_id) ON DELETE CASCADE,
    tab_id SMALLINT NOT NULL CHECK (tab_id >= 0), name TEXT NOT NULL DEFAULT '',
    icon TEXT NOT NULL DEFAULT '', view_rank SMALLINT NOT NULL DEFAULT 0 CHECK (view_rank >= 0),
    withdraw_rank SMALLINT NOT NULL DEFAULT 0 CHECK (withdraw_rank >= 0),
    deposit_rank SMALLINT NOT NULL DEFAULT 0 CHECK (deposit_rank >= 0), PRIMARY KEY (guild_id, tab_id)
);
CREATE TABLE guild_eventlog (
    guild_id BIGINT NOT NULL REFERENCES guild (guild_id) ON DELETE CASCADE,
    log_guid BIGINT NOT NULL, event_type SMALLINT NOT NULL, player_guid1 BIGINT NOT NULL,
    player_guid2 BIGINT NOT NULL, new_rank SMALLINT NOT NULL, timestamp BIGINT NOT NULL,
    PRIMARY KEY (guild_id, log_guid)
);
CREATE INDEX idx_guild_eventlog_player1 ON guild_eventlog (player_guid1);
CREATE INDEX idx_guild_eventlog_player2 ON guild_eventlog (player_guid2);

CREATE TABLE corpse (
    guid BIGINT PRIMARY KEY CHECK (guid >= 0),
    player_guid BIGINT NOT NULL REFERENCES characters (guid) ON DELETE CASCADE,
    position_x REAL NOT NULL DEFAULT 0, position_y REAL NOT NULL DEFAULT 0,
    position_z REAL NOT NULL DEFAULT 0, orientation REAL NOT NULL DEFAULT 0,
    map BIGINT NOT NULL DEFAULT 0 CHECK (map >= 0), time BIGINT NOT NULL DEFAULT 0 CHECK (time >= 0),
    corpse_type SMALLINT NOT NULL DEFAULT 0 CHECK (corpse_type >= 0),
    instance BIGINT NOT NULL DEFAULT 0 CHECK (instance >= 0)
);
CREATE INDEX idx_corpse_player ON corpse (player_guid);
CREATE INDEX idx_corpse_instance ON corpse (instance);
CREATE TABLE character_battleground_data (
    guid BIGINT PRIMARY KEY REFERENCES characters (guid) ON DELETE CASCADE,
    instance_id BIGINT NOT NULL DEFAULT 0 CHECK (instance_id >= 0),
    team SMALLINT NOT NULL DEFAULT 0 CHECK (team >= 0), join_x REAL NOT NULL DEFAULT 0,
    join_y REAL NOT NULL DEFAULT 0, join_z REAL NOT NULL DEFAULT 0, join_o REAL NOT NULL DEFAULT 0,
    join_map BIGINT NOT NULL DEFAULT 0 CHECK (join_map >= 0)
);
CREATE INDEX idx_character_battleground_instance ON character_battleground_data (instance_id);

CREATE TABLE gm_tickets (
    ticket_id BIGINT PRIMARY KEY CHECK (ticket_id >= 0),
    guid BIGINT NOT NULL DEFAULT 0 CHECK (guid >= 0), name TEXT NOT NULL, message TEXT NOT NULL,
    create_time BIGINT NOT NULL DEFAULT 0 CHECK (create_time >= 0), map BIGINT NOT NULL DEFAULT 0 CHECK (map >= 0),
    position_x REAL NOT NULL DEFAULT 0, position_y REAL NOT NULL DEFAULT 0, position_z REAL NOT NULL DEFAULT 0,
    last_modified_time BIGINT NOT NULL DEFAULT 0 CHECK (last_modified_time >= 0), closed_by BIGINT NOT NULL DEFAULT 0,
    assigned_to BIGINT NOT NULL DEFAULT 0 CHECK (assigned_to >= 0), comment TEXT NOT NULL DEFAULT '',
    response TEXT NOT NULL DEFAULT '', completed BOOLEAN NOT NULL DEFAULT FALSE,
    escalated SMALLINT NOT NULL DEFAULT 0 CHECK (escalated >= 0), viewed BOOLEAN NOT NULL DEFAULT FALSE,
    have_ticket BOOLEAN NOT NULL DEFAULT FALSE, ticket_type SMALLINT NOT NULL DEFAULT 0 CHECK (ticket_type >= 0),
    security_needed BIGINT NOT NULL DEFAULT 0 CHECK (security_needed >= 0)
);
CREATE TABLE gm_surveys (
    survey_id BIGINT PRIMARY KEY CHECK (survey_id >= 0), ticket_id BIGINT NOT NULL,
    main_survey SMALLINT NOT NULL DEFAULT 0 CHECK (main_survey >= 0),
    overall_comment TEXT NOT NULL DEFAULT '', response_time BIGINT NOT NULL DEFAULT 0 CHECK (response_time >= 0)
);

CREATE TABLE petition (
    owner_guid BIGINT PRIMARY KEY REFERENCES characters (guid) ON DELETE CASCADE,
    petition_guid BIGINT NOT NULL DEFAULT 0 CHECK (petition_guid >= 0),
    charter_guid BIGINT UNIQUE, name TEXT NOT NULL DEFAULT '', UNIQUE (owner_guid, petition_guid)
);
CREATE TABLE petition_sign (
    owner_guid BIGINT NOT NULL, petition_guid BIGINT NOT NULL,
    player_guid BIGINT NOT NULL REFERENCES characters (guid) ON DELETE CASCADE,
    player_account BIGINT NOT NULL DEFAULT 0 CHECK (player_account >= 0), PRIMARY KEY (petition_guid, player_guid)
);
CREATE INDEX idx_petition_sign_owner ON petition_sign (owner_guid);
CREATE TABLE character_honor_cp (
    guid BIGINT NOT NULL REFERENCES characters (guid) ON DELETE CASCADE,
    victim_type SMALLINT NOT NULL DEFAULT 4 CHECK (victim_type >= 0),
    victim_id BIGINT NOT NULL DEFAULT 0 CHECK (victim_id >= 0), cp REAL NOT NULL DEFAULT 0,
    date BIGINT NOT NULL DEFAULT 0 CHECK (date >= 0), type SMALLINT NOT NULL DEFAULT 0 CHECK (type >= 0)
);
CREATE INDEX idx_character_honor_cp_guid ON character_honor_cp (guid);

CREATE TABLE instance (
    id BIGINT PRIMARY KEY CHECK (id >= 0), map BIGINT NOT NULL DEFAULT 0 CHECK (map >= 0),
    reset_time BIGINT NOT NULL DEFAULT 0, data TEXT
);
CREATE INDEX idx_instance_map ON instance (map);
CREATE INDEX idx_instance_reset_time ON instance (reset_time);
CREATE TABLE character_instance (
    guid BIGINT NOT NULL REFERENCES characters (guid) ON DELETE CASCADE,
    instance BIGINT NOT NULL REFERENCES instance (id) ON DELETE CASCADE,
    permanent SMALLINT NOT NULL DEFAULT 0 CHECK (permanent >= 0),
    extend SMALLINT NOT NULL DEFAULT 0 CHECK (extend >= 0), PRIMARY KEY (guid, instance)
);
CREATE INDEX idx_character_instance_instance ON character_instance (instance);
ALTER TABLE group_instance ADD CONSTRAINT group_instance_instance_fkey
    FOREIGN KEY (instance) REFERENCES instance (id) ON DELETE CASCADE;
-- Consolidated legacy table coverage retained from the previous setup migration
-- Generated schema DDL only; it contains no reference data.

CREATE TABLE IF NOT EXISTS "account_data" (
    "account" BIGINT NOT NULL DEFAULT '0' CHECK ("account" >= 0),
    "type" BIGINT NOT NULL DEFAULT '0' CHECK ("type" >= 0),
    "time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("time" >= 0),
    "data" BYTEA NOT NULL,
    PRIMARY KEY ("account", "type")
);
ALTER TABLE IF EXISTS "account_data" ADD COLUMN IF NOT EXISTS "account" BIGINT NOT NULL DEFAULT '0' CHECK ("account" >= 0);
ALTER TABLE IF EXISTS "account_data" ADD COLUMN IF NOT EXISTS "type" BIGINT NOT NULL DEFAULT '0' CHECK ("type" >= 0);
ALTER TABLE IF EXISTS "account_data" ADD COLUMN IF NOT EXISTS "time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("time" >= 0);
ALTER TABLE IF EXISTS "account_data" ADD COLUMN IF NOT EXISTS "data" BYTEA NOT NULL;

CREATE TABLE IF NOT EXISTS "auction" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "house_id" BIGINT NOT NULL DEFAULT '0' CHECK ("house_id" >= 0),
    "item_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("item_guid" >= 0),
    "item_id" BIGINT NOT NULL DEFAULT '0' CHECK ("item_id" >= 0),
    "seller_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("seller_guid" >= 0),
    "buyout_price" INTEGER NOT NULL DEFAULT '0',
    "expire_time" BIGINT NOT NULL DEFAULT '0',
    "buyer_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("buyer_guid" >= 0),
    "last_bid" INTEGER NOT NULL DEFAULT '0',
    "start_bid" INTEGER NOT NULL DEFAULT '0',
    "deposit" INTEGER NOT NULL DEFAULT '0',
    PRIMARY KEY ("id"),
    UNIQUE ("item_guid")
);
ALTER TABLE IF EXISTS "auction" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "auction" ADD COLUMN IF NOT EXISTS "house_id" BIGINT NOT NULL DEFAULT '0' CHECK ("house_id" >= 0);
ALTER TABLE IF EXISTS "auction" ADD COLUMN IF NOT EXISTS "item_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("item_guid" >= 0);
ALTER TABLE IF EXISTS "auction" ADD COLUMN IF NOT EXISTS "item_id" BIGINT NOT NULL DEFAULT '0' CHECK ("item_id" >= 0);
ALTER TABLE IF EXISTS "auction" ADD COLUMN IF NOT EXISTS "seller_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("seller_guid" >= 0);
ALTER TABLE IF EXISTS "auction" ADD COLUMN IF NOT EXISTS "buyout_price" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "auction" ADD COLUMN IF NOT EXISTS "expire_time" BIGINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "auction" ADD COLUMN IF NOT EXISTS "buyer_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("buyer_guid" >= 0);
ALTER TABLE IF EXISTS "auction" ADD COLUMN IF NOT EXISTS "last_bid" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "auction" ADD COLUMN IF NOT EXISTS "start_bid" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "auction" ADD COLUMN IF NOT EXISTS "deposit" INTEGER NOT NULL DEFAULT '0';
CREATE UNIQUE INDEX IF NOT EXISTS idx_auction_key_item_guid ON "auction" ("item_guid");

CREATE TABLE IF NOT EXISTS "character_account_data" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "type" BIGINT NOT NULL DEFAULT '0' CHECK ("type" >= 0),
    "time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("time" >= 0),
    "data" BYTEA NOT NULL,
    PRIMARY KEY ("guid", "type")
);
ALTER TABLE IF EXISTS "character_account_data" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "character_account_data" ADD COLUMN IF NOT EXISTS "type" BIGINT NOT NULL DEFAULT '0' CHECK ("type" >= 0);
ALTER TABLE IF EXISTS "character_account_data" ADD COLUMN IF NOT EXISTS "time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("time" >= 0);
ALTER TABLE IF EXISTS "character_account_data" ADD COLUMN IF NOT EXISTS "data" BYTEA NOT NULL;

CREATE TABLE IF NOT EXISTS "character_action" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "button" SMALLINT NOT NULL DEFAULT '0' CHECK ("button" >= 0),
    "action" BIGINT NOT NULL DEFAULT '0' CHECK ("action" >= 0),
    "type" SMALLINT NOT NULL DEFAULT '0' CHECK ("type" >= 0),
    PRIMARY KEY ("guid", "button")
);
ALTER TABLE IF EXISTS "character_action" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "character_action" ADD COLUMN IF NOT EXISTS "button" SMALLINT NOT NULL DEFAULT '0' CHECK ("button" >= 0);
ALTER TABLE IF EXISTS "character_action" ADD COLUMN IF NOT EXISTS "action" BIGINT NOT NULL DEFAULT '0' CHECK ("action" >= 0);
ALTER TABLE IF EXISTS "character_action" ADD COLUMN IF NOT EXISTS "type" SMALLINT NOT NULL DEFAULT '0' CHECK ("type" >= 0);

CREATE TABLE IF NOT EXISTS "character_aura" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "caster_guid" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("caster_guid" >= 0),
    "item_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("item_guid" >= 0),
    "spell" BIGINT NOT NULL DEFAULT '0' CHECK ("spell" >= 0),
    "stacks" BIGINT NOT NULL DEFAULT '1' CHECK ("stacks" >= 0),
    "charges" BIGINT NOT NULL DEFAULT '0' CHECK ("charges" >= 0),
    "base_points0" REAL NOT NULL DEFAULT '0',
    "base_points1" REAL NOT NULL DEFAULT '0',
    "base_points2" REAL NOT NULL DEFAULT '0',
    "periodic_time0" BIGINT NOT NULL DEFAULT '0' CHECK ("periodic_time0" >= 0),
    "periodic_time1" BIGINT NOT NULL DEFAULT '0' CHECK ("periodic_time1" >= 0),
    "periodic_time2" BIGINT NOT NULL DEFAULT '0' CHECK ("periodic_time2" >= 0),
    "max_duration" INTEGER NOT NULL DEFAULT '0',
    "duration" INTEGER NOT NULL DEFAULT '0',
    "effect_index_mask" SMALLINT NOT NULL DEFAULT '0' CHECK ("effect_index_mask" >= 0),
    PRIMARY KEY ("guid", "caster_guid", "item_guid", "spell")
);
ALTER TABLE IF EXISTS "character_aura" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "character_aura" ADD COLUMN IF NOT EXISTS "caster_guid" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("caster_guid" >= 0);
ALTER TABLE IF EXISTS "character_aura" ADD COLUMN IF NOT EXISTS "item_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("item_guid" >= 0);
ALTER TABLE IF EXISTS "character_aura" ADD COLUMN IF NOT EXISTS "spell" BIGINT NOT NULL DEFAULT '0' CHECK ("spell" >= 0);
ALTER TABLE IF EXISTS "character_aura" ADD COLUMN IF NOT EXISTS "stacks" BIGINT NOT NULL DEFAULT '1' CHECK ("stacks" >= 0);
ALTER TABLE IF EXISTS "character_aura" ADD COLUMN IF NOT EXISTS "charges" BIGINT NOT NULL DEFAULT '0' CHECK ("charges" >= 0);
ALTER TABLE IF EXISTS "character_aura" ADD COLUMN IF NOT EXISTS "base_points0" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_aura" ADD COLUMN IF NOT EXISTS "base_points1" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_aura" ADD COLUMN IF NOT EXISTS "base_points2" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_aura" ADD COLUMN IF NOT EXISTS "periodic_time0" BIGINT NOT NULL DEFAULT '0' CHECK ("periodic_time0" >= 0);
ALTER TABLE IF EXISTS "character_aura" ADD COLUMN IF NOT EXISTS "periodic_time1" BIGINT NOT NULL DEFAULT '0' CHECK ("periodic_time1" >= 0);
ALTER TABLE IF EXISTS "character_aura" ADD COLUMN IF NOT EXISTS "periodic_time2" BIGINT NOT NULL DEFAULT '0' CHECK ("periodic_time2" >= 0);
ALTER TABLE IF EXISTS "character_aura" ADD COLUMN IF NOT EXISTS "max_duration" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_aura" ADD COLUMN IF NOT EXISTS "duration" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_aura" ADD COLUMN IF NOT EXISTS "effect_index_mask" SMALLINT NOT NULL DEFAULT '0' CHECK ("effect_index_mask" >= 0);

CREATE TABLE IF NOT EXISTS "character_battleground_data" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "instance_id" BIGINT NOT NULL DEFAULT '0' CHECK ("instance_id" >= 0),
    "team" SMALLINT NOT NULL DEFAULT '0' CHECK ("team" >= 0),
    "join_x" REAL NOT NULL DEFAULT '0',
    "join_y" REAL NOT NULL DEFAULT '0',
    "join_z" REAL NOT NULL DEFAULT '0',
    "join_o" REAL NOT NULL DEFAULT '0',
    "join_map" BIGINT NOT NULL DEFAULT '0',
    PRIMARY KEY ("guid")
);
ALTER TABLE IF EXISTS "character_battleground_data" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "character_battleground_data" ADD COLUMN IF NOT EXISTS "instance_id" BIGINT NOT NULL DEFAULT '0' CHECK ("instance_id" >= 0);
ALTER TABLE IF EXISTS "character_battleground_data" ADD COLUMN IF NOT EXISTS "team" SMALLINT NOT NULL DEFAULT '0' CHECK ("team" >= 0);
ALTER TABLE IF EXISTS "character_battleground_data" ADD COLUMN IF NOT EXISTS "join_x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_battleground_data" ADD COLUMN IF NOT EXISTS "join_y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_battleground_data" ADD COLUMN IF NOT EXISTS "join_z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_battleground_data" ADD COLUMN IF NOT EXISTS "join_o" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_battleground_data" ADD COLUMN IF NOT EXISTS "join_map" BIGINT NOT NULL DEFAULT '0';

CREATE TABLE IF NOT EXISTS "character_deleted_items" (
    "id" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("id" >= 0),
    "player_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("player_guid" >= 0),
    "item_id" BIGINT NOT NULL DEFAULT '0' CHECK ("item_id" >= 0),
    "stack_count" BIGINT NOT NULL DEFAULT '1' CHECK ("stack_count" >= 0),
    PRIMARY KEY ("id")
);
ALTER TABLE IF EXISTS "character_deleted_items" ADD COLUMN IF NOT EXISTS "id" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "character_deleted_items" ADD COLUMN IF NOT EXISTS "player_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("player_guid" >= 0);
ALTER TABLE IF EXISTS "character_deleted_items" ADD COLUMN IF NOT EXISTS "item_id" BIGINT NOT NULL DEFAULT '0' CHECK ("item_id" >= 0);
ALTER TABLE IF EXISTS "character_deleted_items" ADD COLUMN IF NOT EXISTS "stack_count" BIGINT NOT NULL DEFAULT '1' CHECK ("stack_count" >= 0);
CREATE INDEX IF NOT EXISTS idx_character_deleted_items_idx_playerguid ON "character_deleted_items" ("player_guid");

CREATE TABLE IF NOT EXISTS "character_duplicate_account" (
    "account" INTEGER DEFAULT NULL
);
ALTER TABLE IF EXISTS "character_duplicate_account" ADD COLUMN IF NOT EXISTS "account" INTEGER DEFAULT NULL;

CREATE TABLE IF NOT EXISTS "character_forgotten_skills" (
    "guid" BIGINT NOT NULL CHECK ("guid" >= 0),
    "skill" BIGINT NOT NULL CHECK ("skill" >= 0),
    "value" BIGINT NOT NULL CHECK ("value" >= 0),
    PRIMARY KEY ("guid", "skill")
);
ALTER TABLE IF EXISTS "character_forgotten_skills" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "character_forgotten_skills" ADD COLUMN IF NOT EXISTS "skill" BIGINT NOT NULL CHECK ("skill" >= 0);
ALTER TABLE IF EXISTS "character_forgotten_skills" ADD COLUMN IF NOT EXISTS "value" BIGINT NOT NULL CHECK ("value" >= 0);

CREATE TABLE IF NOT EXISTS "character_gifts" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "item_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("item_guid" >= 0),
    "item_id" BIGINT NOT NULL DEFAULT '0' CHECK ("item_id" >= 0),
    "flags" BIGINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0),
    PRIMARY KEY ("item_guid")
);
ALTER TABLE IF EXISTS "character_gifts" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "character_gifts" ADD COLUMN IF NOT EXISTS "item_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("item_guid" >= 0);
ALTER TABLE IF EXISTS "character_gifts" ADD COLUMN IF NOT EXISTS "item_id" BIGINT NOT NULL DEFAULT '0' CHECK ("item_id" >= 0);
ALTER TABLE IF EXISTS "character_gifts" ADD COLUMN IF NOT EXISTS "flags" BIGINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0);
CREATE INDEX IF NOT EXISTS idx_character_gifts_idx_guid ON "character_gifts" ("guid");

CREATE TABLE IF NOT EXISTS "character_homebind" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "map" BIGINT NOT NULL DEFAULT '0' CHECK ("map" >= 0),
    "zone" BIGINT NOT NULL DEFAULT '0' CHECK ("zone" >= 0),
    "position_x" REAL NOT NULL DEFAULT '0',
    "position_y" REAL NOT NULL DEFAULT '0',
    "position_z" REAL NOT NULL DEFAULT '0',
    PRIMARY KEY ("guid")
);
ALTER TABLE IF EXISTS "character_homebind" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "character_homebind" ADD COLUMN IF NOT EXISTS "map" BIGINT NOT NULL DEFAULT '0' CHECK ("map" >= 0);
ALTER TABLE IF EXISTS "character_homebind" ADD COLUMN IF NOT EXISTS "zone" BIGINT NOT NULL DEFAULT '0' CHECK ("zone" >= 0);
ALTER TABLE IF EXISTS "character_homebind" ADD COLUMN IF NOT EXISTS "position_x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_homebind" ADD COLUMN IF NOT EXISTS "position_y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_homebind" ADD COLUMN IF NOT EXISTS "position_z" REAL NOT NULL DEFAULT '0';

CREATE TABLE IF NOT EXISTS "character_honor_cp" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "victim_type" SMALLINT NOT NULL DEFAULT '4' CHECK ("victim_type" >= 0),
    "victim_id" BIGINT NOT NULL DEFAULT '0' CHECK ("victim_id" >= 0),
    "cp" REAL NOT NULL DEFAULT '0',
    "date" BIGINT NOT NULL DEFAULT '0' CHECK ("date" >= 0),
    "type" SMALLINT NOT NULL DEFAULT '0' CHECK ("type" >= 0)
);
ALTER TABLE IF EXISTS "character_honor_cp" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "character_honor_cp" ADD COLUMN IF NOT EXISTS "victim_type" SMALLINT NOT NULL DEFAULT '4' CHECK ("victim_type" >= 0);
ALTER TABLE IF EXISTS "character_honor_cp" ADD COLUMN IF NOT EXISTS "victim_id" BIGINT NOT NULL DEFAULT '0' CHECK ("victim_id" >= 0);
ALTER TABLE IF EXISTS "character_honor_cp" ADD COLUMN IF NOT EXISTS "cp" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_honor_cp" ADD COLUMN IF NOT EXISTS "date" BIGINT NOT NULL DEFAULT '0' CHECK ("date" >= 0);
ALTER TABLE IF EXISTS "character_honor_cp" ADD COLUMN IF NOT EXISTS "type" SMALLINT NOT NULL DEFAULT '0' CHECK ("type" >= 0);
CREATE INDEX IF NOT EXISTS idx_character_honor_cp_idx_guid ON "character_honor_cp" ("guid");

CREATE TABLE IF NOT EXISTS "character_instance" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "instance" BIGINT NOT NULL DEFAULT '0' CHECK ("instance" >= 0),
    "permanent" SMALLINT NOT NULL DEFAULT '0' CHECK ("permanent" >= 0),
    PRIMARY KEY ("guid", "instance")
);
ALTER TABLE IF EXISTS "character_instance" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "character_instance" ADD COLUMN IF NOT EXISTS "instance" BIGINT NOT NULL DEFAULT '0' CHECK ("instance" >= 0);
ALTER TABLE IF EXISTS "character_instance" ADD COLUMN IF NOT EXISTS "permanent" SMALLINT NOT NULL DEFAULT '0' CHECK ("permanent" >= 0);
CREATE INDEX IF NOT EXISTS idx_character_instance_idx_instance ON "character_instance" ("instance");

CREATE TABLE IF NOT EXISTS "character_inventory" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "bag" BIGINT NOT NULL DEFAULT '0' CHECK ("bag" >= 0),
    "slot" SMALLINT NOT NULL DEFAULT '0' CHECK ("slot" >= 0),
    "item_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("item_guid" >= 0),
    "item_id" BIGINT NOT NULL DEFAULT '0' CHECK ("item_id" >= 0),
    PRIMARY KEY ("item_guid")
);
ALTER TABLE IF EXISTS "character_inventory" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "character_inventory" ADD COLUMN IF NOT EXISTS "bag" BIGINT NOT NULL DEFAULT '0' CHECK ("bag" >= 0);
ALTER TABLE IF EXISTS "character_inventory" ADD COLUMN IF NOT EXISTS "slot" SMALLINT NOT NULL DEFAULT '0' CHECK ("slot" >= 0);
ALTER TABLE IF EXISTS "character_inventory" ADD COLUMN IF NOT EXISTS "item_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("item_guid" >= 0);
ALTER TABLE IF EXISTS "character_inventory" ADD COLUMN IF NOT EXISTS "item_id" BIGINT NOT NULL DEFAULT '0' CHECK ("item_id" >= 0);
CREATE INDEX IF NOT EXISTS idx_character_inventory_idx_guid ON "character_inventory" ("guid");

CREATE TABLE IF NOT EXISTS "character_pet" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0),
    "owner_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("owner_guid" >= 0),
    "display_id" BIGINT DEFAULT '0' CHECK ("display_id" >= 0),
    "created_by_spell" BIGINT NOT NULL DEFAULT '0' CHECK ("created_by_spell" >= 0),
    "pet_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("pet_type" >= 0),
    "level" BIGINT NOT NULL DEFAULT '1' CHECK ("level" >= 0),
    "xp" BIGINT NOT NULL DEFAULT '0' CHECK ("xp" >= 0),
    "react_state" SMALLINT NOT NULL DEFAULT '0' CHECK ("react_state" >= 0),
    "loyalty_points" INTEGER NOT NULL DEFAULT '0',
    "loyalty" BIGINT NOT NULL DEFAULT '0' CHECK ("loyalty" >= 0),
    "training_points" INTEGER NOT NULL DEFAULT '0',
    "name" VARCHAR(100) DEFAULT 'Pet',
    "renamed" SMALLINT NOT NULL DEFAULT '0' CHECK ("renamed" >= 0),
    "slot" BIGINT NOT NULL DEFAULT '0' CHECK ("slot" >= 0),
    "current_health" BIGINT NOT NULL DEFAULT '1' CHECK ("current_health" >= 0),
    "current_mana" BIGINT NOT NULL DEFAULT '0' CHECK ("current_mana" >= 0),
    "current_happiness" BIGINT NOT NULL DEFAULT '0' CHECK ("current_happiness" >= 0),
    "save_time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("save_time" >= 0),
    "reset_talents_cost" BIGINT NOT NULL DEFAULT '0' CHECK ("reset_talents_cost" >= 0),
    "reset_talents_time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("reset_talents_time" >= 0),
    "action_bar_data" TEXT,
    "teach_spell_data" TEXT,
    PRIMARY KEY ("id")
);
ALTER TABLE IF EXISTS "character_pet" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "character_pet" ADD COLUMN IF NOT EXISTS "entry" BIGINT NOT NULL DEFAULT '0' CHECK ("entry" >= 0);
ALTER TABLE IF EXISTS "character_pet" ADD COLUMN IF NOT EXISTS "owner_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("owner_guid" >= 0);
ALTER TABLE IF EXISTS "character_pet" ADD COLUMN IF NOT EXISTS "display_id" BIGINT DEFAULT '0' CHECK ("display_id" >= 0);
ALTER TABLE IF EXISTS "character_pet" ADD COLUMN IF NOT EXISTS "created_by_spell" BIGINT NOT NULL DEFAULT '0' CHECK ("created_by_spell" >= 0);
ALTER TABLE IF EXISTS "character_pet" ADD COLUMN IF NOT EXISTS "pet_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("pet_type" >= 0);
ALTER TABLE IF EXISTS "character_pet" ADD COLUMN IF NOT EXISTS "level" BIGINT NOT NULL DEFAULT '1' CHECK ("level" >= 0);
ALTER TABLE IF EXISTS "character_pet" ADD COLUMN IF NOT EXISTS "xp" BIGINT NOT NULL DEFAULT '0' CHECK ("xp" >= 0);
ALTER TABLE IF EXISTS "character_pet" ADD COLUMN IF NOT EXISTS "react_state" SMALLINT NOT NULL DEFAULT '0' CHECK ("react_state" >= 0);
ALTER TABLE IF EXISTS "character_pet" ADD COLUMN IF NOT EXISTS "loyalty_points" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_pet" ADD COLUMN IF NOT EXISTS "loyalty" BIGINT NOT NULL DEFAULT '0' CHECK ("loyalty" >= 0);
ALTER TABLE IF EXISTS "character_pet" ADD COLUMN IF NOT EXISTS "training_points" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_pet" ADD COLUMN IF NOT EXISTS "name" VARCHAR(100) DEFAULT 'Pet';
ALTER TABLE IF EXISTS "character_pet" ADD COLUMN IF NOT EXISTS "renamed" SMALLINT NOT NULL DEFAULT '0' CHECK ("renamed" >= 0);
ALTER TABLE IF EXISTS "character_pet" ADD COLUMN IF NOT EXISTS "slot" BIGINT NOT NULL DEFAULT '0' CHECK ("slot" >= 0);
ALTER TABLE IF EXISTS "character_pet" ADD COLUMN IF NOT EXISTS "current_health" BIGINT NOT NULL DEFAULT '1' CHECK ("current_health" >= 0);
ALTER TABLE IF EXISTS "character_pet" ADD COLUMN IF NOT EXISTS "current_mana" BIGINT NOT NULL DEFAULT '0' CHECK ("current_mana" >= 0);
ALTER TABLE IF EXISTS "character_pet" ADD COLUMN IF NOT EXISTS "current_happiness" BIGINT NOT NULL DEFAULT '0' CHECK ("current_happiness" >= 0);
ALTER TABLE IF EXISTS "character_pet" ADD COLUMN IF NOT EXISTS "save_time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("save_time" >= 0);
ALTER TABLE IF EXISTS "character_pet" ADD COLUMN IF NOT EXISTS "reset_talents_cost" BIGINT NOT NULL DEFAULT '0' CHECK ("reset_talents_cost" >= 0);
ALTER TABLE IF EXISTS "character_pet" ADD COLUMN IF NOT EXISTS "reset_talents_time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("reset_talents_time" >= 0);
ALTER TABLE IF EXISTS "character_pet" ADD COLUMN IF NOT EXISTS "action_bar_data" TEXT;
ALTER TABLE IF EXISTS "character_pet" ADD COLUMN IF NOT EXISTS "teach_spell_data" TEXT;
CREATE INDEX IF NOT EXISTS idx_character_pet_idx_owner ON "character_pet" ("owner_guid");

CREATE TABLE IF NOT EXISTS "character_queststatus" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "quest" BIGINT NOT NULL DEFAULT '0' CHECK ("quest" >= 0),
    "status" BIGINT NOT NULL DEFAULT '0' CHECK ("status" >= 0),
    "rewarded" SMALLINT NOT NULL DEFAULT '0' CHECK ("rewarded" >= 0),
    "explored" SMALLINT NOT NULL DEFAULT '0' CHECK ("explored" >= 0),
    "timer" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("timer" >= 0),
    "mob_count1" BIGINT NOT NULL DEFAULT '0' CHECK ("mob_count1" >= 0),
    "mob_count2" BIGINT NOT NULL DEFAULT '0' CHECK ("mob_count2" >= 0),
    "mob_count3" BIGINT NOT NULL DEFAULT '0' CHECK ("mob_count3" >= 0),
    "mob_count4" BIGINT NOT NULL DEFAULT '0' CHECK ("mob_count4" >= 0),
    "item_count1" BIGINT NOT NULL DEFAULT '0' CHECK ("item_count1" >= 0),
    "item_count2" BIGINT NOT NULL DEFAULT '0' CHECK ("item_count2" >= 0),
    "item_count3" BIGINT NOT NULL DEFAULT '0' CHECK ("item_count3" >= 0),
    "item_count4" BIGINT NOT NULL DEFAULT '0' CHECK ("item_count4" >= 0),
    "reward_choice" BIGINT NOT NULL DEFAULT '0' CHECK ("reward_choice" >= 0),
    PRIMARY KEY ("guid", "quest")
);
ALTER TABLE IF EXISTS "character_queststatus" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "character_queststatus" ADD COLUMN IF NOT EXISTS "quest" BIGINT NOT NULL DEFAULT '0' CHECK ("quest" >= 0);
ALTER TABLE IF EXISTS "character_queststatus" ADD COLUMN IF NOT EXISTS "status" BIGINT NOT NULL DEFAULT '0' CHECK ("status" >= 0);
ALTER TABLE IF EXISTS "character_queststatus" ADD COLUMN IF NOT EXISTS "rewarded" SMALLINT NOT NULL DEFAULT '0' CHECK ("rewarded" >= 0);
ALTER TABLE IF EXISTS "character_queststatus" ADD COLUMN IF NOT EXISTS "explored" SMALLINT NOT NULL DEFAULT '0' CHECK ("explored" >= 0);
ALTER TABLE IF EXISTS "character_queststatus" ADD COLUMN IF NOT EXISTS "timer" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("timer" >= 0);
ALTER TABLE IF EXISTS "character_queststatus" ADD COLUMN IF NOT EXISTS "mob_count1" BIGINT NOT NULL DEFAULT '0' CHECK ("mob_count1" >= 0);
ALTER TABLE IF EXISTS "character_queststatus" ADD COLUMN IF NOT EXISTS "mob_count2" BIGINT NOT NULL DEFAULT '0' CHECK ("mob_count2" >= 0);
ALTER TABLE IF EXISTS "character_queststatus" ADD COLUMN IF NOT EXISTS "mob_count3" BIGINT NOT NULL DEFAULT '0' CHECK ("mob_count3" >= 0);
ALTER TABLE IF EXISTS "character_queststatus" ADD COLUMN IF NOT EXISTS "mob_count4" BIGINT NOT NULL DEFAULT '0' CHECK ("mob_count4" >= 0);
ALTER TABLE IF EXISTS "character_queststatus" ADD COLUMN IF NOT EXISTS "item_count1" BIGINT NOT NULL DEFAULT '0' CHECK ("item_count1" >= 0);
ALTER TABLE IF EXISTS "character_queststatus" ADD COLUMN IF NOT EXISTS "item_count2" BIGINT NOT NULL DEFAULT '0' CHECK ("item_count2" >= 0);
ALTER TABLE IF EXISTS "character_queststatus" ADD COLUMN IF NOT EXISTS "item_count3" BIGINT NOT NULL DEFAULT '0' CHECK ("item_count3" >= 0);
ALTER TABLE IF EXISTS "character_queststatus" ADD COLUMN IF NOT EXISTS "item_count4" BIGINT NOT NULL DEFAULT '0' CHECK ("item_count4" >= 0);
ALTER TABLE IF EXISTS "character_queststatus" ADD COLUMN IF NOT EXISTS "reward_choice" BIGINT NOT NULL DEFAULT '0' CHECK ("reward_choice" >= 0);

CREATE TABLE IF NOT EXISTS "character_reputation" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "faction" BIGINT NOT NULL DEFAULT '0' CHECK ("faction" >= 0),
    "standing" INTEGER NOT NULL DEFAULT '0',
    "flags" INTEGER NOT NULL DEFAULT '0',
    PRIMARY KEY ("guid", "faction")
);
ALTER TABLE IF EXISTS "character_reputation" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "character_reputation" ADD COLUMN IF NOT EXISTS "faction" BIGINT NOT NULL DEFAULT '0' CHECK ("faction" >= 0);
ALTER TABLE IF EXISTS "character_reputation" ADD COLUMN IF NOT EXISTS "standing" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_reputation" ADD COLUMN IF NOT EXISTS "flags" INTEGER NOT NULL DEFAULT '0';

CREATE TABLE IF NOT EXISTS "character_skills" (
    "guid" BIGINT NOT NULL CHECK ("guid" >= 0),
    "skill" BIGINT NOT NULL CHECK ("skill" >= 0),
    "value" BIGINT NOT NULL CHECK ("value" >= 0),
    "max" BIGINT NOT NULL CHECK ("max" >= 0),
    PRIMARY KEY ("guid", "skill")
);
ALTER TABLE IF EXISTS "character_skills" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "character_skills" ADD COLUMN IF NOT EXISTS "skill" BIGINT NOT NULL CHECK ("skill" >= 0);
ALTER TABLE IF EXISTS "character_skills" ADD COLUMN IF NOT EXISTS "value" BIGINT NOT NULL CHECK ("value" >= 0);
ALTER TABLE IF EXISTS "character_skills" ADD COLUMN IF NOT EXISTS "max" BIGINT NOT NULL CHECK ("max" >= 0);

CREATE TABLE IF NOT EXISTS "character_social" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "friend" BIGINT NOT NULL DEFAULT '0' CHECK ("friend" >= 0),
    "flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0),
    PRIMARY KEY ("guid", "friend")
);
ALTER TABLE IF EXISTS "character_social" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "character_social" ADD COLUMN IF NOT EXISTS "friend" BIGINT NOT NULL DEFAULT '0' CHECK ("friend" >= 0);
ALTER TABLE IF EXISTS "character_social" ADD COLUMN IF NOT EXISTS "flags" SMALLINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0);
CREATE INDEX IF NOT EXISTS idx_character_social_idx_guid ON "character_social" ("guid");
CREATE INDEX IF NOT EXISTS idx_character_social_idx_friend ON "character_social" ("friend");
CREATE INDEX IF NOT EXISTS idx_character_social_idx_guid_flags ON "character_social" ("guid", "flags");
CREATE INDEX IF NOT EXISTS idx_character_social_idx_friend_flags ON "character_social" ("friend", "flags");

CREATE TABLE IF NOT EXISTS "character_spell" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "spell" BIGINT NOT NULL DEFAULT '0' CHECK ("spell" >= 0),
    "active" SMALLINT NOT NULL DEFAULT '1' CHECK ("active" >= 0),
    "disabled" SMALLINT NOT NULL DEFAULT '0' CHECK ("disabled" >= 0),
    PRIMARY KEY ("guid", "spell")
);
ALTER TABLE IF EXISTS "character_spell" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "character_spell" ADD COLUMN IF NOT EXISTS "spell" BIGINT NOT NULL DEFAULT '0' CHECK ("spell" >= 0);
ALTER TABLE IF EXISTS "character_spell" ADD COLUMN IF NOT EXISTS "active" SMALLINT NOT NULL DEFAULT '1' CHECK ("active" >= 0);
ALTER TABLE IF EXISTS "character_spell" ADD COLUMN IF NOT EXISTS "disabled" SMALLINT NOT NULL DEFAULT '0' CHECK ("disabled" >= 0);
CREATE INDEX IF NOT EXISTS idx_character_spell_idx_spell ON "character_spell" ("spell");

CREATE TABLE IF NOT EXISTS "character_spell_cooldown" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "spell" BIGINT NOT NULL DEFAULT '0' CHECK ("spell" >= 0),
    "spell_expire_time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("spell_expire_time" >= 0),
    "category" BIGINT NOT NULL DEFAULT '0' CHECK ("category" >= 0),
    "category_expire_time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("category_expire_time" >= 0),
    "item_id" BIGINT NOT NULL DEFAULT '0' CHECK ("item_id" >= 0),
    PRIMARY KEY ("guid", "spell")
);
ALTER TABLE IF EXISTS "character_spell_cooldown" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "character_spell_cooldown" ADD COLUMN IF NOT EXISTS "spell" BIGINT NOT NULL DEFAULT '0' CHECK ("spell" >= 0);
ALTER TABLE IF EXISTS "character_spell_cooldown" ADD COLUMN IF NOT EXISTS "spell_expire_time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("spell_expire_time" >= 0);
ALTER TABLE IF EXISTS "character_spell_cooldown" ADD COLUMN IF NOT EXISTS "category" BIGINT NOT NULL DEFAULT '0' CHECK ("category" >= 0);
ALTER TABLE IF EXISTS "character_spell_cooldown" ADD COLUMN IF NOT EXISTS "category_expire_time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("category_expire_time" >= 0);
ALTER TABLE IF EXISTS "character_spell_cooldown" ADD COLUMN IF NOT EXISTS "item_id" BIGINT NOT NULL DEFAULT '0' CHECK ("item_id" >= 0);

CREATE TABLE IF NOT EXISTS "character_stats" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "max_health" BIGINT NOT NULL DEFAULT '0' CHECK ("max_health" >= 0),
    "max_power1" BIGINT NOT NULL DEFAULT '0' CHECK ("max_power1" >= 0),
    "max_power2" BIGINT NOT NULL DEFAULT '0' CHECK ("max_power2" >= 0),
    "max_power3" BIGINT NOT NULL DEFAULT '0' CHECK ("max_power3" >= 0),
    "max_power4" BIGINT NOT NULL DEFAULT '0' CHECK ("max_power4" >= 0),
    "max_power5" BIGINT NOT NULL DEFAULT '0' CHECK ("max_power5" >= 0),
    "max_power6" BIGINT NOT NULL DEFAULT '0' CHECK ("max_power6" >= 0),
    "max_power7" BIGINT NOT NULL DEFAULT '0' CHECK ("max_power7" >= 0),
    "strength" REAL NOT NULL DEFAULT '0',
    "agility" REAL NOT NULL DEFAULT '0',
    "stamina" REAL NOT NULL DEFAULT '0',
    "intellect" REAL NOT NULL DEFAULT '0',
    "spirit" REAL NOT NULL DEFAULT '0',
    "armor" INTEGER NOT NULL DEFAULT '0',
    "holy_res" INTEGER NOT NULL DEFAULT '0',
    "fire_res" INTEGER NOT NULL DEFAULT '0',
    "nature_res" INTEGER NOT NULL DEFAULT '0',
    "frost_res" INTEGER NOT NULL DEFAULT '0',
    "shadow_res" INTEGER NOT NULL DEFAULT '0',
    "arcane_res" INTEGER NOT NULL DEFAULT '0',
    "block_chance" REAL NOT NULL DEFAULT '0',
    "dodge_chance" REAL NOT NULL DEFAULT '0',
    "parry_chance" REAL NOT NULL DEFAULT '0',
    "crit_chance" REAL NOT NULL DEFAULT '0',
    "ranged_crit_chance" REAL NOT NULL DEFAULT '0',
    "spell_crit_chance" REAL NOT NULL DEFAULT '0',
    "attack_power" BIGINT NOT NULL DEFAULT '0' CHECK ("attack_power" >= 0),
    "ranged_attack_power" BIGINT NOT NULL DEFAULT '0' CHECK ("ranged_attack_power" >= 0),
    "spell_damage" BIGINT NOT NULL DEFAULT '0' CHECK ("spell_damage" >= 0),
    "spell_healing" BIGINT NOT NULL DEFAULT '0' CHECK ("spell_healing" >= 0),
    PRIMARY KEY ("guid")
);
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "max_health" BIGINT NOT NULL DEFAULT '0' CHECK ("max_health" >= 0);
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "max_power1" BIGINT NOT NULL DEFAULT '0' CHECK ("max_power1" >= 0);
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "max_power2" BIGINT NOT NULL DEFAULT '0' CHECK ("max_power2" >= 0);
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "max_power3" BIGINT NOT NULL DEFAULT '0' CHECK ("max_power3" >= 0);
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "max_power4" BIGINT NOT NULL DEFAULT '0' CHECK ("max_power4" >= 0);
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "max_power5" BIGINT NOT NULL DEFAULT '0' CHECK ("max_power5" >= 0);
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "max_power6" BIGINT NOT NULL DEFAULT '0' CHECK ("max_power6" >= 0);
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "max_power7" BIGINT NOT NULL DEFAULT '0' CHECK ("max_power7" >= 0);
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "strength" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "agility" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "stamina" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "intellect" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "spirit" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "armor" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "holy_res" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "fire_res" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "nature_res" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "frost_res" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "shadow_res" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "arcane_res" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "block_chance" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "dodge_chance" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "parry_chance" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "crit_chance" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "ranged_crit_chance" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "spell_crit_chance" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "attack_power" BIGINT NOT NULL DEFAULT '0' CHECK ("attack_power" >= 0);
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "ranged_attack_power" BIGINT NOT NULL DEFAULT '0' CHECK ("ranged_attack_power" >= 0);
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "spell_damage" BIGINT NOT NULL DEFAULT '0' CHECK ("spell_damage" >= 0);
ALTER TABLE IF EXISTS "character_stats" ADD COLUMN IF NOT EXISTS "spell_healing" BIGINT NOT NULL DEFAULT '0' CHECK ("spell_healing" >= 0);

CREATE TABLE IF NOT EXISTS "character_talent" (
    "guid" BIGINT NOT NULL CHECK ("guid" >= 0),
    "talent_id" BIGINT NOT NULL CHECK ("talent_id" >= 0),
    "current_rank" SMALLINT NOT NULL DEFAULT '0' CHECK ("current_rank" >= 0),
    PRIMARY KEY ("guid", "talent_id")
);
ALTER TABLE IF EXISTS "character_talent" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "character_talent" ADD COLUMN IF NOT EXISTS "talent_id" BIGINT NOT NULL CHECK ("talent_id" >= 0);
ALTER TABLE IF EXISTS "character_talent" ADD COLUMN IF NOT EXISTS "current_rank" SMALLINT NOT NULL DEFAULT '0' CHECK ("current_rank" >= 0);

CREATE TABLE IF NOT EXISTS "character_tutorial" (
    "account" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("account" >= 0),
    "tut0" BIGINT NOT NULL DEFAULT '0' CHECK ("tut0" >= 0),
    "tut1" BIGINT NOT NULL DEFAULT '0' CHECK ("tut1" >= 0),
    "tut2" BIGINT NOT NULL DEFAULT '0' CHECK ("tut2" >= 0),
    "tut3" BIGINT NOT NULL DEFAULT '0' CHECK ("tut3" >= 0),
    "tut4" BIGINT NOT NULL DEFAULT '0' CHECK ("tut4" >= 0),
    "tut5" BIGINT NOT NULL DEFAULT '0' CHECK ("tut5" >= 0),
    "tut6" BIGINT NOT NULL DEFAULT '0' CHECK ("tut6" >= 0),
    "tut7" BIGINT NOT NULL DEFAULT '0' CHECK ("tut7" >= 0),
    PRIMARY KEY ("account")
);
ALTER TABLE IF EXISTS "character_tutorial" ADD COLUMN IF NOT EXISTS "account" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("account" >= 0);
ALTER TABLE IF EXISTS "character_tutorial" ADD COLUMN IF NOT EXISTS "tut0" BIGINT NOT NULL DEFAULT '0' CHECK ("tut0" >= 0);
ALTER TABLE IF EXISTS "character_tutorial" ADD COLUMN IF NOT EXISTS "tut1" BIGINT NOT NULL DEFAULT '0' CHECK ("tut1" >= 0);
ALTER TABLE IF EXISTS "character_tutorial" ADD COLUMN IF NOT EXISTS "tut2" BIGINT NOT NULL DEFAULT '0' CHECK ("tut2" >= 0);
ALTER TABLE IF EXISTS "character_tutorial" ADD COLUMN IF NOT EXISTS "tut3" BIGINT NOT NULL DEFAULT '0' CHECK ("tut3" >= 0);
ALTER TABLE IF EXISTS "character_tutorial" ADD COLUMN IF NOT EXISTS "tut4" BIGINT NOT NULL DEFAULT '0' CHECK ("tut4" >= 0);
ALTER TABLE IF EXISTS "character_tutorial" ADD COLUMN IF NOT EXISTS "tut5" BIGINT NOT NULL DEFAULT '0' CHECK ("tut5" >= 0);
ALTER TABLE IF EXISTS "character_tutorial" ADD COLUMN IF NOT EXISTS "tut6" BIGINT NOT NULL DEFAULT '0' CHECK ("tut6" >= 0);
ALTER TABLE IF EXISTS "character_tutorial" ADD COLUMN IF NOT EXISTS "tut7" BIGINT NOT NULL DEFAULT '0' CHECK ("tut7" >= 0);

CREATE TABLE IF NOT EXISTS "characters" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "account" BIGINT NOT NULL DEFAULT '0' CHECK ("account" >= 0),
    "name" VARCHAR(12) NOT NULL DEFAULT '',
    "race" SMALLINT NOT NULL DEFAULT '0' CHECK ("race" >= 0),
    "class" SMALLINT NOT NULL DEFAULT '0' CHECK ("class" >= 0),
    "gender" SMALLINT NOT NULL DEFAULT '0' CHECK ("gender" >= 0),
    "skin" SMALLINT NOT NULL DEFAULT '0' CHECK ("skin" >= 0),
    "face" SMALLINT NOT NULL DEFAULT '0' CHECK ("face" >= 0),
    "hair_style" SMALLINT NOT NULL DEFAULT '0' CHECK ("hair_style" >= 0),
    "hair_color" SMALLINT NOT NULL DEFAULT '0' CHECK ("hair_color" >= 0),
    "facial_hair" SMALLINT NOT NULL DEFAULT '0' CHECK ("facial_hair" >= 0),
    "level" SMALLINT NOT NULL DEFAULT '0' CHECK ("level" >= 0),
    "xp" BIGINT NOT NULL DEFAULT '0' CHECK ("xp" >= 0),
    "money" BIGINT NOT NULL DEFAULT '0' CHECK ("money" >= 0),
    "character_flags" BIGINT NOT NULL DEFAULT '0' CHECK ("character_flags" >= 0),
    "zone" BIGINT NOT NULL DEFAULT '0' CHECK ("zone" >= 0),
    "map" BIGINT NOT NULL DEFAULT '0' CHECK ("map" >= 0),
    "instance" BIGINT NOT NULL DEFAULT '0' CHECK ("instance" >= 0),
    "position_x" REAL NOT NULL DEFAULT '0',
    "position_y" REAL NOT NULL DEFAULT '0',
    "position_z" REAL NOT NULL DEFAULT '0',
    "orientation" REAL NOT NULL DEFAULT '0',
    "transport_guid" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("transport_guid" >= 0),
    "transport_x" REAL NOT NULL DEFAULT '0',
    "transport_y" REAL NOT NULL DEFAULT '0',
    "transport_z" REAL NOT NULL DEFAULT '0',
    "transport_o" REAL NOT NULL DEFAULT '0',
    "known_taxi_mask" TEXT,
    "current_taxi_path" TEXT,
    "online" SMALLINT NOT NULL DEFAULT '0' CHECK ("online" >= 0),
    "played_time_total" BIGINT NOT NULL DEFAULT '0' CHECK ("played_time_total" >= 0),
    "played_time_level" BIGINT NOT NULL DEFAULT '0' CHECK ("played_time_level" >= 0),
    "create_time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("create_time" >= 0),
    "logout_time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("logout_time" >= 0),
    "rest_bonus" REAL NOT NULL DEFAULT '0',
    "reset_talents_multiplier" BIGINT NOT NULL DEFAULT '0' CHECK ("reset_talents_multiplier" >= 0),
    "reset_talents_time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("reset_talents_time" >= 0),
    "death_expire_time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("death_expire_time" >= 0),
    "stable_slots" SMALLINT NOT NULL DEFAULT '0' CHECK ("stable_slots" >= 0),
    "bank_bag_slots" SMALLINT NOT NULL DEFAULT '0' CHECK ("bank_bag_slots" >= 0),
    "extra_flags" BIGINT NOT NULL DEFAULT '0' CHECK ("extra_flags" >= 0),
    "honor_rank_points" REAL NOT NULL DEFAULT '0',
    "honor_highest_rank" BIGINT NOT NULL DEFAULT '0' CHECK ("honor_highest_rank" >= 0),
    "honor_standing" BIGINT NOT NULL DEFAULT '0' CHECK ("honor_standing" >= 0),
    "honor_last_week_hk" BIGINT NOT NULL DEFAULT '0' CHECK ("honor_last_week_hk" >= 0),
    "honor_last_week_cp" REAL NOT NULL DEFAULT '0',
    "honor_stored_hk" INTEGER NOT NULL DEFAULT '0',
    "honor_stored_dk" INTEGER NOT NULL DEFAULT '0',
    "watched_faction" INTEGER NOT NULL DEFAULT '-1',
    "drunk" INTEGER NOT NULL DEFAULT '0' CHECK ("drunk" >= 0),
    "health" BIGINT NOT NULL DEFAULT '0' CHECK ("health" >= 0),
    "power1" BIGINT NOT NULL DEFAULT '0' CHECK ("power1" >= 0),
    "power2" BIGINT NOT NULL DEFAULT '0' CHECK ("power2" >= 0),
    "power3" BIGINT NOT NULL DEFAULT '0' CHECK ("power3" >= 0),
    "power4" BIGINT NOT NULL DEFAULT '0' CHECK ("power4" >= 0),
    "power5" BIGINT NOT NULL DEFAULT '0' CHECK ("power5" >= 0),
    "explored_zones" TEXT,
    "equipment_cache" TEXT,
    "ammo_id" BIGINT NOT NULL DEFAULT '0' CHECK ("ammo_id" >= 0),
    "action_bars" SMALLINT NOT NULL DEFAULT '0' CHECK ("action_bars" >= 0),
    "deleted_account" BIGINT DEFAULT NULL CHECK ("deleted_account" >= 0),
    "deleted_name" VARCHAR(12) DEFAULT NULL,
    "deleted_time" BIGINT DEFAULT NULL,
    "world_phase_mask" INTEGER DEFAULT '0',
    PRIMARY KEY ("guid")
);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "account" BIGINT NOT NULL DEFAULT '0' CHECK ("account" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "name" VARCHAR(12) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "race" SMALLINT NOT NULL DEFAULT '0' CHECK ("race" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "class" SMALLINT NOT NULL DEFAULT '0' CHECK ("class" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "gender" SMALLINT NOT NULL DEFAULT '0' CHECK ("gender" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "skin" SMALLINT NOT NULL DEFAULT '0' CHECK ("skin" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "face" SMALLINT NOT NULL DEFAULT '0' CHECK ("face" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "hair_style" SMALLINT NOT NULL DEFAULT '0' CHECK ("hair_style" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "hair_color" SMALLINT NOT NULL DEFAULT '0' CHECK ("hair_color" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "facial_hair" SMALLINT NOT NULL DEFAULT '0' CHECK ("facial_hair" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "level" SMALLINT NOT NULL DEFAULT '0' CHECK ("level" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "xp" BIGINT NOT NULL DEFAULT '0' CHECK ("xp" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "money" BIGINT NOT NULL DEFAULT '0' CHECK ("money" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "character_flags" BIGINT NOT NULL DEFAULT '0' CHECK ("character_flags" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "zone" BIGINT NOT NULL DEFAULT '0' CHECK ("zone" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "map" BIGINT NOT NULL DEFAULT '0' CHECK ("map" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "instance" BIGINT NOT NULL DEFAULT '0' CHECK ("instance" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "position_x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "position_y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "position_z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "orientation" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "transport_guid" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("transport_guid" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "transport_x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "transport_y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "transport_z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "transport_o" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "known_taxi_mask" TEXT;
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "current_taxi_path" TEXT;
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "online" SMALLINT NOT NULL DEFAULT '0' CHECK ("online" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "played_time_total" BIGINT NOT NULL DEFAULT '0' CHECK ("played_time_total" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "played_time_level" BIGINT NOT NULL DEFAULT '0' CHECK ("played_time_level" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "create_time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("create_time" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "logout_time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("logout_time" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "rest_bonus" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "reset_talents_multiplier" BIGINT NOT NULL DEFAULT '0' CHECK ("reset_talents_multiplier" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "reset_talents_time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("reset_talents_time" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "death_expire_time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("death_expire_time" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "stable_slots" SMALLINT NOT NULL DEFAULT '0' CHECK ("stable_slots" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "bank_bag_slots" SMALLINT NOT NULL DEFAULT '0' CHECK ("bank_bag_slots" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "extra_flags" BIGINT NOT NULL DEFAULT '0' CHECK ("extra_flags" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "honor_rank_points" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "honor_highest_rank" BIGINT NOT NULL DEFAULT '0' CHECK ("honor_highest_rank" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "honor_standing" BIGINT NOT NULL DEFAULT '0' CHECK ("honor_standing" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "honor_last_week_hk" BIGINT NOT NULL DEFAULT '0' CHECK ("honor_last_week_hk" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "honor_last_week_cp" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "honor_stored_hk" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "honor_stored_dk" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "watched_faction" INTEGER NOT NULL DEFAULT '-1';
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "drunk" INTEGER NOT NULL DEFAULT '0' CHECK ("drunk" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "health" BIGINT NOT NULL DEFAULT '0' CHECK ("health" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "power1" BIGINT NOT NULL DEFAULT '0' CHECK ("power1" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "power2" BIGINT NOT NULL DEFAULT '0' CHECK ("power2" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "power3" BIGINT NOT NULL DEFAULT '0' CHECK ("power3" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "power4" BIGINT NOT NULL DEFAULT '0' CHECK ("power4" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "power5" BIGINT NOT NULL DEFAULT '0' CHECK ("power5" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "explored_zones" TEXT;
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "equipment_cache" TEXT;
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "ammo_id" BIGINT NOT NULL DEFAULT '0' CHECK ("ammo_id" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "action_bars" SMALLINT NOT NULL DEFAULT '0' CHECK ("action_bars" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "deleted_account" BIGINT DEFAULT NULL CHECK ("deleted_account" >= 0);
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "deleted_name" VARCHAR(12) DEFAULT NULL;
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "deleted_time" BIGINT DEFAULT NULL;
ALTER TABLE IF EXISTS "characters" ADD COLUMN IF NOT EXISTS "world_phase_mask" INTEGER DEFAULT '0';
CREATE INDEX IF NOT EXISTS idx_characters_idx_account ON "characters" ("account");
CREATE INDEX IF NOT EXISTS idx_characters_idx_online ON "characters" ("online");
CREATE INDEX IF NOT EXISTS idx_characters_idx_name ON "characters" ("name");
CREATE INDEX IF NOT EXISTS idx_characters_idx_instance ON "characters" ("instance");

CREATE TABLE IF NOT EXISTS "characters_guid_delete" (
    "guid" INTEGER DEFAULT NULL,
    UNIQUE ("guid")
);
ALTER TABLE IF EXISTS "characters_guid_delete" ADD COLUMN IF NOT EXISTS "guid" INTEGER DEFAULT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_characters_guid_delete_key_guid ON "characters_guid_delete" ("guid");

CREATE TABLE IF NOT EXISTS "characters_item_delete" (
    "entry" INTEGER DEFAULT NULL,
    UNIQUE ("entry")
);
ALTER TABLE IF EXISTS "characters_item_delete" ADD COLUMN IF NOT EXISTS "entry" INTEGER DEFAULT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_characters_item_delete_key_entry ON "characters_item_delete" ("entry");

CREATE TABLE IF NOT EXISTS "corpse" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "player_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("player_guid" >= 0),
    "position_x" REAL NOT NULL DEFAULT '0',
    "position_y" REAL NOT NULL DEFAULT '0',
    "position_z" REAL NOT NULL DEFAULT '0',
    "orientation" REAL NOT NULL DEFAULT '0',
    "map" BIGINT NOT NULL DEFAULT '0' CHECK ("map" >= 0),
    "time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("time" >= 0),
    "corpse_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("corpse_type" >= 0),
    "instance" BIGINT NOT NULL DEFAULT '0' CHECK ("instance" >= 0),
    PRIMARY KEY ("guid")
);
ALTER TABLE IF EXISTS "corpse" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "corpse" ADD COLUMN IF NOT EXISTS "player_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("player_guid" >= 0);
ALTER TABLE IF EXISTS "corpse" ADD COLUMN IF NOT EXISTS "position_x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "corpse" ADD COLUMN IF NOT EXISTS "position_y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "corpse" ADD COLUMN IF NOT EXISTS "position_z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "corpse" ADD COLUMN IF NOT EXISTS "orientation" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "corpse" ADD COLUMN IF NOT EXISTS "map" BIGINT NOT NULL DEFAULT '0' CHECK ("map" >= 0);
ALTER TABLE IF EXISTS "corpse" ADD COLUMN IF NOT EXISTS "time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("time" >= 0);
ALTER TABLE IF EXISTS "corpse" ADD COLUMN IF NOT EXISTS "corpse_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("corpse_type" >= 0);
ALTER TABLE IF EXISTS "corpse" ADD COLUMN IF NOT EXISTS "instance" BIGINT NOT NULL DEFAULT '0' CHECK ("instance" >= 0);
CREATE INDEX IF NOT EXISTS idx_corpse_idx_type ON "corpse" ("corpse_type");
CREATE INDEX IF NOT EXISTS idx_corpse_idx_instance ON "corpse" ("instance");
CREATE INDEX IF NOT EXISTS idx_corpse_idx_player ON "corpse" ("player_guid");
CREATE INDEX IF NOT EXISTS idx_corpse_idx_time ON "corpse" ("time");

CREATE TABLE IF NOT EXISTS "creature_respawn" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "respawn_time" BIGINT NOT NULL DEFAULT '0',
    "instance" BIGINT NOT NULL DEFAULT '0' CHECK ("instance" >= 0),
    "map" BIGINT DEFAULT '0' CHECK ("map" >= 0),
    PRIMARY KEY ("guid", "instance")
);
ALTER TABLE IF EXISTS "creature_respawn" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "creature_respawn" ADD COLUMN IF NOT EXISTS "respawn_time" BIGINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "creature_respawn" ADD COLUMN IF NOT EXISTS "instance" BIGINT NOT NULL DEFAULT '0' CHECK ("instance" >= 0);
ALTER TABLE IF EXISTS "creature_respawn" ADD COLUMN IF NOT EXISTS "map" BIGINT DEFAULT '0' CHECK ("map" >= 0);
CREATE INDEX IF NOT EXISTS idx_creature_respawn_idx_instance ON "creature_respawn" ("instance");

CREATE TABLE IF NOT EXISTS "game_event_status" (
    "event" INTEGER NOT NULL DEFAULT '0' CHECK ("event" >= 0),
    PRIMARY KEY ("event")
);
ALTER TABLE IF EXISTS "game_event_status" ADD COLUMN IF NOT EXISTS "event" INTEGER NOT NULL DEFAULT '0' CHECK ("event" >= 0);

CREATE TABLE IF NOT EXISTS "gameobject_respawn" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "respawn_time" BIGINT NOT NULL DEFAULT '0',
    "instance" BIGINT NOT NULL DEFAULT '0' CHECK ("instance" >= 0),
    "map" BIGINT DEFAULT '0' CHECK ("map" >= 0),
    PRIMARY KEY ("guid", "instance")
);
ALTER TABLE IF EXISTS "gameobject_respawn" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "gameobject_respawn" ADD COLUMN IF NOT EXISTS "respawn_time" BIGINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gameobject_respawn" ADD COLUMN IF NOT EXISTS "instance" BIGINT NOT NULL DEFAULT '0' CHECK ("instance" >= 0);
ALTER TABLE IF EXISTS "gameobject_respawn" ADD COLUMN IF NOT EXISTS "map" BIGINT DEFAULT '0' CHECK ("map" >= 0);
CREATE INDEX IF NOT EXISTS idx_gameobject_respawn_idx_instance ON "gameobject_respawn" ("instance");

CREATE TABLE IF NOT EXISTS "gm_subsurveys" (
    "survey_id" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("survey_id" >= 0),
    "subsurvey_id" BIGINT NOT NULL DEFAULT '0' CHECK ("subsurvey_id" >= 0),
    "rank" BIGINT NOT NULL DEFAULT '0' CHECK ("rank" >= 0),
    "comment" TEXT NOT NULL,
    PRIMARY KEY ("survey_id", "subsurvey_id")
);
ALTER TABLE IF EXISTS "gm_subsurveys" ADD COLUMN IF NOT EXISTS "survey_id" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("survey_id" >= 0);
ALTER TABLE IF EXISTS "gm_subsurveys" ADD COLUMN IF NOT EXISTS "subsurvey_id" BIGINT NOT NULL DEFAULT '0' CHECK ("subsurvey_id" >= 0);
ALTER TABLE IF EXISTS "gm_subsurveys" ADD COLUMN IF NOT EXISTS "rank" BIGINT NOT NULL DEFAULT '0' CHECK ("rank" >= 0);
ALTER TABLE IF EXISTS "gm_subsurveys" ADD COLUMN IF NOT EXISTS "comment" TEXT NOT NULL;

CREATE TABLE IF NOT EXISTS "gm_surveys" (
    "survey_id" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("survey_id" >= 0),
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "main_survey" BIGINT NOT NULL DEFAULT '0' CHECK ("main_survey" >= 0),
    "overall_comment" TEXT NOT NULL,
    "create_time" BIGINT NOT NULL DEFAULT '0' CHECK ("create_time" >= 0),
    PRIMARY KEY ("survey_id")
);
ALTER TABLE IF EXISTS "gm_surveys" ADD COLUMN IF NOT EXISTS "survey_id" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("survey_id" >= 0);
ALTER TABLE IF EXISTS "gm_surveys" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "gm_surveys" ADD COLUMN IF NOT EXISTS "main_survey" BIGINT NOT NULL DEFAULT '0' CHECK ("main_survey" >= 0);
ALTER TABLE IF EXISTS "gm_surveys" ADD COLUMN IF NOT EXISTS "overall_comment" TEXT NOT NULL;
ALTER TABLE IF EXISTS "gm_surveys" ADD COLUMN IF NOT EXISTS "create_time" BIGINT NOT NULL DEFAULT '0' CHECK ("create_time" >= 0);

CREATE TABLE IF NOT EXISTS "gm_tickets" (
    "ticket_id" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("ticket_id" >= 0),
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "name" VARCHAR(12) NOT NULL,
    "message" TEXT NOT NULL,
    "create_time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("create_time" >= 0),
    "map" INTEGER NOT NULL DEFAULT '0' CHECK ("map" >= 0),
    "position_x" REAL NOT NULL DEFAULT '0',
    "position_y" REAL NOT NULL DEFAULT '0',
    "position_z" REAL NOT NULL DEFAULT '0',
    "last_modified_time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("last_modified_time" >= 0),
    "closed_by" INTEGER NOT NULL DEFAULT '0',
    "assigned_to" BIGINT NOT NULL DEFAULT '0' CHECK ("assigned_to" >= 0),
    "comment" TEXT NOT NULL,
    "response" TEXT NOT NULL,
    "completed" SMALLINT NOT NULL DEFAULT '0' CHECK ("completed" >= 0),
    "escalated" SMALLINT NOT NULL DEFAULT '0' CHECK ("escalated" >= 0),
    "viewed" SMALLINT NOT NULL DEFAULT '0' CHECK ("viewed" >= 0),
    "have_ticket" SMALLINT NOT NULL DEFAULT '0' CHECK ("have_ticket" >= 0),
    "ticket_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("ticket_type" >= 0),
    "security_needed" SMALLINT NOT NULL DEFAULT '0' CHECK ("security_needed" >= 0),
    PRIMARY KEY ("ticket_id")
);
ALTER TABLE IF EXISTS "gm_tickets" ADD COLUMN IF NOT EXISTS "ticket_id" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("ticket_id" >= 0);
ALTER TABLE IF EXISTS "gm_tickets" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "gm_tickets" ADD COLUMN IF NOT EXISTS "name" VARCHAR(12) NOT NULL;
ALTER TABLE IF EXISTS "gm_tickets" ADD COLUMN IF NOT EXISTS "message" TEXT NOT NULL;
ALTER TABLE IF EXISTS "gm_tickets" ADD COLUMN IF NOT EXISTS "create_time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("create_time" >= 0);
ALTER TABLE IF EXISTS "gm_tickets" ADD COLUMN IF NOT EXISTS "map" INTEGER NOT NULL DEFAULT '0' CHECK ("map" >= 0);
ALTER TABLE IF EXISTS "gm_tickets" ADD COLUMN IF NOT EXISTS "position_x" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gm_tickets" ADD COLUMN IF NOT EXISTS "position_y" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gm_tickets" ADD COLUMN IF NOT EXISTS "position_z" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gm_tickets" ADD COLUMN IF NOT EXISTS "last_modified_time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("last_modified_time" >= 0);
ALTER TABLE IF EXISTS "gm_tickets" ADD COLUMN IF NOT EXISTS "closed_by" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "gm_tickets" ADD COLUMN IF NOT EXISTS "assigned_to" BIGINT NOT NULL DEFAULT '0' CHECK ("assigned_to" >= 0);
ALTER TABLE IF EXISTS "gm_tickets" ADD COLUMN IF NOT EXISTS "comment" TEXT NOT NULL;
ALTER TABLE IF EXISTS "gm_tickets" ADD COLUMN IF NOT EXISTS "response" TEXT NOT NULL;
ALTER TABLE IF EXISTS "gm_tickets" ADD COLUMN IF NOT EXISTS "completed" SMALLINT NOT NULL DEFAULT '0' CHECK ("completed" >= 0);
ALTER TABLE IF EXISTS "gm_tickets" ADD COLUMN IF NOT EXISTS "escalated" SMALLINT NOT NULL DEFAULT '0' CHECK ("escalated" >= 0);
ALTER TABLE IF EXISTS "gm_tickets" ADD COLUMN IF NOT EXISTS "viewed" SMALLINT NOT NULL DEFAULT '0' CHECK ("viewed" >= 0);
ALTER TABLE IF EXISTS "gm_tickets" ADD COLUMN IF NOT EXISTS "have_ticket" SMALLINT NOT NULL DEFAULT '0' CHECK ("have_ticket" >= 0);
ALTER TABLE IF EXISTS "gm_tickets" ADD COLUMN IF NOT EXISTS "ticket_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("ticket_type" >= 0);
ALTER TABLE IF EXISTS "gm_tickets" ADD COLUMN IF NOT EXISTS "security_needed" SMALLINT NOT NULL DEFAULT '0' CHECK ("security_needed" >= 0);

CREATE TABLE IF NOT EXISTS "group_instance" (
    "leader_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("leader_guid" >= 0),
    "instance" BIGINT NOT NULL DEFAULT '0' CHECK ("instance" >= 0),
    "permanent" SMALLINT NOT NULL DEFAULT '0' CHECK ("permanent" >= 0),
    PRIMARY KEY ("leader_guid", "instance")
);
ALTER TABLE IF EXISTS "group_instance" ADD COLUMN IF NOT EXISTS "leader_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("leader_guid" >= 0);
ALTER TABLE IF EXISTS "group_instance" ADD COLUMN IF NOT EXISTS "instance" BIGINT NOT NULL DEFAULT '0' CHECK ("instance" >= 0);
ALTER TABLE IF EXISTS "group_instance" ADD COLUMN IF NOT EXISTS "permanent" SMALLINT NOT NULL DEFAULT '0' CHECK ("permanent" >= 0);
CREATE INDEX IF NOT EXISTS idx_group_instance_idx_instance ON "group_instance" ("instance");

CREATE TABLE IF NOT EXISTS "group_member" (
    "group_id" BIGINT NOT NULL CHECK ("group_id" >= 0),
    "member_guid" BIGINT NOT NULL CHECK ("member_guid" >= 0),
    "assistant" SMALLINT NOT NULL CHECK ("assistant" >= 0),
    "subgroup" INTEGER NOT NULL CHECK ("subgroup" >= 0),
    PRIMARY KEY ("group_id", "member_guid")
);
ALTER TABLE IF EXISTS "group_member" ADD COLUMN IF NOT EXISTS "group_id" BIGINT NOT NULL CHECK ("group_id" >= 0);
ALTER TABLE IF EXISTS "group_member" ADD COLUMN IF NOT EXISTS "member_guid" BIGINT NOT NULL CHECK ("member_guid" >= 0);
ALTER TABLE IF EXISTS "group_member" ADD COLUMN IF NOT EXISTS "assistant" SMALLINT NOT NULL CHECK ("assistant" >= 0);
ALTER TABLE IF EXISTS "group_member" ADD COLUMN IF NOT EXISTS "subgroup" INTEGER NOT NULL CHECK ("subgroup" >= 0);
CREATE INDEX IF NOT EXISTS idx_group_member_idx_memberguid ON "group_member" ("member_guid");

CREATE TABLE IF NOT EXISTS "groups" (
    "group_id" BIGINT NOT NULL CHECK ("group_id" >= 0),
    "leader_guid" BIGINT NOT NULL CHECK ("leader_guid" >= 0),
    "main_tank_guid" BIGINT NOT NULL CHECK ("main_tank_guid" >= 0),
    "main_assistant_guid" BIGINT NOT NULL CHECK ("main_assistant_guid" >= 0),
    "loot_method" SMALLINT NOT NULL CHECK ("loot_method" >= 0),
    "loot_threshold" SMALLINT NOT NULL CHECK ("loot_threshold" >= 0),
    "looter_guid" BIGINT NOT NULL CHECK ("looter_guid" >= 0),
    "icon1" BIGINT NOT NULL CHECK ("icon1" >= 0),
    "icon2" BIGINT NOT NULL CHECK ("icon2" >= 0),
    "icon3" BIGINT NOT NULL CHECK ("icon3" >= 0),
    "icon4" BIGINT NOT NULL CHECK ("icon4" >= 0),
    "icon5" BIGINT NOT NULL CHECK ("icon5" >= 0),
    "icon6" BIGINT NOT NULL CHECK ("icon6" >= 0),
    "icon7" BIGINT NOT NULL CHECK ("icon7" >= 0),
    "icon8" BIGINT NOT NULL CHECK ("icon8" >= 0),
    "is_raid" SMALLINT NOT NULL CHECK ("is_raid" >= 0),
    PRIMARY KEY ("group_id"),
    UNIQUE ("leader_guid")
);
ALTER TABLE IF EXISTS "groups" ADD COLUMN IF NOT EXISTS "group_id" BIGINT NOT NULL CHECK ("group_id" >= 0);
ALTER TABLE IF EXISTS "groups" ADD COLUMN IF NOT EXISTS "leader_guid" BIGINT NOT NULL CHECK ("leader_guid" >= 0);
ALTER TABLE IF EXISTS "groups" ADD COLUMN IF NOT EXISTS "main_tank_guid" BIGINT NOT NULL CHECK ("main_tank_guid" >= 0);
ALTER TABLE IF EXISTS "groups" ADD COLUMN IF NOT EXISTS "main_assistant_guid" BIGINT NOT NULL CHECK ("main_assistant_guid" >= 0);
ALTER TABLE IF EXISTS "groups" ADD COLUMN IF NOT EXISTS "loot_method" SMALLINT NOT NULL CHECK ("loot_method" >= 0);
ALTER TABLE IF EXISTS "groups" ADD COLUMN IF NOT EXISTS "loot_threshold" SMALLINT NOT NULL CHECK ("loot_threshold" >= 0);
ALTER TABLE IF EXISTS "groups" ADD COLUMN IF NOT EXISTS "looter_guid" BIGINT NOT NULL CHECK ("looter_guid" >= 0);
ALTER TABLE IF EXISTS "groups" ADD COLUMN IF NOT EXISTS "icon1" BIGINT NOT NULL CHECK ("icon1" >= 0);
ALTER TABLE IF EXISTS "groups" ADD COLUMN IF NOT EXISTS "icon2" BIGINT NOT NULL CHECK ("icon2" >= 0);
ALTER TABLE IF EXISTS "groups" ADD COLUMN IF NOT EXISTS "icon3" BIGINT NOT NULL CHECK ("icon3" >= 0);
ALTER TABLE IF EXISTS "groups" ADD COLUMN IF NOT EXISTS "icon4" BIGINT NOT NULL CHECK ("icon4" >= 0);
ALTER TABLE IF EXISTS "groups" ADD COLUMN IF NOT EXISTS "icon5" BIGINT NOT NULL CHECK ("icon5" >= 0);
ALTER TABLE IF EXISTS "groups" ADD COLUMN IF NOT EXISTS "icon6" BIGINT NOT NULL CHECK ("icon6" >= 0);
ALTER TABLE IF EXISTS "groups" ADD COLUMN IF NOT EXISTS "icon7" BIGINT NOT NULL CHECK ("icon7" >= 0);
ALTER TABLE IF EXISTS "groups" ADD COLUMN IF NOT EXISTS "icon8" BIGINT NOT NULL CHECK ("icon8" >= 0);
ALTER TABLE IF EXISTS "groups" ADD COLUMN IF NOT EXISTS "is_raid" SMALLINT NOT NULL CHECK ("is_raid" >= 0);
CREATE UNIQUE INDEX IF NOT EXISTS idx_groups_key_leaderguid ON "groups" ("leader_guid");

CREATE TABLE IF NOT EXISTS "guild" (
    "guild_id" BIGINT NOT NULL DEFAULT '0' CHECK ("guild_id" >= 0),
    "name" VARCHAR(255) NOT NULL DEFAULT '',
    "leader_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("leader_guid" >= 0),
    "emblem_style" INTEGER NOT NULL DEFAULT '0',
    "emblem_color" INTEGER NOT NULL DEFAULT '0',
    "border_style" INTEGER NOT NULL DEFAULT '0',
    "border_color" INTEGER NOT NULL DEFAULT '0',
    "background_color" INTEGER NOT NULL DEFAULT '0',
    "info" TEXT NOT NULL,
    "motd" VARCHAR(255) NOT NULL DEFAULT '',
    "create_date" BIGINT NOT NULL DEFAULT '0',
    "bank_money" BIGINT NOT NULL DEFAULT '0' CHECK ("bank_money" >= 0),
    PRIMARY KEY ("guild_id")
);
ALTER TABLE IF EXISTS "guild" ADD COLUMN IF NOT EXISTS "guild_id" BIGINT NOT NULL DEFAULT '0' CHECK ("guild_id" >= 0);
ALTER TABLE IF EXISTS "guild" ADD COLUMN IF NOT EXISTS "name" VARCHAR(255) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "guild" ADD COLUMN IF NOT EXISTS "leader_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("leader_guid" >= 0);
ALTER TABLE IF EXISTS "guild" ADD COLUMN IF NOT EXISTS "emblem_style" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "guild" ADD COLUMN IF NOT EXISTS "emblem_color" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "guild" ADD COLUMN IF NOT EXISTS "border_style" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "guild" ADD COLUMN IF NOT EXISTS "border_color" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "guild" ADD COLUMN IF NOT EXISTS "background_color" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "guild" ADD COLUMN IF NOT EXISTS "info" TEXT NOT NULL;
ALTER TABLE IF EXISTS "guild" ADD COLUMN IF NOT EXISTS "motd" VARCHAR(255) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "guild" ADD COLUMN IF NOT EXISTS "create_date" BIGINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "guild" ADD COLUMN IF NOT EXISTS "bank_money" BIGINT NOT NULL DEFAULT '0' CHECK ("bank_money" >= 0);

CREATE TABLE IF NOT EXISTS "guild_bank_tab" (
    "guild_id" BIGINT NOT NULL DEFAULT '0' CHECK ("guild_id" >= 0),
    "tab_id" SMALLINT NOT NULL DEFAULT '0' CHECK ("tab_id" >= 0),
    "name" VARCHAR(255) NOT NULL DEFAULT '',
    "icon" VARCHAR(255) NOT NULL DEFAULT '',
    "view_rank" SMALLINT NOT NULL DEFAULT '0' CHECK ("view_rank" >= 0),
    "withdraw_rank" SMALLINT NOT NULL DEFAULT '0' CHECK ("withdraw_rank" >= 0),
    "deposit_rank" SMALLINT NOT NULL DEFAULT '0' CHECK ("deposit_rank" >= 0),
    PRIMARY KEY ("guild_id", "tab_id")
);
ALTER TABLE IF EXISTS "guild_bank_tab" ADD COLUMN IF NOT EXISTS "guild_id" BIGINT NOT NULL DEFAULT '0' CHECK ("guild_id" >= 0);
ALTER TABLE IF EXISTS "guild_bank_tab" ADD COLUMN IF NOT EXISTS "tab_id" SMALLINT NOT NULL DEFAULT '0' CHECK ("tab_id" >= 0);
ALTER TABLE IF EXISTS "guild_bank_tab" ADD COLUMN IF NOT EXISTS "name" VARCHAR(255) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "guild_bank_tab" ADD COLUMN IF NOT EXISTS "icon" VARCHAR(255) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "guild_bank_tab" ADD COLUMN IF NOT EXISTS "view_rank" SMALLINT NOT NULL DEFAULT '0' CHECK ("view_rank" >= 0);
ALTER TABLE IF EXISTS "guild_bank_tab" ADD COLUMN IF NOT EXISTS "withdraw_rank" SMALLINT NOT NULL DEFAULT '0' CHECK ("withdraw_rank" >= 0);
ALTER TABLE IF EXISTS "guild_bank_tab" ADD COLUMN IF NOT EXISTS "deposit_rank" SMALLINT NOT NULL DEFAULT '0' CHECK ("deposit_rank" >= 0);
CREATE INDEX IF NOT EXISTS idx_guild_bank_tab_idx_guildid ON "guild_bank_tab" ("guild_id");

CREATE TABLE IF NOT EXISTS "guild_eventlog" (
    "guild_id" INTEGER NOT NULL,
    "log_guid" INTEGER NOT NULL,
    "event_type" SMALLINT NOT NULL,
    "player_guid1" INTEGER NOT NULL,
    "player_guid2" INTEGER NOT NULL,
    "new_rank" SMALLINT NOT NULL,
    "timestamp" BIGINT NOT NULL,
    PRIMARY KEY ("guild_id", "log_guid")
);
ALTER TABLE IF EXISTS "guild_eventlog" ADD COLUMN IF NOT EXISTS "guild_id" INTEGER NOT NULL;
ALTER TABLE IF EXISTS "guild_eventlog" ADD COLUMN IF NOT EXISTS "log_guid" INTEGER NOT NULL;
ALTER TABLE IF EXISTS "guild_eventlog" ADD COLUMN IF NOT EXISTS "event_type" SMALLINT NOT NULL;
ALTER TABLE IF EXISTS "guild_eventlog" ADD COLUMN IF NOT EXISTS "player_guid1" INTEGER NOT NULL;
ALTER TABLE IF EXISTS "guild_eventlog" ADD COLUMN IF NOT EXISTS "player_guid2" INTEGER NOT NULL;
ALTER TABLE IF EXISTS "guild_eventlog" ADD COLUMN IF NOT EXISTS "new_rank" SMALLINT NOT NULL;
ALTER TABLE IF EXISTS "guild_eventlog" ADD COLUMN IF NOT EXISTS "timestamp" BIGINT NOT NULL;
CREATE INDEX IF NOT EXISTS idx_guild_eventlog_idx_playerguid1 ON "guild_eventlog" ("player_guid1");
CREATE INDEX IF NOT EXISTS idx_guild_eventlog_idx_playerguid2 ON "guild_eventlog" ("player_guid2");
CREATE INDEX IF NOT EXISTS idx_guild_eventlog_idx_logguid ON "guild_eventlog" ("log_guid");

CREATE TABLE IF NOT EXISTS "guild_member" (
    "guild_id" BIGINT NOT NULL DEFAULT '0' CHECK ("guild_id" >= 0),
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "rank" SMALLINT NOT NULL DEFAULT '0' CHECK ("rank" >= 0),
    "player_note" VARCHAR(255) NOT NULL DEFAULT '',
    "officer_note" VARCHAR(255) NOT NULL DEFAULT '',
    UNIQUE ("guid")
);
ALTER TABLE IF EXISTS "guild_member" ADD COLUMN IF NOT EXISTS "guild_id" BIGINT NOT NULL DEFAULT '0' CHECK ("guild_id" >= 0);
ALTER TABLE IF EXISTS "guild_member" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "guild_member" ADD COLUMN IF NOT EXISTS "rank" SMALLINT NOT NULL DEFAULT '0' CHECK ("rank" >= 0);
ALTER TABLE IF EXISTS "guild_member" ADD COLUMN IF NOT EXISTS "player_note" VARCHAR(255) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "guild_member" ADD COLUMN IF NOT EXISTS "officer_note" VARCHAR(255) NOT NULL DEFAULT '';
CREATE UNIQUE INDEX IF NOT EXISTS idx_guild_member_key_guid ON "guild_member" ("guid");
CREATE INDEX IF NOT EXISTS idx_guild_member_idx_guildid ON "guild_member" ("guild_id");
CREATE INDEX IF NOT EXISTS idx_guild_member_idx_guildid_rank ON "guild_member" ("guild_id", "rank");

CREATE TABLE IF NOT EXISTS "guild_rank" (
    "guild_id" BIGINT NOT NULL DEFAULT '0' CHECK ("guild_id" >= 0),
    "id" BIGINT NOT NULL CHECK ("id" >= 0),
    "name" VARCHAR(255) NOT NULL DEFAULT '',
    "rights" BIGINT NOT NULL DEFAULT '0' CHECK ("rights" >= 0),
    PRIMARY KEY ("guild_id", "id")
);
ALTER TABLE IF EXISTS "guild_rank" ADD COLUMN IF NOT EXISTS "guild_id" BIGINT NOT NULL DEFAULT '0' CHECK ("guild_id" >= 0);
ALTER TABLE IF EXISTS "guild_rank" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "guild_rank" ADD COLUMN IF NOT EXISTS "name" VARCHAR(255) NOT NULL DEFAULT '';
ALTER TABLE IF EXISTS "guild_rank" ADD COLUMN IF NOT EXISTS "rights" BIGINT NOT NULL DEFAULT '0' CHECK ("rights" >= 0);
CREATE INDEX IF NOT EXISTS idx_guild_rank_idx_rid ON "guild_rank" ("id");

CREATE TABLE IF NOT EXISTS "instance" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "map" BIGINT NOT NULL DEFAULT '0' CHECK ("map" >= 0),
    "reset_time" BIGINT NOT NULL DEFAULT '0',
    "data" TEXT,
    PRIMARY KEY ("id")
);
ALTER TABLE IF EXISTS "instance" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "instance" ADD COLUMN IF NOT EXISTS "map" BIGINT NOT NULL DEFAULT '0' CHECK ("map" >= 0);
ALTER TABLE IF EXISTS "instance" ADD COLUMN IF NOT EXISTS "reset_time" BIGINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "instance" ADD COLUMN IF NOT EXISTS "data" TEXT;
CREATE INDEX IF NOT EXISTS idx_instance_idx_map ON "instance" ("map");
CREATE INDEX IF NOT EXISTS idx_instance_idx_resettime ON "instance" ("reset_time");

CREATE TABLE IF NOT EXISTS "instance_reset" (
    "map" BIGINT NOT NULL DEFAULT '0' CHECK ("map" >= 0),
    "reset_time" BIGINT NOT NULL DEFAULT '0',
    PRIMARY KEY ("map")
);
ALTER TABLE IF EXISTS "instance_reset" ADD COLUMN IF NOT EXISTS "map" BIGINT NOT NULL DEFAULT '0' CHECK ("map" >= 0);
ALTER TABLE IF EXISTS "instance_reset" ADD COLUMN IF NOT EXISTS "reset_time" BIGINT NOT NULL DEFAULT '0';

CREATE TABLE IF NOT EXISTS "item_instance" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "item_id" BIGINT NOT NULL DEFAULT '0' CHECK ("item_id" >= 0),
    "owner_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("owner_guid" >= 0),
    "creator_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("creator_guid" >= 0),
    "gift_creator_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("gift_creator_guid" >= 0),
    "count" BIGINT NOT NULL DEFAULT '1' CHECK ("count" >= 0),
    "duration" INTEGER NOT NULL DEFAULT '0',
    "charges" TEXT,
    "flags" BIGINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0),
    "enchantments" TEXT NOT NULL,
    "random_property_id" SMALLINT NOT NULL DEFAULT '0',
    "durability" INTEGER NOT NULL DEFAULT '0' CHECK ("durability" >= 0),
    "text" BIGINT NOT NULL DEFAULT '0' CHECK ("text" >= 0),
    "generated_loot" SMALLINT DEFAULT '0',
    PRIMARY KEY ("guid")
);
ALTER TABLE IF EXISTS "item_instance" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "item_instance" ADD COLUMN IF NOT EXISTS "item_id" BIGINT NOT NULL DEFAULT '0' CHECK ("item_id" >= 0);
ALTER TABLE IF EXISTS "item_instance" ADD COLUMN IF NOT EXISTS "owner_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("owner_guid" >= 0);
ALTER TABLE IF EXISTS "item_instance" ADD COLUMN IF NOT EXISTS "creator_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("creator_guid" >= 0);
ALTER TABLE IF EXISTS "item_instance" ADD COLUMN IF NOT EXISTS "gift_creator_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("gift_creator_guid" >= 0);
ALTER TABLE IF EXISTS "item_instance" ADD COLUMN IF NOT EXISTS "count" BIGINT NOT NULL DEFAULT '1' CHECK ("count" >= 0);
ALTER TABLE IF EXISTS "item_instance" ADD COLUMN IF NOT EXISTS "duration" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_instance" ADD COLUMN IF NOT EXISTS "charges" TEXT;
ALTER TABLE IF EXISTS "item_instance" ADD COLUMN IF NOT EXISTS "flags" BIGINT NOT NULL DEFAULT '0' CHECK ("flags" >= 0);
ALTER TABLE IF EXISTS "item_instance" ADD COLUMN IF NOT EXISTS "enchantments" TEXT NOT NULL;
ALTER TABLE IF EXISTS "item_instance" ADD COLUMN IF NOT EXISTS "random_property_id" SMALLINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "item_instance" ADD COLUMN IF NOT EXISTS "durability" INTEGER NOT NULL DEFAULT '0' CHECK ("durability" >= 0);
ALTER TABLE IF EXISTS "item_instance" ADD COLUMN IF NOT EXISTS "text" BIGINT NOT NULL DEFAULT '0' CHECK ("text" >= 0);
ALTER TABLE IF EXISTS "item_instance" ADD COLUMN IF NOT EXISTS "generated_loot" SMALLINT DEFAULT '0';
CREATE INDEX IF NOT EXISTS idx_item_instance_idx_owner_guid ON "item_instance" ("owner_guid");
CREATE INDEX IF NOT EXISTS idx_item_instance_idx_itementry ON "item_instance" ("item_id");

CREATE TABLE IF NOT EXISTS "item_loot" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "owner_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("owner_guid" >= 0),
    "item_id" BIGINT NOT NULL DEFAULT '0' CHECK ("item_id" >= 0),
    "amount" BIGINT NOT NULL DEFAULT '0' CHECK ("amount" >= 0),
    "property" INTEGER NOT NULL DEFAULT '0',
    PRIMARY KEY ("guid", "item_id")
);
ALTER TABLE IF EXISTS "item_loot" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "item_loot" ADD COLUMN IF NOT EXISTS "owner_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("owner_guid" >= 0);
ALTER TABLE IF EXISTS "item_loot" ADD COLUMN IF NOT EXISTS "item_id" BIGINT NOT NULL DEFAULT '0' CHECK ("item_id" >= 0);
ALTER TABLE IF EXISTS "item_loot" ADD COLUMN IF NOT EXISTS "amount" BIGINT NOT NULL DEFAULT '0' CHECK ("amount" >= 0);
ALTER TABLE IF EXISTS "item_loot" ADD COLUMN IF NOT EXISTS "property" INTEGER NOT NULL DEFAULT '0';
CREATE INDEX IF NOT EXISTS idx_item_loot_idx_owner_guid ON "item_loot" ("owner_guid");

CREATE TABLE IF NOT EXISTS "item_text" (
    "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0),
    "text" TEXT,
    PRIMARY KEY ("id")
);
ALTER TABLE IF EXISTS "item_text" ADD COLUMN IF NOT EXISTS "id" BIGINT NOT NULL DEFAULT '0' CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "item_text" ADD COLUMN IF NOT EXISTS "text" TEXT;

CREATE TABLE IF NOT EXISTS "mail" (
    "id" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("id" >= 0),
    "message_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("message_type" >= 0),
    "stationery" SMALLINT NOT NULL DEFAULT '41',
    "mail_template_id" BIGINT NOT NULL DEFAULT '0' CHECK ("mail_template_id" >= 0),
    "sender_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("sender_guid" >= 0),
    "receiver_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("receiver_guid" >= 0),
    "subject" TEXT,
    "item_text_id" BIGINT NOT NULL DEFAULT '0' CHECK ("item_text_id" >= 0),
    "has_items" SMALLINT NOT NULL DEFAULT '0' CHECK ("has_items" >= 0),
    "expire_time" BIGINT NOT NULL DEFAULT '0',
    "deliver_time" BIGINT NOT NULL DEFAULT '0',
    "money" BIGINT NOT NULL DEFAULT '0' CHECK ("money" >= 0),
    "cod" BIGINT NOT NULL DEFAULT '0' CHECK ("cod" >= 0),
    "checked" SMALLINT NOT NULL DEFAULT '0' CHECK ("checked" >= 0),
    PRIMARY KEY ("id")
);
ALTER TABLE IF EXISTS "mail" ADD COLUMN IF NOT EXISTS "id" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL CHECK ("id" >= 0);
ALTER TABLE IF EXISTS "mail" ADD COLUMN IF NOT EXISTS "message_type" SMALLINT NOT NULL DEFAULT '0' CHECK ("message_type" >= 0);
ALTER TABLE IF EXISTS "mail" ADD COLUMN IF NOT EXISTS "stationery" SMALLINT NOT NULL DEFAULT '41';
ALTER TABLE IF EXISTS "mail" ADD COLUMN IF NOT EXISTS "mail_template_id" BIGINT NOT NULL DEFAULT '0' CHECK ("mail_template_id" >= 0);
ALTER TABLE IF EXISTS "mail" ADD COLUMN IF NOT EXISTS "sender_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("sender_guid" >= 0);
ALTER TABLE IF EXISTS "mail" ADD COLUMN IF NOT EXISTS "receiver_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("receiver_guid" >= 0);
ALTER TABLE IF EXISTS "mail" ADD COLUMN IF NOT EXISTS "subject" TEXT;
ALTER TABLE IF EXISTS "mail" ADD COLUMN IF NOT EXISTS "item_text_id" BIGINT NOT NULL DEFAULT '0' CHECK ("item_text_id" >= 0);
ALTER TABLE IF EXISTS "mail" ADD COLUMN IF NOT EXISTS "has_items" SMALLINT NOT NULL DEFAULT '0' CHECK ("has_items" >= 0);
ALTER TABLE IF EXISTS "mail" ADD COLUMN IF NOT EXISTS "expire_time" BIGINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "mail" ADD COLUMN IF NOT EXISTS "deliver_time" BIGINT NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "mail" ADD COLUMN IF NOT EXISTS "money" BIGINT NOT NULL DEFAULT '0' CHECK ("money" >= 0);
ALTER TABLE IF EXISTS "mail" ADD COLUMN IF NOT EXISTS "cod" BIGINT NOT NULL DEFAULT '0' CHECK ("cod" >= 0);
ALTER TABLE IF EXISTS "mail" ADD COLUMN IF NOT EXISTS "checked" SMALLINT NOT NULL DEFAULT '0' CHECK ("checked" >= 0);
CREATE INDEX IF NOT EXISTS idx_mail_idx_receiver ON "mail" ("receiver_guid");

CREATE TABLE IF NOT EXISTS "mail_items" (
    "mail_id" BIGINT NOT NULL DEFAULT '0' CHECK ("mail_id" >= 0),
    "item_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("item_guid" >= 0),
    "item_id" BIGINT NOT NULL DEFAULT '0' CHECK ("item_id" >= 0),
    "receiver_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("receiver_guid" >= 0),
    PRIMARY KEY ("mail_id", "item_guid")
);
ALTER TABLE IF EXISTS "mail_items" ADD COLUMN IF NOT EXISTS "mail_id" BIGINT NOT NULL DEFAULT '0' CHECK ("mail_id" >= 0);
ALTER TABLE IF EXISTS "mail_items" ADD COLUMN IF NOT EXISTS "item_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("item_guid" >= 0);
ALTER TABLE IF EXISTS "mail_items" ADD COLUMN IF NOT EXISTS "item_id" BIGINT NOT NULL DEFAULT '0' CHECK ("item_id" >= 0);
ALTER TABLE IF EXISTS "mail_items" ADD COLUMN IF NOT EXISTS "receiver_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("receiver_guid" >= 0);
CREATE INDEX IF NOT EXISTS idx_mail_items_idx_receiver ON "mail_items" ("receiver_guid");
CREATE INDEX IF NOT EXISTS idx_mail_items_idx_item_guid ON "mail_items" ("item_guid");

CREATE TABLE IF NOT EXISTS "pet_aura" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "caster_guid" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("caster_guid" >= 0),
    "item_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("item_guid" >= 0),
    "spell" BIGINT NOT NULL DEFAULT '0' CHECK ("spell" >= 0),
    "stacks" BIGINT NOT NULL DEFAULT '1' CHECK ("stacks" >= 0),
    "charges" BIGINT NOT NULL DEFAULT '0' CHECK ("charges" >= 0),
    "base_points0" REAL NOT NULL DEFAULT '0',
    "base_points1" REAL NOT NULL DEFAULT '0',
    "base_points2" REAL NOT NULL DEFAULT '0',
    "periodic_time0" BIGINT NOT NULL DEFAULT '0' CHECK ("periodic_time0" >= 0),
    "periodic_time1" BIGINT NOT NULL DEFAULT '0' CHECK ("periodic_time1" >= 0),
    "periodic_time2" BIGINT NOT NULL DEFAULT '0' CHECK ("periodic_time2" >= 0),
    "max_duration" INTEGER NOT NULL DEFAULT '0',
    "duration" INTEGER NOT NULL DEFAULT '0',
    "effect_index_mask" SMALLINT NOT NULL DEFAULT '0' CHECK ("effect_index_mask" >= 0),
    PRIMARY KEY ("guid", "caster_guid", "item_guid", "spell")
);
ALTER TABLE IF EXISTS "pet_aura" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "pet_aura" ADD COLUMN IF NOT EXISTS "caster_guid" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("caster_guid" >= 0);
ALTER TABLE IF EXISTS "pet_aura" ADD COLUMN IF NOT EXISTS "item_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("item_guid" >= 0);
ALTER TABLE IF EXISTS "pet_aura" ADD COLUMN IF NOT EXISTS "spell" BIGINT NOT NULL DEFAULT '0' CHECK ("spell" >= 0);
ALTER TABLE IF EXISTS "pet_aura" ADD COLUMN IF NOT EXISTS "stacks" BIGINT NOT NULL DEFAULT '1' CHECK ("stacks" >= 0);
ALTER TABLE IF EXISTS "pet_aura" ADD COLUMN IF NOT EXISTS "charges" BIGINT NOT NULL DEFAULT '0' CHECK ("charges" >= 0);
ALTER TABLE IF EXISTS "pet_aura" ADD COLUMN IF NOT EXISTS "base_points0" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "pet_aura" ADD COLUMN IF NOT EXISTS "base_points1" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "pet_aura" ADD COLUMN IF NOT EXISTS "base_points2" REAL NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "pet_aura" ADD COLUMN IF NOT EXISTS "periodic_time0" BIGINT NOT NULL DEFAULT '0' CHECK ("periodic_time0" >= 0);
ALTER TABLE IF EXISTS "pet_aura" ADD COLUMN IF NOT EXISTS "periodic_time1" BIGINT NOT NULL DEFAULT '0' CHECK ("periodic_time1" >= 0);
ALTER TABLE IF EXISTS "pet_aura" ADD COLUMN IF NOT EXISTS "periodic_time2" BIGINT NOT NULL DEFAULT '0' CHECK ("periodic_time2" >= 0);
ALTER TABLE IF EXISTS "pet_aura" ADD COLUMN IF NOT EXISTS "max_duration" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "pet_aura" ADD COLUMN IF NOT EXISTS "duration" INTEGER NOT NULL DEFAULT '0';
ALTER TABLE IF EXISTS "pet_aura" ADD COLUMN IF NOT EXISTS "effect_index_mask" SMALLINT NOT NULL DEFAULT '0' CHECK ("effect_index_mask" >= 0);

CREATE TABLE IF NOT EXISTS "pet_spell" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "spell" BIGINT NOT NULL DEFAULT '0' CHECK ("spell" >= 0),
    "active" BIGINT NOT NULL DEFAULT '0' CHECK ("active" >= 0),
    PRIMARY KEY ("guid", "spell")
);
ALTER TABLE IF EXISTS "pet_spell" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "pet_spell" ADD COLUMN IF NOT EXISTS "spell" BIGINT NOT NULL DEFAULT '0' CHECK ("spell" >= 0);
ALTER TABLE IF EXISTS "pet_spell" ADD COLUMN IF NOT EXISTS "active" BIGINT NOT NULL DEFAULT '0' CHECK ("active" >= 0);

CREATE TABLE IF NOT EXISTS "pet_spell_cooldown" (
    "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0),
    "spell" BIGINT NOT NULL DEFAULT '0' CHECK ("spell" >= 0),
    "time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("time" >= 0),
    PRIMARY KEY ("guid", "spell")
);
ALTER TABLE IF EXISTS "pet_spell_cooldown" ADD COLUMN IF NOT EXISTS "guid" BIGINT NOT NULL DEFAULT '0' CHECK ("guid" >= 0);
ALTER TABLE IF EXISTS "pet_spell_cooldown" ADD COLUMN IF NOT EXISTS "spell" BIGINT NOT NULL DEFAULT '0' CHECK ("spell" >= 0);
ALTER TABLE IF EXISTS "pet_spell_cooldown" ADD COLUMN IF NOT EXISTS "time" NUMERIC(20,0) NOT NULL DEFAULT '0' CHECK ("time" >= 0);

CREATE TABLE IF NOT EXISTS "petition" (
    "owner_guid" BIGINT NOT NULL CHECK ("owner_guid" >= 0),
    "petition_guid" BIGINT DEFAULT '0' CHECK ("petition_guid" >= 0),
    "charter_guid" BIGINT DEFAULT NULL CHECK ("charter_guid" >= 0),
    "name" VARCHAR(255) NOT NULL DEFAULT '',
    PRIMARY KEY ("owner_guid"),
    UNIQUE ("owner_guid", "petition_guid"),
    UNIQUE ("charter_guid")
);
ALTER TABLE IF EXISTS "petition" ADD COLUMN IF NOT EXISTS "owner_guid" BIGINT NOT NULL CHECK ("owner_guid" >= 0);
ALTER TABLE IF EXISTS "petition" ADD COLUMN IF NOT EXISTS "petition_guid" BIGINT DEFAULT '0' CHECK ("petition_guid" >= 0);
ALTER TABLE IF EXISTS "petition" ADD COLUMN IF NOT EXISTS "charter_guid" BIGINT DEFAULT NULL CHECK ("charter_guid" >= 0);
ALTER TABLE IF EXISTS "petition" ADD COLUMN IF NOT EXISTS "name" VARCHAR(255) NOT NULL DEFAULT '';
CREATE UNIQUE INDEX IF NOT EXISTS idx_petition_key_ownerguid_petitionguid ON "petition" ("owner_guid", "petition_guid");
CREATE UNIQUE INDEX IF NOT EXISTS idx_petition_charterguid ON "petition" ("charter_guid");

CREATE TABLE IF NOT EXISTS "petition_sign" (
    "owner_guid" BIGINT NOT NULL CHECK ("owner_guid" >= 0),
    "petition_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("petition_guid" >= 0),
    "player_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("player_guid" >= 0),
    "player_account" BIGINT NOT NULL DEFAULT '0' CHECK ("player_account" >= 0),
    PRIMARY KEY ("petition_guid", "player_guid")
);
ALTER TABLE IF EXISTS "petition_sign" ADD COLUMN IF NOT EXISTS "owner_guid" BIGINT NOT NULL CHECK ("owner_guid" >= 0);
ALTER TABLE IF EXISTS "petition_sign" ADD COLUMN IF NOT EXISTS "petition_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("petition_guid" >= 0);
ALTER TABLE IF EXISTS "petition_sign" ADD COLUMN IF NOT EXISTS "player_guid" BIGINT NOT NULL DEFAULT '0' CHECK ("player_guid" >= 0);
ALTER TABLE IF EXISTS "petition_sign" ADD COLUMN IF NOT EXISTS "player_account" BIGINT NOT NULL DEFAULT '0' CHECK ("player_account" >= 0);
CREATE INDEX IF NOT EXISTS idx_petition_sign_idx_playerguid ON "petition_sign" ("player_guid");
CREATE INDEX IF NOT EXISTS idx_petition_sign_idx_ownerguid ON "petition_sign" ("owner_guid");

CREATE TABLE IF NOT EXISTS "playerbot" (
    "char_guid" NUMERIC(20,0) NOT NULL CHECK ("char_guid" >= 0),
    "chance" BIGINT NOT NULL DEFAULT '10' CHECK ("chance" >= 0),
    "comment" VARCHAR(255) DEFAULT NULL,
    "ai" VARCHAR(50) DEFAULT NULL,
    PRIMARY KEY ("char_guid")
);
ALTER TABLE IF EXISTS "playerbot" ADD COLUMN IF NOT EXISTS "char_guid" NUMERIC(20,0) NOT NULL CHECK ("char_guid" >= 0);
ALTER TABLE IF EXISTS "playerbot" ADD COLUMN IF NOT EXISTS "chance" BIGINT NOT NULL DEFAULT '10' CHECK ("chance" >= 0);
ALTER TABLE IF EXISTS "playerbot" ADD COLUMN IF NOT EXISTS "comment" VARCHAR(255) DEFAULT NULL;
ALTER TABLE IF EXISTS "playerbot" ADD COLUMN IF NOT EXISTS "ai" VARCHAR(50) DEFAULT NULL;

CREATE TABLE IF NOT EXISTS "saved_variables" (
    "key" SMALLINT NOT NULL DEFAULT '0' CHECK ("key" >= 0),
    "cleaning_flags" BIGINT NOT NULL DEFAULT '0' CHECK ("cleaning_flags" >= 0),
    "honor_last_maintenance_day" BIGINT NOT NULL DEFAULT '0' CHECK ("honor_last_maintenance_day" >= 0),
    "honor_next_maintenance_day" BIGINT NOT NULL DEFAULT '0' CHECK ("honor_next_maintenance_day" >= 0),
    "honor_maintenance_marker" SMALLINT NOT NULL DEFAULT '0' CHECK ("honor_maintenance_marker" >= 0),
    PRIMARY KEY ("key")
);
ALTER TABLE IF EXISTS "saved_variables" ADD COLUMN IF NOT EXISTS "key" SMALLINT NOT NULL DEFAULT '0' CHECK ("key" >= 0);
ALTER TABLE IF EXISTS "saved_variables" ADD COLUMN IF NOT EXISTS "cleaning_flags" BIGINT NOT NULL DEFAULT '0' CHECK ("cleaning_flags" >= 0);
ALTER TABLE IF EXISTS "saved_variables" ADD COLUMN IF NOT EXISTS "honor_last_maintenance_day" BIGINT NOT NULL DEFAULT '0' CHECK ("honor_last_maintenance_day" >= 0);
ALTER TABLE IF EXISTS "saved_variables" ADD COLUMN IF NOT EXISTS "honor_next_maintenance_day" BIGINT NOT NULL DEFAULT '0' CHECK ("honor_next_maintenance_day" >= 0);
ALTER TABLE IF EXISTS "saved_variables" ADD COLUMN IF NOT EXISTS "honor_maintenance_marker" SMALLINT NOT NULL DEFAULT '0' CHECK ("honor_maintenance_marker" >= 0);

CREATE TABLE IF NOT EXISTS "world" (
    "map" BIGINT NOT NULL DEFAULT '0' CHECK ("map" >= 0),
    "data" TEXT,
    PRIMARY KEY ("map")
);
ALTER TABLE IF EXISTS "world" ADD COLUMN IF NOT EXISTS "map" BIGINT NOT NULL DEFAULT '0' CHECK ("map" >= 0);
ALTER TABLE IF EXISTS "world" ADD COLUMN IF NOT EXISTS "data" TEXT;

CREATE TABLE IF NOT EXISTS "worldstates" (
    "entry" INTEGER DEFAULT NULL,
    "value" INTEGER DEFAULT NULL,
    "comment" VARCHAR(255) DEFAULT NULL,
    UNIQUE ("entry")
);
ALTER TABLE IF EXISTS "worldstates" ADD COLUMN IF NOT EXISTS "entry" INTEGER DEFAULT NULL;
ALTER TABLE IF EXISTS "worldstates" ADD COLUMN IF NOT EXISTS "value" INTEGER DEFAULT NULL;
ALTER TABLE IF EXISTS "worldstates" ADD COLUMN IF NOT EXISTS "comment" VARCHAR(255) DEFAULT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_worldstates_key_entry ON "worldstates" ("entry");
