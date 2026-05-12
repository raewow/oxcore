--[[
    @script_type: creature_ai
    @entry: 12057
    @name: boss_garr

    Garr - Molten Core
    Ported from vmangos boss_garr.cpp

    Also includes Firesworn add AI (entry 12099).
]]

-- Garr Spells (exact from vmangos)
local SPELL_ANTIMAGICPULSE = 19492
local SPELL_MAGMASHACKLES = 19496
local SPELL_ENRAGE = 19516            -- Stacks up to 10 (gained when Firesworn dies)
local SPELL_SEPARATION_ANXIETY = 23487
local SPELL_ERUPTION_TRIGGER = 20482  -- Forces a Firesworn to explode (after 6 min)

-- Firesworn Spells
local SPELL_THRASH = 3391
local SPELL_IMMOLATE = 15732
local SPELL_ADD_ERUPTION = 19497      -- Normal death explosion
local SPELL_MASSIVE_ERUPTION = 20483  -- Forced explosion

-- NPC entries (from molten_core.h)
local NPC_GARR = 12057
local NPC_FIRESWORN = 12099

-- Garr Timer IDs
local TIMER_ANTIMAGICPULSE = 1
local TIMER_MAGMASHACKLES = 2
local TIMER_MASSIVE_ERUPTION = 3

-- Firesworn Timer IDs
local TIMER_FS_IMMOLATE = 1

------------------------------------------------------------------------
-- Garr
------------------------------------------------------------------------
local garr = {}

function garr:OnEnterCombat(input)
    return {
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_ANTIMAGICPULSE, duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_MAGMASHACKLES, duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_MASSIVE_ERUPTION, duration = 360000 },
    }
end

function garr:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Antimagic Pulse (repeats urand(15,20)s)
    if input:IsTimerReady(TIMER_ANTIMAGICPULSE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ANTIMAGICPULSE, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ANTIMAGICPULSE, duration = 17500 })
    end

    -- Magma Shackles (repeats urand(10,15)s)
    if input:IsTimerReady(TIMER_MAGMASHACKLES) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MAGMASHACKLES, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MAGMASHACKLES, duration = 12500 })
    end

    -- After 6 min, force-explode adds every 20s
    if input:IsTimerReady(TIMER_MASSIVE_ERUPTION) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ERUPTION_TRIGGER, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MASSIVE_ERUPTION, duration = 20000 })
    end

    return actions
end

RegisterCreatureAI(12057, garr)

------------------------------------------------------------------------
-- Firesworn (entry 12099)
------------------------------------------------------------------------
local firesworn = {}

function firesworn:OnEnterCombat(input)
    return {
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "CAST_SPELL", spell_id = SPELL_THRASH, target = "self" },
        { action = "SET_TIMER", timer_id = TIMER_FS_IMMOLATE, duration = 10000 },
    }
end

function firesworn:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    if input:IsTimerReady(TIMER_FS_IMMOLATE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_IMMOLATE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FS_IMMOLATE, duration = 20000 })
    end

    return actions
end

function firesworn:OnDeath(input)
    return {
        { action = "CAST_SPELL", spell_id = SPELL_ADD_ERUPTION, target = "self" },
    }
end

RegisterCreatureAI(12099, firesworn)
