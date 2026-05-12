--[[
    @script_type: creature_ai
    @entry: 15085
    @name: boss_wushoolay

    Wushoolay - Zul'Gurub
    Verified against vmangos boss_wushoolay.cpp
]]

local SPELL_LIGHTNINGCLOUD = 25033
local SPELL_LIGHTNINGWAVE  = 24819

local TIMER_LIGHTNINGCLOUD = 1
local TIMER_LIGHTNINGWAVE  = 2

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_LIGHTNINGCLOUD, duration = math.random(5000, 10000) },
        { action = "SET_TIMER", timer_id = TIMER_LIGHTNINGWAVE,  duration = math.random(8000, 16000) },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_LIGHTNINGCLOUD, duration = math.random(5000, 10000) },
        { action = "SET_TIMER", timer_id = TIMER_LIGHTNINGWAVE,  duration = math.random(8000, 16000) },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    if input:IsTimerReady(TIMER_LIGHTNINGCLOUD) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_LIGHTNINGCLOUD, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_LIGHTNINGCLOUD, duration = math.random(15000, 20000) })
    end

    if input:IsTimerReady(TIMER_LIGHTNINGWAVE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_LIGHTNINGWAVE, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_LIGHTNINGWAVE, duration = math.random(12000, 16000) })
    end

    return actions
end

RegisterCreatureAI(15085, boss)
