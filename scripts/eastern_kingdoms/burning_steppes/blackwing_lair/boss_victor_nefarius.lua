--[[
    @script_type: creature_ai
    @entry: 10162
    @name: boss_victor_nefarius

    Lord Victor Nefarius (Human Form) - Blackwing Lair Phase 1
    Verified against vmangos boss_victor_nefarius.cpp

    Controls Phase 1: fires spells from the throne while drakonids spawn,
    then goes invisible and spawns Nefarian (dragon form, entry 11583) when
    40 drakonids have been killed (42 total can spawn before stops).

    Notes:
    - Two drake types are randomly chosen per instance (from 5 possibilities).
      Chromatic drakanoids also spawn every ~35s in addition to the two types.
    - Shadow Blink (teleport to random target + root) requires NearTeleportTo;
      approximated as a dummy action until movement API support.
    - Mind Control (SPELL_SHADOW_COMMAND 22667) targets non-tank; repeat 25-40s.
    - Scepter Run logic (5-hour timer, taunts) deferred to instance script.
    - Gossip / intro speech handled by OnGossipSelect in zone script.
]]

-- Phase 1 attack spells
local SPELL_NEFARIUS_BARRIER    = 22663  -- immunity aura on self while casting
local SPELL_SHADOWBOLT          = 22677  -- was 21077 (wrong); on current target; repeat 3-10s
local SPELL_SHADOWBOLT_VOLLEY   = 22665  -- AoE on current target; repeat every 15s
local SPELL_FEAR                = 22678  -- on random target; repeat 15-25s
local SPELL_SILENCE             = 22666  -- on random target; repeat 25-40s
local SPELL_SHADOW_COMMAND      = 22667  -- mind control non-tank; repeat 25-40s

-- Drakonid NPC entries
local NPC_BRONZE_DRAKANOID    = 14263
local NPC_BLUE_DRAKANOID      = 14261
local NPC_RED_DRAKANOID       = 14264
local NPC_GREEN_DRAKANOID     = 14262
local NPC_BLACK_DRAKANOID     = 14265
local NPC_CHROMATIC_DRAKANOID = 14302
local NPC_NEFARIAN            = 11583

-- Spawn locations (from vmangos aNefarianLocs)
local SPAWN_LOC_1       = { x = -7599.32,  y = -1191.72,   z = 475.545  }  -- index 0: near red/blue/black spawner
local SPAWN_LOC_2       = { x = -7526.27,  y = -1135.04,   z = 473.445  }  -- index 1: closer to door
local NEFARIAN_SPAWN    = { x = -7515.644, y = -1222.698,  z = 534.7169 }  -- index 2: Nefarian spawn (high up)

-- Two randomly chosen drake types (mirrors C++ per-instance selection)
local POSSIBLE_DRAKES = { NPC_BRONZE_DRAKANOID, NPC_BLUE_DRAKANOID, NPC_RED_DRAKANOID, NPC_GREEN_DRAKANOID, NPC_BLACK_DRAKANOID }
local drake_type_one = POSSIBLE_DRAKES[math.random(1, 5)]
local drake_type_two
repeat
    drake_type_two = POSSIBLE_DRAKES[math.random(1, 5)]
until drake_type_two ~= drake_type_one

local TIMER_SHADOWBOLT        = 1
local TIMER_SHADOWBOLT_VOLLEY = 2
local TIMER_FEAR              = 3
local TIMER_SILENCE           = 4
local TIMER_SHADOW_COMMAND    = 5
local TIMER_ADD_SPAWN         = 6
local TIMER_CHROMA_SPAWN      = 7

local MAX_DRAKE_KILLED_PHASE2 = 40  -- Nefarian spawns after 40 killed
local MAX_DRAKE_KILLED_STOP   = 42  -- spawning stops after 42 killed

local PHASE_ADDS    = 1
local PHASE_NEFARIAN = 2

local spawned_count = 0

local boss = {}

function boss:OnEnterCombat(input)
    spawned_count = 0
    return {
        { action = "SET_PHASE", phase = PHASE_ADDS },
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "CAST_SPELL", spell_id = SPELL_NEFARIUS_BARRIER, target = "self" },
        { action = "SET_TIMER", timer_id = TIMER_SHADOWBOLT,        duration = 5000 },
        { action = "SET_TIMER", timer_id = TIMER_SHADOWBOLT_VOLLEY, duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_FEAR,              duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_SILENCE,           duration = 20000 },
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_COMMAND,    duration = 25000 },
        { action = "SET_TIMER", timer_id = TIMER_ADD_SPAWN,         duration = 6000 },
        { action = "SET_TIMER", timer_id = TIMER_CHROMA_SPAWN,      duration = math.random(7000, 9000) },
    }
