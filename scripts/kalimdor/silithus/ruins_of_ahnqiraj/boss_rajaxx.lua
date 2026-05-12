--[[
    @script_type: creature_ai
    @entry: 15341
    @name: boss_rajaxx

    General Rajaxx - Ruins of Ahn'Qiraj
    Verified against vmangos ruins_of_ahnqiraj.cpp

    Note: vmangos has no dedicated boss AI for Rajaxx (entry 15341) — only a spell
    script for Thunder Crash damage calculation. This script provides basic combat AI
    with Thunder Crash and Disarm based on the encounter design.
]]

local SPELL_THUNDER_CRASH   = 25599
local SPELL_DISARM          = 6713

local TIMER_THUNDER_CRASH   = 1
local TIMER_DISARM          = 2

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "YELL", text = "The time of our retribution is at hand! Let darkness reign in the hearts of our enemies!" },
        { action = "SET_TIMER", timer_id = TIMER_THUNDER_CRASH, duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_DISARM, duration = 8000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Thunder Crash
    if input:IsTimerReady(TIMER_THUNDER_CRASH) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_THUNDER_CRASH, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_THUNDER_CRASH, duration = 12000 })
    end

    -- Disarm
    if input:IsTimerReady(TIMER_DISARM) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DISARM, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_DISARM, duration = 15000 })
    end

    return actions
end

RegisterCreatureAI(15341, boss)
