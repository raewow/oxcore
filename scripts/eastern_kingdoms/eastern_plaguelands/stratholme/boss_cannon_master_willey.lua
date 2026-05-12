--[[
    @script_type: creature_ai
    @entry: 10997
    @name: boss_cannon_master_willey

    Cannon Master Willey - Stratholme
    Ported from MaNGOS boss_cannon_master_willey.cpp

    Mechanics:
    - Knock Away on current target every 14s (initial 11s)
    - Pummel on current target every 12s (initial 7s)
    - Shoot on current target every 1s
    - Summons 3 Crimson Riflemen (entry 11054) every 30s (initial 15s)
    - On death, spawns 7 Crimson Riflemen at fixed positions
]]

local SPELL_KNOCK_AWAY = 10101
local SPELL_PUMMEL     = 15615
local SPELL_SHOOT      = 20463

local NPC_CRIMSON_RIFLEMAN = 11054

local TIMER_KNOCK_AWAY       = 1
local TIMER_PUMMEL           = 2
local TIMER_SHOOT            = 3
local TIMER_SUMMON_RIFLEMAN  = 4

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_SHOOT, duration = 1000 },
        { action = "SET_TIMER", timer_id = TIMER_PUMMEL, duration = 7000 },
        { action = "SET_TIMER", timer_id = TIMER_KNOCK_AWAY, duration = 11000 },
        { action = "SET_TIMER", timer_id = TIMER_SUMMON_RIFLEMAN, duration = 15000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Shoot
    if input:IsTimerReady(TIMER_SHOOT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHOOT, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHOOT, duration = 1000 })
    end

    -- Pummel
    if input:IsTimerReady(TIMER_PUMMEL) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_PUMMEL, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PUMMEL, duration = 12000 })
    end

    -- Knock Away
    if input:IsTimerReady(TIMER_KNOCK_AWAY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_KNOCK_AWAY, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_KNOCK_AWAY, duration = 14000 })
    end

    -- Summon Riflemen (3 per wave)
    if input:IsTimerReady(TIMER_SUMMON_RIFLEMAN) then
        table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_CRIMSON_RIFLEMAN, target = "self" })
        table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_CRIMSON_RIFLEMAN, target = "self" })
        table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_CRIMSON_RIFLEMAN, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SUMMON_RIFLEMAN, duration = 30000 })
    end

    return actions
end

function boss:OnDeath(input)
    -- Spawn 7 Crimson Riflemen on death
    return {
        { action = "SPAWN_CREATURE", entry = NPC_CRIMSON_RIFLEMAN, target = "self" },
        { action = "SPAWN_CREATURE", entry = NPC_CRIMSON_RIFLEMAN, target = "self" },
        { action = "SPAWN_CREATURE", entry = NPC_CRIMSON_RIFLEMAN, target = "self" },
        { action = "SPAWN_CREATURE", entry = NPC_CRIMSON_RIFLEMAN, target = "self" },
        { action = "SPAWN_CREATURE", entry = NPC_CRIMSON_RIFLEMAN, target = "self" },
        { action = "SPAWN_CREATURE", entry = NPC_CRIMSON_RIFLEMAN, target = "self" },
        { action = "SPAWN_CREATURE", entry = NPC_CRIMSON_RIFLEMAN, target = "self" },
    }
end

RegisterCreatureAI(10997, boss)
