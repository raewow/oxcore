--[[
    @script_type: creature_ai
    @entry: 9237
    @name: boss_warmaster_voone

    Boss War Master Voone - Lower Blackrock Spire
    Ported from MaNGOS boss_warmaster_voone.cpp
    Note: Axe-throwing weapon swap is cosmetic and omitted.
]]

local SPELL_SNAP_KICK     = 15618
local SPELL_CLEAVE        = 15284
local SPELL_UPPERCUT      = 10966
local SPELL_MORTAL_STRIKE = 15708
local SPELL_PUMMEL        = 15615
local SPELL_THROW_AXE     = 16075

local TIMER_SNAP_KICK     = 1
local TIMER_CLEAVE        = 2
local TIMER_UPPERCUT      = 3
local TIMER_MORTAL_STRIKE = 4
local TIMER_PUMMEL        = 5
local TIMER_THROW_AXE     = 6

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_SNAP_KICK, duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = 14000 },
        { action = "SET_TIMER", timer_id = TIMER_UPPERCUT, duration = 20000 },
        { action = "SET_TIMER", timer_id = TIMER_MORTAL_STRIKE, duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_PUMMEL, duration = 32000 },
        { action = "SET_TIMER", timer_id = TIMER_THROW_AXE, duration = 1000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_SNAP_KICK, duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = 14000 },
        { action = "SET_TIMER", timer_id = TIMER_UPPERCUT, duration = 20000 },
        { action = "SET_TIMER", timer_id = TIMER_MORTAL_STRIKE, duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_PUMMEL, duration = 32000 },
        { action = "SET_TIMER", timer_id = TIMER_THROW_AXE, duration = 1000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    if input:IsTimerReady(TIMER_SNAP_KICK) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SNAP_KICK, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SNAP_KICK, duration = 6000 })
    end

    if input:IsTimerReady(TIMER_CLEAVE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CLEAVE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = 12000 })
    end

    if input:IsTimerReady(TIMER_UPPERCUT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_UPPERCUT, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_UPPERCUT, duration = 14000 })
    end

    if input:IsTimerReady(TIMER_MORTAL_STRIKE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MORTAL_STRIKE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MORTAL_STRIKE, duration = 10000 })
    end

    if input:IsTimerReady(TIMER_PUMMEL) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_PUMMEL, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PUMMEL, duration = 16000 })
    end

    if input:IsTimerReady(TIMER_THROW_AXE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_THROW_AXE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_THROW_AXE, duration = 8000 })
    end

    return actions
end

RegisterCreatureAI(9237, boss)
