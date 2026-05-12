--[[
    @script_type: creature_ai
    @entry: 6487
    @name: boss_arcanist_doan

    Boss Arcanist Doan - Scarlet Monastery (Library)
    Ported from MaNGOS boss_arcanist_doan.cpp
]]

local SPELL_POLYMORPH       = 13323
local SPELL_AOE_SILENCE     = 8988
local SPELL_ARCANE_EXPLOSION = 9433
local SPELL_FIRE_AOE        = 9435
local SPELL_ARCANE_BUBBLE   = 9438

local TIMER_POLYMORPH       = 1
local TIMER_AOE_SILENCE     = 2
local TIMER_ARCANE_EXPLOSION = 3

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SAY", text = "You will not defile these mysteries!" },
        { action = "SET_TIMER", timer_id = TIMER_POLYMORPH, duration = 20000 },
        { action = "SET_TIMER", timer_id = TIMER_AOE_SILENCE, duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_ARCANE_EXPLOSION, duration = 3000 },
        { action = "SET_CUSTOM_DATA", key = "shielded", value = 0 },
        { action = "SET_CUSTOM_DATA", key = "detonated", value = 0 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_POLYMORPH, duration = 20000 },
        { action = "SET_TIMER", timer_id = TIMER_AOE_SILENCE, duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_ARCANE_EXPLOSION, duration = 3000 },
        { action = "SET_CUSTOM_DATA", key = "shielded", value = 0 },
        { action = "SET_CUSTOM_DATA", key = "detonated", value = 0 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    local shielded = input:GetCustomData("shielded") or 0
    local detonated = input:GetCustomData("detonated") or 0

    -- Detonate after bubble
    if shielded == 1 and detonated == 0 then
        table.insert(actions, { action = "SAY", text = "Burn in righteous fire!" })
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FIRE_AOE, target = "self" })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "detonated", value = 1 })
        return actions
    end

    -- Arcane Bubble below 50% HP (one-time)
    if shielded == 0 and input.health_pct <= 0.50 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ARCANE_BUBBLE, target = "self" })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "shielded", value = 1 })
        return actions
    end

    -- Polymorph on random hostile
    if input:IsTimerReady(TIMER_POLYMORPH) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_POLYMORPH, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_POLYMORPH, duration = 20000 })
    end

    -- AoE Silence
    if input:IsTimerReady(TIMER_AOE_SILENCE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_AOE_SILENCE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_AOE_SILENCE, duration = math.random(15000, 20000) })
    end

    -- Arcane Explosion
    if input:IsTimerReady(TIMER_ARCANE_EXPLOSION) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ARCANE_EXPLOSION, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ARCANE_EXPLOSION, duration = 8000 })
    end

    return actions
end

RegisterCreatureAI(6487, boss)
