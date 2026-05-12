--[[
    @script_type: creature_ai
    @entry: 15369
    @name: boss_ayamiss

    Ayamiss the Hunter - Ruins of Ahn'Qiraj
    Verified against vmangos boss_ayamiss.cpp

    Abilities:
    - Stinger Spray: 10s init, urand(10,15)s repeat (all phases)
    - Poison Stinger: 5s init, 3s repeat (air phase only)
    - Paralyze: 10s init, 15s repeat (random player + spawn larva)
    - Lash: 15s init, urand(10,20)s repeat (ground phase only)
    - Trash: 10s init, urand(10,20)s repeat (ground phase only)
    - Summon Swarmers: 60s repeat, 20 swarmers each wave
    - Frenzy: At 20% HP (one-time)

    Phases:
    - Phase 0 (AIR): Ranged attacks, fly mode; transitions to ground at 70% HP
    - Phase 1 (GROUND): Melee combat, threat reset on transition
]]

local SPELL_STINGER_SPRAY   = 25749
local SPELL_POISON_STINGER  = 25748
local SPELL_PARALYZE        = 25725
local SPELL_LASH            = 25852
local SPELL_TRASH           = 3391
local SPELL_FRENZY          = 8269

local NPC_LARVA             = 15555
local NPC_SWARMER           = 15546

local PHASE_AIR             = 0
local PHASE_GROUND          = 1

local TIMER_STINGER_SPRAY   = 1
local TIMER_POISON_STINGER  = 2
local TIMER_PARALYZE        = 3
local TIMER_LASH            = 4
local TIMER_TRASH           = 5
local TIMER_SWARMERS        = 6

-- Larva spawn locations (from vmangos C++)
local LARVA_LOCS = {
    { x = -9700.20, y = 1567.98, z = 23.90 },
    { x = -9659.25, y = 1530.88, z = 22.34 },
}

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_PHASE", phase = PHASE_AIR },
        { action = "SET_TIMER", timer_id = TIMER_STINGER_SPRAY,  duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_POISON_STINGER, duration = 5000 },
        { action = "SET_TIMER", timer_id = TIMER_PARALYZE,       duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_LASH,           duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_TRASH,          duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_SWARMERS,       duration = 60000 },
        { action = "SET_CUSTOM_DATA", key = "frenzied", value = 0 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_PHASE", phase = PHASE_AIR },
        { action = "SET_CUSTOM_DATA", key = "frenzied", value = 0 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Frenzy at 20% HP (one-time)
    local frenzied = input:GetCustomData("frenzied") or 0
    if frenzied == 0 and input.health_pct and input.health_pct <= 0.20 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FRENZY, target = "self" })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "frenzied", value = 1 })
    end

    -- Phase transition: air -> ground at 70% HP (with threat reset)
    if input.phase == PHASE_AIR and input.health_pct and input.health_pct <= 0.70 then
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_GROUND })
        table.insert(actions, { action = "RESET_THREAT" })
        return actions
    end

    -- Stinger Spray (all phases, urand(10,15)s repeat)
    if input:IsTimerReady(TIMER_STINGER_SPRAY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_STINGER_SPRAY, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_STINGER_SPRAY, duration = math.random(10000, 15000) })
    end

    -- Paralyze random player + summon larva (all phases, 15s repeat)
    if input:IsTimerReady(TIMER_PARALYZE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_PARALYZE, target = "random_hostile" })
        local loc = LARVA_LOCS[math.random(1, 2)]
        table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_LARVA,
            x = loc.x, y = loc.y, z = loc.z, o = 0, summon_type = 1, duration = 15000 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PARALYZE, duration = 15000 })
    end

    -- Summon 20 swarmers every 60s
    if input:IsTimerReady(TIMER_SWARMERS) then
        for _ = 1, 20 do
            table.insert(actions, { action = "SPAWN_CREATURE_NEAR_SELF", entry = NPC_SWARMER })
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SWARMERS, duration = 60000 })
    end

    if input.phase == PHASE_AIR then
        -- Poison Stinger (air phase only, 3s repeat)
        if input:IsTimerReady(TIMER_POISON_STINGER) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_POISON_STINGER, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_POISON_STINGER, duration = 3000 })
        end
    else
        -- Ground phase: melee abilities
        -- Lash (urand(10,20)s repeat)
        if input:IsTimerReady(TIMER_LASH) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_LASH, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_LASH, duration = math.random(10000, 20000) })
        end

        -- Trash (urand(10,20)s repeat)
        if input:IsTimerReady(TIMER_TRASH) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TRASH, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TRASH, duration = math.random(10000, 20000) })
        end
    end

    return actions
end

RegisterCreatureAI(15369, boss)
