--[[
    @script_type: creature_ai
    @entry: 9236
    @name: boss_shadow_hunter_voshgajin

    Boss Shadow Hunter Vosh'gajin - Lower Blackrock Spire
    Ported from MaNGOS boss_shadow_hunter_voshgajin.cpp
]]

local SPELL_CURSE_OF_BLOOD = 24673
local SPELL_HEX            = 16708
local SPELL_CLEAVE         = 20691

local TIMER_CURSE_OF_BLOOD = 1
local TIMER_HEX            = 2
local TIMER_CLEAVE         = 3

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_CURSE_OF_BLOOD, duration = 2000 },
        { action = "SET_TIMER", timer_id = TIMER_HEX, duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = 14000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_CURSE_OF_BLOOD, duration = 2000 },
        { action = "SET_TIMER", timer_id = TIMER_HEX, duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = 14000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Curse of Blood (self-cast AoE)
    if input:IsTimerReady(TIMER_CURSE_OF_BLOOD) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CURSE_OF_BLOOD, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CURSE_OF_BLOOD, duration = 45000 })
    end

    -- Hex on random hostile
    if input:IsTimerReady(TIMER_HEX) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_HEX, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_HEX, duration = 15000 })
    end

    -- Cleave
    if input:IsTimerReady(TIMER_CLEAVE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CLEAVE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = 7000 })
    end

    return actions
end

RegisterCreatureAI(9236, boss)
