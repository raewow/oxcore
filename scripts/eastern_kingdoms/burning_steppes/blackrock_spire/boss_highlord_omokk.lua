--[[
    @script_type: creature_ai
    @entry: 9196
    @name: boss_highlord_omokk

    Boss Highlord Omokk - Lower Blackrock Spire
    Ported from MaNGOS boss_highlord_omokk.cpp
]]

local SPELL_WAR_STOMP    = 24375
local SPELL_STRIKE       = 18368
local SPELL_REND         = 18106
local SPELL_SUNDER_ARMOR = 24317
local SPELL_KNOCK_AWAY   = 20686
local SPELL_SLOW         = 22356

local TIMER_WAR_STOMP    = 1
local TIMER_STRIKE       = 2
local TIMER_REND         = 3
local TIMER_SUNDER_ARMOR = 4
local TIMER_KNOCK_AWAY   = 5
local TIMER_SLOW         = 6

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_WAR_STOMP, duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_STRIKE, duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_REND, duration = 14000 },
        { action = "SET_TIMER", timer_id = TIMER_SUNDER_ARMOR, duration = 2000 },
        { action = "SET_TIMER", timer_id = TIMER_KNOCK_AWAY, duration = 18000 },
        { action = "SET_TIMER", timer_id = TIMER_SLOW, duration = 24000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_WAR_STOMP, duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_STRIKE, duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_REND, duration = 14000 },
        { action = "SET_TIMER", timer_id = TIMER_SUNDER_ARMOR, duration = 2000 },
        { action = "SET_TIMER", timer_id = TIMER_KNOCK_AWAY, duration = 18000 },
        { action = "SET_TIMER", timer_id = TIMER_SLOW, duration = 24000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    if input:IsTimerReady(TIMER_WAR_STOMP) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_WAR_STOMP, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WAR_STOMP, duration = 14000 })
    end

    if input:IsTimerReady(TIMER_STRIKE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_STRIKE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_STRIKE, duration = 10000 })
    end

    if input:IsTimerReady(TIMER_REND) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_REND, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_REND, duration = 18000 })
    end

    if input:IsTimerReady(TIMER_SUNDER_ARMOR) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUNDER_ARMOR, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SUNDER_ARMOR, duration = 25000 })
    end

    if input:IsTimerReady(TIMER_KNOCK_AWAY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_KNOCK_AWAY, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_KNOCK_AWAY, duration = 12000 })
    end

    if input:IsTimerReady(TIMER_SLOW) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SLOW, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SLOW, duration = 18000 })
    end

    return actions
end

RegisterCreatureAI(9196, boss)
