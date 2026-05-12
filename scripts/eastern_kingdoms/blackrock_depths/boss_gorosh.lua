--[[
    @script_type: creature_ai
    @entry: 9027
    @name: boss_gorosh

    Boss Gorosh the Dervish - Blackrock Depths (Ring of Law)
    Ported from MaNGOS boss_gorosh_the_dervish.cpp
]]

local SPELL_WHIRLWIND    = 15589
local SPELL_MORTAL_STRIKE = 15708
local SPELL_BLOODLUST    = 21049

local TIMER_WHIRLWIND    = 1
local TIMER_MORTAL_STRIKE = 2
local TIMER_BLOODLUST    = 3

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_WHIRLWIND, duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_MORTAL_STRIKE, duration = 22000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_WHIRLWIND, duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_MORTAL_STRIKE, duration = 22000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Whirlwind (self-cast AoE)
    if input:IsTimerReady(TIMER_WHIRLWIND) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_WHIRLWIND, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WHIRLWIND, duration = 15000 })
    end

    -- Mortal Strike on current target
    if input:IsTimerReady(TIMER_MORTAL_STRIKE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MORTAL_STRIKE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MORTAL_STRIKE, duration = 15000 })
    end

    -- Bloodlust below 51% HP (self-cast buff)
    if input.health_pct < 0.51 and input:IsTimerReady(TIMER_BLOODLUST) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BLOODLUST, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BLOODLUST, duration = 45000 })
    end

    return actions
end

RegisterCreatureAI(9027, boss)
