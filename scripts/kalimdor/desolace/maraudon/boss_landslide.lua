--[[
    @script_type: creature_ai
    @entry: 12203
    @name: boss_landslide

    Boss Landslide - Maraudon
    Ported from MaNGOS boss_landslide.cpp
]]

local SPELL_KNOCK_AWAY = 18670
local SPELL_TRAMPLE    = 5568
local SPELL_LANDSLIDE  = 21808

local TIMER_KNOCK_AWAY = 1
local TIMER_TRAMPLE    = 2
local TIMER_LANDSLIDE  = 3

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_KNOCK_AWAY, duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_TRAMPLE, duration = 2000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_KNOCK_AWAY, duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_TRAMPLE, duration = 2000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Knock Away
    if input:IsTimerReady(TIMER_KNOCK_AWAY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_KNOCK_AWAY, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_KNOCK_AWAY, duration = 15000 })
    end

    -- Trample (self-cast AoE)
    if input:IsTimerReady(TIMER_TRAMPLE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TRAMPLE, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TRAMPLE, duration = 8000 })
    end

    -- Landslide below 50% HP
    if input.health_pct < 0.50 and input:IsTimerReady(TIMER_LANDSLIDE) then
        table.insert(actions, { action = "INTERRUPT_SPELL" })
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_LANDSLIDE, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_LANDSLIDE, duration = 60000 })
    end

    return actions
end

RegisterCreatureAI(12203, boss)
