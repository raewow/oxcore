--[[
    @script_type: creature_ai
    @entry: 12259
    @name: boss_gehennas

    Gehennas - Molten Core
    Ported from vmangos boss_gehennas.cpp
]]

-- Spells (exact from vmangos)
local SPELL_GEHENNAS_CURSE = 19716       -- Reduces healing received by 75%
local SPELL_RAIN_OF_FIRE = 19717         -- AoE fire damage on random
local SPELL_SHADOW_BOLT_RANDOM = 19728   -- Shadow bolt on random
local SPELL_SHADOW_BOLT_TARGET = 19729   -- Shadow bolt on current target

-- Timer IDs
local TIMER_CURSE = 1
local TIMER_RAIN_OF_FIRE = 2
local TIMER_SHADOW_BOLT_RANDOM = 3
local TIMER_SHADOW_BOLT_TARGET = 4

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_CURSE, duration = 7500 },             -- urand(5000,10000)
        { action = "SET_TIMER", timer_id = TIMER_RAIN_OF_FIRE, duration = 9000 },      -- urand(6000,12000)
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_BOLT_RANDOM, duration = 4500 },-- urand(3000,6000)
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_BOLT_TARGET, duration = 4500 },-- urand(3000,6000)
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Rain of Fire (random target, repeats urand(6000,12000) -> 9000)
    if input:IsTimerReady(TIMER_RAIN_OF_FIRE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_RAIN_OF_FIRE, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_RAIN_OF_FIRE, duration = 9000 })
    end

    -- Gehennas' Curse (self-cast AoE, repeats urand(25000,30000) -> 27500)
    if input:IsTimerReady(TIMER_CURSE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_GEHENNAS_CURSE, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CURSE, duration = 27500 })
    end

    -- Shadow Bolt random (repeats urand(3000,6000) -> 4500)
    if input:IsTimerReady(TIMER_SHADOW_BOLT_RANDOM) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_BOLT_RANDOM, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_BOLT_RANDOM, duration = 4500 })
    end

    -- Shadow Bolt on current target (repeats urand(3000,6000) -> 4500)
    if input:IsTimerReady(TIMER_SHADOW_BOLT_TARGET) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_BOLT_TARGET, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_BOLT_TARGET, duration = 4500 })
    end

    return actions
end

RegisterCreatureAI(12259, boss)
