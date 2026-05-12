--[[
    @script_type: creature_ai
    @entry: 10339
    @name: boss_gyth

    Gyth - Upper Blackrock Spire (Rend Blackhand encounter)
    Ported from MaNGOS boss_gyth.cpp

    Simplified port: The original C++ has a complex wave-spawning event
    where Gyth is invisible and summons waves of dragonkin/orcs before
    revealing himself. That event logic is instance-script driven.
    This script covers the combat phase after Gyth becomes active,
    plus the Rend Blackhand summon at low health.
]]

local SPELL_CORROSIVEACID = 20667
local SPELL_FREEZE        = 16350
local SPELL_FLAMEBREATH   = 20712

local NPC_REND_BLACKHAND  = 10429

local TIMER_CORROSIVEACID = 1
local TIMER_FREEZE        = 2
local TIMER_FLAMEBREATH   = 3

-- Phase constants
local PHASE_COMBAT = 1

local boss = {}

-- Track whether Rend has been summoned (use phase 2)
local PHASE_REND_SUMMONED = 2

function boss:OnEnterCombat(input)
    return {
        { action = "SET_PHASE", phase = PHASE_COMBAT },
        { action = "SET_TIMER", timer_id = TIMER_CORROSIVEACID, duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_FREEZE, duration = 11000 },
        { action = "SET_TIMER", timer_id = TIMER_FLAMEBREATH, duration = 4000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_PHASE", phase = PHASE_COMBAT },
        { action = "SET_TIMER", timer_id = TIMER_CORROSIVEACID, duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_FREEZE, duration = 11000 },
        { action = "SET_TIMER", timer_id = TIMER_FLAMEBREATH, duration = 4000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Corrosive Acid
    if input:IsTimerReady(TIMER_CORROSIVEACID) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CORROSIVEACID, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CORROSIVEACID, duration = 7000 })
    end

    -- Freeze
    if input:IsTimerReady(TIMER_FREEZE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FREEZE, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FREEZE, duration = 16000 })
    end

    -- Flame Breath
    if input:IsTimerReady(TIMER_FLAMEBREATH) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FLAMEBREATH, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FLAMEBREATH, duration = 10500 })
    end

    -- Summon Rend Blackhand at 11% health
    if input.phase == PHASE_COMBAT and input.health_pct < 0.11 then
        table.insert(actions, { action = "INTERRUPT_SPELL" })
        table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_REND_BLACKHAND })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_REND_SUMMONED })
    end

    return actions
end

RegisterCreatureAI(10339, boss)
