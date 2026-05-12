--[[
    @script_type: creature_ai
    @entry: 14020
    @name: boss_chromaggus

    Chromaggus - Blackwing Lair
    Verified against vmangos boss_chromaggus.cpp
]]

-- Vulnerability (Shimmer) spells: 75% damage reduction + 1100% boost to one school
local SPELL_FIRE_VULNERABILITY   = 22277
local SPELL_FROST_VULNERABILITY  = 22278
local SPELL_SHADOW_VULNERABILITY = 22279
local SPELL_NATURE_VULNERABILITY = 22280
local SPELL_ARCANE_VULNERABILITY = 22281

-- Two breath spells randomly chosen per spawn (from 5 possibilities)
local SPELL_INCINERATE     = 23308
local SPELL_TIME_LAPSE     = 23310
local SPELL_CORROSIVE_ACID = 23313
local SPELL_IGNITE_FLESH   = 23315
local SPELL_FROST_BURN     = 23187

-- Brood Afflictions (applied 11-15 times per cast to random targets)
local SPELL_BROODAF_BLUE   = 23153
local SPELL_BROODAF_BLACK  = 23154
local SPELL_BROODAF_RED    = 23155  -- boss heals 150k HP when player w/ red dies
local SPELL_BROODAF_BRONZE = 23170
local SPELL_BROODAF_GREEN  = 23169

local SPELL_FRENZY  = 23128  -- was 28371 (wrong); repeats every 15s
local SPELL_ENRAGE  = 28747  -- triggered once at 20% HP

local TIMER_SHIMMER     = 1
local TIMER_BREATH_ONE  = 2
local TIMER_BREATH_TWO  = 3
local TIMER_AFFLICTION  = 4
local TIMER_FRENZY      = 5

local PHASE_NORMAL  = 1
local PHASE_ENRAGED = 2

local POSSIBLE_BREATHS = {
    SPELL_INCINERATE,
    SPELL_TIME_LAPSE,
    SPELL_CORROSIVE_ACID,
    SPELL_IGNITE_FLESH,
    SPELL_FROST_BURN
}

local VULNERABILITY_SPELLS = {
    SPELL_FIRE_VULNERABILITY,
    SPELL_FROST_VULNERABILITY,
    SPELL_SHADOW_VULNERABILITY,
    SPELL_NATURE_VULNERABILITY,
    SPELL_ARCANE_VULNERABILITY,
}

local AFFLICTION_SPELLS = {
    SPELL_BROODAF_BLUE,
    SPELL_BROODAF_BLACK,
    SPELL_BROODAF_RED,
    SPELL_BROODAF_BRONZE,
    SPELL_BROODAF_GREEN,
}

-- Select two different breaths per spawn (matches vmangos instance-seed logic)
local breath_one = POSSIBLE_BREATHS[math.random(1, 5)]
local breath_two
repeat
    breath_two = POSSIBLE_BREATHS[math.random(1, 5)]
until breath_two ~= breath_one

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
        { action = "SET_COMBAT_WITH_ZONE" },
        -- Shimmer fires immediately on pull (m_uiShimmerTimer = 0 in C++)
        { action = "SET_TIMER", timer_id = TIMER_SHIMMER,     duration = 0 },
        { action = "SET_TIMER", timer_id = TIMER_BREATH_ONE,  duration = 30000 },
        { action = "SET_TIMER", timer_id = TIMER_BREATH_TWO,  duration = 60000 },
        { action = "SET_TIMER", timer_id = TIMER_AFFLICTION,  duration = 7500 },
        { action = "SET_TIMER", timer_id = TIMER_FRENZY,      duration = 15000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
        { action = "SET_TIMER", timer_id = TIMER_SHIMMER,     duration = 0 },
        { action = "SET_TIMER", timer_id = TIMER_BREATH_ONE,  duration = 30000 },
        { action = "SET_TIMER", timer_id = TIMER_BREATH_TWO,  duration = 60000 },
        { action = "SET_TIMER", timer_id = TIMER_AFFLICTION,  duration = 7500 },
        { action = "SET_TIMER", timer_id = TIMER_FRENZY,      duration = 15000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Shimmer: remove old vulnerability, apply new random one; changes every 20s
    if input:IsTimerReady(TIMER_SHIMMER) then
        local vuln = VULNERABILITY_SPELLS[math.random(1, 5)]
        table.insert(actions, { action = "REMOVE_AURAS_BY_TYPE", spell_ids = VULNERABILITY_SPELLS })
        table.insert(actions, { action = "CAST_SPELL", spell_id = vuln, target = "self" })
        table.insert(actions, { action = "EMOTE", emote_id = 9793 })  -- EMOTE_SHIMMER
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHIMMER, duration = 20000 })
    end

    -- Breath One; first at 30s, repeats every 60s
    if input:IsTimerReady(TIMER_BREATH_ONE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = breath_one, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BREATH_ONE, duration = 60000 })
    end

    -- Breath Two; first at 60s, repeats every 60s
    if input:IsTimerReady(TIMER_BREATH_TWO) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = breath_two, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BREATH_TWO, duration = 60000 })
    end

    -- Brood Affliction: apply random affliction to 11-15 random targets; repeats every 7.5s
    -- Note: chromatic mutation (all 5 afflictions) and red-affliction death healing
    -- require server-side aura tracking beyond Lua action system; handled via C++ SpellHit
    if input:IsTimerReady(TIMER_AFFLICTION) then
        local afflict = AFFLICTION_SPELLS[math.random(1, 5)]
        local count = math.random(11, 15)
        for _ = 1, count do
            table.insert(actions, { action = "CAST_SPELL", spell_id = afflict, target = "random_hostile", triggered = true })
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_AFFLICTION, duration = 7500 })
    end

    -- Frenzy (self); emote + cast; repeats every 15s
    if input:IsTimerReady(TIMER_FRENZY) then
        table.insert(actions, { action = "EMOTE", emote_id = 7797 })  -- EMOTE_GENERIC_FRENZY_KILL
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FRENZY, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FRENZY, duration = 15000 })
    end

    -- Enrage once below 20% HP
    if input.phase == PHASE_NORMAL and input.health_pct < 0.20 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ENRAGE, target = "self" })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_ENRAGED })
    end

    return actions
end

RegisterCreatureAI(14020, boss)
