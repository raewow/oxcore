--[[
    @script_type: creature_ai
    @entry: 9018
    @name: boss_gerstahn

    Boss High Interrogator Gerstahn - Blackrock Depths
    Ported from MaNGOS boss_high_interrogator_gerstahn.cpp
]]

local SPELL_SHADOW_WORD_PAIN = 14032
local SPELL_MANA_BURN        = 14033
local SPELL_PSYCHIC_SCREAM   = 13704
local SPELL_SHADOW_SHIELD    = 12040

local TIMER_SHADOW_WORD_PAIN = 1
local TIMER_MANA_BURN        = 2
local TIMER_PSYCHIC_SCREAM   = 3
local TIMER_SHADOW_SHIELD    = 4

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_WORD_PAIN, duration = 4000 },
        { action = "SET_TIMER", timer_id = TIMER_MANA_BURN, duration = 14000 },
        { action = "SET_TIMER", timer_id = TIMER_PSYCHIC_SCREAM, duration = 32000 },
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_SHIELD, duration = 8000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_WORD_PAIN, duration = 4000 },
        { action = "SET_TIMER", timer_id = TIMER_MANA_BURN, duration = 14000 },
        { action = "SET_TIMER", timer_id = TIMER_PSYCHIC_SCREAM, duration = 32000 },
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_SHIELD, duration = 8000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Shadow Word: Pain on random hostile
    if input:IsTimerReady(TIMER_SHADOW_WORD_PAIN) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_WORD_PAIN, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_WORD_PAIN, duration = 7000 })
    end

    -- Mana Burn on random hostile
    if input:IsTimerReady(TIMER_MANA_BURN) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MANA_BURN, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MANA_BURN, duration = 10000 })
    end

    -- Psychic Scream (AoE fear, cast on self/target)
    if input:IsTimerReady(TIMER_PSYCHIC_SCREAM) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_PSYCHIC_SCREAM, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PSYCHIC_SCREAM, duration = 30000 })
    end

    -- Shadow Shield (self-cast)
    if input:IsTimerReady(TIMER_SHADOW_SHIELD) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_SHIELD, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_SHIELD, duration = 25000 })
    end

    return actions
end

RegisterCreatureAI(9018, boss)
