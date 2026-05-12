--[[
    @script_type: creature_ai
    @entry: 11988
    @name: boss_golemagg

    Golemagg the Incinerator - Molten Core
    Ported from vmangos boss_golemagg.cpp

    Also includes Core Rager add AI (entry 11672).
]]

-- Golemagg Spells (exact from vmangos)
local SPELL_PYROBLAST = 20228
local SPELL_EARTHQUAKE = 19798
local SPELL_GOLEMAGG_TRUST = 20553   -- Self-buff, buffs nearby Core Ragers
local SPELL_ATTRACK_RAGER = 20544    -- Cast at 10% HP

-- Core Rager Spells
local SPELL_MANGLE = 19820
local SPELL_FULL_HEAL = 17683        -- Heal to full at 50% HP if Golemagg alive

-- NPC entries
local NPC_CORE_RAGER = 11672

-- Golemagg Timer IDs
local TIMER_PYROBLAST = 1
local TIMER_GOLEMAGG_TRUST = 2
local TIMER_EARTHQUAKE = 3

-- Core Rager Timer IDs
local TIMER_MANGLE = 1

-- Emote text IDs
local EMOTE_LOWHP = 7865

------------------------------------------------------------------------
-- Golemagg
------------------------------------------------------------------------
local golemagg = {}
local enrage_triggered = false

function golemagg:OnEnterCombat(input)
    enrage_triggered = false
    return {
        { action = "SET_TIMER", timer_id = TIMER_PYROBLAST, duration = 7000 },
        { action = "SET_TIMER", timer_id = TIMER_GOLEMAGG_TRUST, duration = 10000 },
    }
end

function golemagg:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Enrage at 10% HP: cast Attrack Rager + start Earthquake
    if not enrage_triggered and input.health_pct < 0.10 then
        enrage_triggered = true
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ATTRACK_RAGER, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EARTHQUAKE, duration = 5000 })
    end

    -- Pyroblast on random target (repeats every 7s)
    if input:IsTimerReady(TIMER_PYROBLAST) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_PYROBLAST, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PYROBLAST, duration = 7000 })
    end

    -- Golemagg's Trust self-buff (repeats every 2s)
    if input:IsTimerReady(TIMER_GOLEMAGG_TRUST) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_GOLEMAGG_TRUST, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_GOLEMAGG_TRUST, duration = 2000 })
    end

    -- Earthquake (only in enrage phase, repeats every 5s)
    if enrage_triggered and input:IsTimerReady(TIMER_EARTHQUAKE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_EARTHQUAKE, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EARTHQUAKE, duration = 5000 })
    end

    return actions
end

RegisterCreatureAI(11988, golemagg)

------------------------------------------------------------------------
-- Core Rager (entry 11672)
------------------------------------------------------------------------
local core_rager = {}

function core_rager:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_MANGLE, duration = 7000 },
    }
end

function core_rager:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Mangle (repeats every 10s)
    if input:IsTimerReady(TIMER_MANGLE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MANGLE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MANGLE, duration = 10000 })
    end

    -- Full Heal at 50% HP (if Golemagg is alive - simplified, always heals)
    if input.health_pct < 0.50 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FULL_HEAL, target = "self" })
        table.insert(actions, { action = "PLAY_EMOTE", emote_id = EMOTE_LOWHP })
    end

    return actions
end

RegisterCreatureAI(11672, core_rager)
