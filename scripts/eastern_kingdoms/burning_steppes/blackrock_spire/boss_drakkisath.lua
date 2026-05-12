--[[
    @script_type: creature_ai
    @entry: 10363
    @name: boss_drakkisath

    General Drakkisath - Upper Blackrock Spire
    Ported from MaNGOS boss_drakkisath.cpp
]]

local SPELL_FIRENOVA       = 23462
local SPELL_CLEAVE         = 20691
local SPELL_CONFLIGURATION = 16805
local SPELL_THUNDERCLAP    = 15548

local TIMER_FIRENOVA       = 1
local TIMER_CLEAVE         = 2
local TIMER_CONFLIGURATION = 3
local TIMER_THUNDERCLAP    = 4

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_FIRENOVA, duration = 6000 },
        { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_CONFLIGURATION, duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_THUNDERCLAP, duration = 17000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_FIRENOVA, duration = 6000 },
        { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_CONFLIGURATION, duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_THUNDERCLAP, duration = 17000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Fire Nova
    if input:IsTimerReady(TIMER_FIRENOVA) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FIRENOVA, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FIRENOVA, duration = 10000 })
    end

    -- Cleave
    if input:IsTimerReady(TIMER_CLEAVE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CLEAVE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = 8000 })
    end

    -- Conflagration
    if input:IsTimerReady(TIMER_CONFLIGURATION) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CONFLIGURATION, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CONFLIGURATION, duration = 18000 })
    end

    -- Thunderclap
    if input:IsTimerReady(TIMER_THUNDERCLAP) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_THUNDERCLAP, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_THUNDERCLAP, duration = 20000 })
    end

    return actions
end

RegisterCreatureAI(10363, boss)
