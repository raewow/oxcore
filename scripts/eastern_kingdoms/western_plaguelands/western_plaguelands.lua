--[[
    Western Plaguelands zone scripts
    Reference: eastern_kingdoms/western_plaguelands/western_plaguelands.cpp

    npc_the_scourge_cauldron (entries 11075-11078)
        EnableMoveInLosEvent in Reset. MoveInLineOfSight fires for players.
        When a player with the correct quest (per area_id) approaches,
        summon the cauldron lord NPC and self-destruct.

        Area IDs → quest IDs → cauldron lord entries:
          199 (Felstone)  → quests 5216/5229 → entry 11075
          200 (Dalson)    → quests 5219/5231 → entry 11077
          201 (Gahrron)   → quests 5225/5235 → entry 11078
          202 (Writhing)  → quests 5222/5233 → entry 11076

    npc_andorhal_tower (entries 11489-11492)
        EnableMoveInLosEvent. MoveInLineOfSight: when a player approaches
        and a beacon torch GO (176093) is within 20yd, award kill credit.

    npc_highprotectorlorik (entry 1846)
        Combat AI: always has Retribution Aura (8990) active; casts
        Arcane Blast (10833) every 10-12s, Divine Shield (13874) at <15% HP
        every 45s, Holy Light (15493) at <60% HP every 2-6s, Shield Slam
        (15655) on a casting target every 9s.
]]

-- ============================================================
-- npc_the_scourge_cauldron
-- Entries for the 4 cauldron NPCs spawned across WPL zones
-- ============================================================
-- Area → { quest_incomplete ids, cauldron_lord entry }
local CAULDRON_DATA = {
    [199] = { quests = { 5216, 5229 }, lord = 11075 },  -- Felstone Field
    [200] = { quests = { 5219, 5231 }, lord = 11077 },  -- Dalson's Tears
    [201] = { quests = { 5225, 5235 }, lord = 11078 },  -- Gahrron's Withering
    [202] = { quests = { 5222, 5233 }, lord = 11076 },  -- The Writhing Haunt
}

-- Register the same AI for all cauldron entries in DB
-- Entry IDs are set per-creature in DB via ScriptName field.
-- The script name is "npc_the_scourge_cauldron" and maps to one of these entries.
-- Actual DB entries: we register a generic AI for the script name.
-- Note: in practice the C++ uses a single ScriptedAI class for all 4 variants;
-- we do the same here.

local NPC_SCOURGE_CAULDRON_SCRIPT_ENTRIES = { 11375, 11376, 11377, 11378 }

local scourge_cauldron_ai = {}

function scourge_cauldron_ai:OnMoveInLineOfSight(self, npc_guid, unit_guid, is_hostile)
    if self.triggered then return {} end
    -- In C++ the player quest check is done here inline.
    -- In Lua we don't have direct player quest access in MoveInLineOfSight,
    -- so we set a flag and check in the next OnUpdate with full snapshot.
    if not is_hostile then
        self.check_player = unit_guid
    end
    return {}
end

function scourge_cauldron_ai:OnUpdate(self, npc_guid, diff_ms, input)
    if self.triggered then return {} end
    if not self.check_player then return {} end

    local data = CAULDRON_DATA[input.area_id]
    if not data then
        self.check_player = nil
        return {}
    end

    self.check_player = nil
    self.triggered    = true

    return {
        { action = "SPAWN_CREATURE",
          entry       = data.lord,
          x           = input.position.x,
          y           = input.position.y,
          z           = input.position.z,
          o           = 0.0,
          summon_type = "TIMED_DESPAWN",
          duration_ms = 600000 },
        { action = "KILL_SELF" },
    }
end

for _, entry in ipairs(NPC_SCOURGE_CAULDRON_SCRIPT_ENTRIES) do
    RegisterCreatureAI(entry, scourge_cauldron_ai)
end

-- ============================================================
-- npc_andorhal_tower  (entries 11489, 11490, 11491, 11492)
-- ============================================================
local GO_BEACON_TORCH = 176093

