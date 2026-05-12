--[[
    @script_type: creature_ai
    @entry: 4542
    @name: boss_fairbanks

    Boss High Inquisitor Fairbanks - Scarlet Monastery (Cathedral)
    Ported from MaNGOS boss_high_inquisitor_fairbanks.cpp
]]

local SPELL_CURSE_OF_BLOOD    = 8282
local SPELL_DISPEL_MAGIC      = 15090
local SPELL_FEAR              = 12096
local SPELL_HEAL              = 12039
local SPELL_POWER_WORD_SHIELD = 11647
local SPELL_SLEEP             = 8399

local TIMER_CURSE_OF_BLOOD    = 1
local TIMER_DISPEL_MAGIC      = 2
local TIMER_FEAR              = 3
local TIMER_HEAL              = 4
local TIMER_SLEEP             = 5

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_CURSE_OF_BLOOD, duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_DISPEL_MAGIC, duration = 20000 },
        { action = "SET_TIMER", timer_id = TIMER_FEAR, duration = 40000 },
        { action = "SET_TIMER", timer_id = TIMER_HEAL, duration = 30000 },
        { action = "SET_TIMER", timer_id = TIMER_SLEEP, duration = 30000 },
        { action = "SET_CUSTOM_DATA", key = "shielded", value = 0 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_CURSE_OF_BLOOD, duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_DISPEL_MAGIC, duration = 20000 },
        { action = "SET_TIMER", timer_id = TIMER_FEAR, duration = 40000 },
        { action = "SET_TIMER", timer_id = TIMER_HEAL, duration = 30000 },
        { action = "SET_TIMER", timer_id = TIMER_SLEEP, duration = 30000 },
        { action = "SET_CUSTOM_DATA", key = "shielded", value = 0 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Heal below 25% HP
    if input.health_pct <= 0.25 and input:IsTimerReady(TIMER_HEAL) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_HEAL, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_HEAL, duration = 30000 })
    end

    -- Power Word: Shield once below 25% HP
    local shielded = input:GetCustomData("shielded") or 0
    if shielded == 0 and input.health_pct <= 0.25 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_POWER_WORD_SHIELD, target = "self" })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "shielded", value = 1 })
    end

    -- Fear on random hostile
    if input:IsTimerReady(TIMER_FEAR) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FEAR, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FEAR, duration = 40000 })
    end

    -- Sleep on current target
    if input:IsTimerReady(TIMER_SLEEP) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SLEEP, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SLEEP, duration = 30000 })
    end

    -- Dispel Magic on random hostile
    if input:IsTimerReady(TIMER_DISPEL_MAGIC) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DISPEL_MAGIC, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_DISPEL_MAGIC, duration = 30000 })
    end

    -- Curse of Blood
    if input:IsTimerReady(TIMER_CURSE_OF_BLOOD) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CURSE_OF_BLOOD, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CURSE_OF_BLOOD, duration = 25000 })
    end

    return actions
end

RegisterCreatureAI(4542, boss)