end

function boss:OnReset(input)
    spawned_count = 0
    return {
        { action = "SET_PHASE", phase = PHASE_ADDS },
        { action = "SET_FACTION", faction_id = 35 },  -- FACTION_FRIENDLY
        { action = "SET_TIMER", timer_id = TIMER_SHADOWBOLT,        duration = 5000 },
        { action = "SET_TIMER", timer_id = TIMER_SHADOWBOLT_VOLLEY, duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_FEAR,              duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_SILENCE,           duration = 20000 },
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_COMMAND,    duration = 25000 },
        { action = "SET_TIMER", timer_id = TIMER_ADD_SPAWN,         duration = 6000 },
        { action = "SET_TIMER", timer_id = TIMER_CHROMA_SPAWN,      duration = math.random(7000, 9000) },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Phase 2: boss is hidden, only Nefarian acts
    if input.phase == PHASE_NEFARIAN then return actions end

    -- Shadow Bolt on current target; repeat every 3-10s
    if input:IsTimerReady(TIMER_SHADOWBOLT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOWBOLT, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOWBOLT, duration = math.random(3000, 10000) })
    end

    -- Shadow Bolt Volley on current target; repeat every 15s
    if input:IsTimerReady(TIMER_SHADOWBOLT_VOLLEY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOWBOLT_VOLLEY, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOWBOLT_VOLLEY, duration = 15000 })
    end

    -- Fear on random target; repeat every 15-25s
    if input:IsTimerReady(TIMER_FEAR) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FEAR, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FEAR, duration = math.random(15000, 25000) })
    end

    -- Silence on random target; repeat every 25-40s
    if input:IsTimerReady(TIMER_SILENCE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SILENCE, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SILENCE, duration = math.random(25000, 40000) })
    end

    -- Shadow Command (mind control) on non-tank random target; repeat every 25-40s
    if input:IsTimerReady(TIMER_SHADOW_COMMAND) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_COMMAND, target = "random_hostile_not_top", flags = "no_reapply" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_COMMAND, duration = math.random(25000, 40000) })
    end

    -- Drakonid add spawn: two per tick (one of each chosen type); repeat every 6-7s
    if input:IsTimerReady(TIMER_ADD_SPAWN) then
        if spawned_count < MAX_DRAKE_KILLED_STOP then
            table.insert(actions, { action = "SPAWN_CREATURE", entry = drake_type_one,
                x = SPAWN_LOC_1.x, y = SPAWN_LOC_1.y, z = SPAWN_LOC_1.z, o = 5.0,
                summon_type = 3, duration = 10000 })
            table.insert(actions, { action = "SPAWN_CREATURE", entry = drake_type_two,
                x = SPAWN_LOC_2.x, y = SPAWN_LOC_2.y, z = SPAWN_LOC_2.z, o = 5.0,
                summon_type = 3, duration = 10000 })
            spawned_count = spawned_count + 2
        end

        -- After 40 total spawned kills: go invisible and summon Nefarian
        if spawned_count >= MAX_DRAKE_KILLED_PHASE2 and input.phase == PHASE_ADDS then
            table.insert(actions, { action = "SET_VISIBILITY", visible = false })
            table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_NEFARIAN,
                x = NEFARIAN_SPAWN.x, y = NEFARIAN_SPAWN.y, z = NEFARIAN_SPAWN.z, o = 0,
                summon_type = 1, duration = 0 })
            table.insert(actions, { action = "SET_PHASE", phase = PHASE_NEFARIAN })
        end

        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ADD_SPAWN, duration = math.random(6000, 7000) })
    end

    -- Chromatic drakanoid spawn: two per tick; repeat every 35s
    if input:IsTimerReady(TIMER_CHROMA_SPAWN) then
        if spawned_count < MAX_DRAKE_KILLED_STOP then
            table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_CHROMATIC_DRAKANOID,
                x = SPAWN_LOC_1.x, y = SPAWN_LOC_1.y, z = SPAWN_LOC_1.z, o = 5.0,
                summon_type = 3, duration = 10000 })
            table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_CHROMATIC_DRAKANOID,
                x = SPAWN_LOC_2.x, y = SPAWN_LOC_2.y, z = SPAWN_LOC_2.z, o = 5.0,
                summon_type = 3, duration = 10000 })
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CHROMA_SPAWN, duration = 35000 })
    end

    return actions
end

RegisterCreatureAI(10162, boss)