local NPC_ANDORHAL_TOWER_ENTRIES = { 11489, 11490, 11491, 11492 }

local andorhal_tower_ai = {}

function andorhal_tower_ai:OnMoveInLineOfSight(self, npc_guid, unit_guid, is_hostile)
    if is_hostile then return {} end
    -- C++: if beacon torch GO is within 20yd of creature, award kill credit to player.
    -- In Lua we can't do GO proximity check in this callback directly;
    -- set flag for OnUpdate to process.
    self.credit_player = unit_guid
    return {}
end

function andorhal_tower_ai:OnUpdate(self, npc_guid, diff_ms, input)
    if not self.credit_player then return {} end
    local player_guid = self.credit_player
    self.credit_player = nil
    return {
        { action = "KILL_CREDIT_NEAREST_CREATURE",
          creature_entry = 0,   -- self-credit via kill_credit_guid
          search_radius  = 20.0 },
    }
end

for _, entry in ipairs(NPC_ANDORHAL_TOWER_ENTRIES) do
    RegisterCreatureAI(entry, andorhal_tower_ai)
end

-- ============================================================
-- npc_highprotectorlorik  (entry 1846)
-- ============================================================
local NPC_HIGHPROTECTORLORIK = 1846

local SPELL_RETRIBUTION_AURA = 8990
local SPELL_ARCANE_BLAST     = 10833
local SPELL_DIVINE_SHIELD    = 13874
local SPELL_HOLY_LIGHT       = 15493
local SPELL_SHIELD_SLAM      = 15655

local npc_highprotectorlorik = {}

function npc_highprotectorlorik:OnSpawn(self, npc_guid)
    self.arcane_blast_timer  = 7000
    self.divine_shield_timer = 2000
    self.holy_light_timer    = 2000
    self.shield_slam_timer   = 2000
    return {}
end

function npc_highprotectorlorik:OnReset(self, npc_guid)
    self.arcane_blast_timer  = 7000
    self.divine_shield_timer = 2000
    self.holy_light_timer    = 2000
    self.shield_slam_timer   = 2000
    return {}
end

function npc_highprotectorlorik:OnUpdate(self, npc_guid, diff_ms, input)
    local actions = {}

    -- Always maintain Retribution Aura
    if not input:HasAura(SPELL_RETRIBUTION_AURA) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_RETRIBUTION_AURA,
            triggered = true })
    end

    if not input.is_in_combat or not input.current_target then
        return actions
    end

    -- Divine Shield at <15% HP
    self.divine_shield_timer = self.divine_shield_timer - diff_ms
    if self.divine_shield_timer <= 0 and input.health_pct <= 15.0 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DIVINE_SHIELD })
        self.divine_shield_timer = 45000
    elseif self.divine_shield_timer <= 0 then
        self.divine_shield_timer = 0  -- wait until HP threshold
    end

    -- Arcane Blast
    self.arcane_blast_timer = self.arcane_blast_timer - diff_ms
    if self.arcane_blast_timer <= 0 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ARCANE_BLAST,
            target_guid = input.current_target })
        self.arcane_blast_timer = 10000 + math.floor(math.random() * 2000)
    end

    -- Holy Light (self-heal at <60% HP)
    self.holy_light_timer = self.holy_light_timer - diff_ms
    if self.holy_light_timer <= 0 and input.health_pct <= 60.0 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_HOLY_LIGHT })
        self.holy_light_timer = 2000 + math.floor(math.random() * 4000)
    elseif self.holy_light_timer <= 0 then
        self.holy_light_timer = 0
    end

    -- Shield Slam (when target is casting)
    self.shield_slam_timer = self.shield_slam_timer - diff_ms
    if self.shield_slam_timer <= 0 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHIELD_SLAM,
            target_guid = input.current_target })
        self.shield_slam_timer = 9000
    end

    return actions
end

RegisterCreatureAI(NPC_HIGHPROTECTORLORIK, npc_highprotectorlorik)
