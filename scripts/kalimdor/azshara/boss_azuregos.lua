--[[
    @script_type: creature_ai
    @entry: 6109
    @name: boss_azuregos

    Azuregos - Azshara (World Boss)
    No vmangos C++ reference available; verified against MaNGOS boss_azuregos.cpp

    Blue dragon world boss. Uses arcane/frost spells, teleports players,
    reflects magic, and marks killed players with Mark of Frost.
]]

-- Spell constants
local SPELL_ARCANE_VACUUM        = 21147
local SPELL_MARK_OF_FROST_AURA   = 23184
local SPELL_MANA_STORM           = 21097
local SPELL_CHILL                = 21098
local SPELL_FROST_BREATH         = 21099
local SPELL_REFLECT              = 22067
local SPELL_CLEAVE               = 19983
local SPELL_ENRAGE               = 23537

-- Timer IDs
local TIMER_MANA_STORM           = 1
local TIMER_CHILL                = 2
local TIMER_FROST_BREATH         = 3
local TIMER_TELEPORT             = 4
local TIMER_REFLECT              = 5
local TIMER_CLEAVE               = 6
local TIMER_ENRAGE_CHECK         = 7

-- Phase constants
local PHASE_NORMAL               = 0
local PHASE_ENRAGED              = 1

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
        { action = "CAST_SPELL", spell_id = SPELL_MARK_OF_FROST_AURA, target = "self" },
        { action = "SET_TIMER", timer_id = TIMER_MANA_STORM, duration = 11000 },
        { action = "SET_TIMER", timer_id = TIMER_CHILL, duration = 20000 },
        { action = "SET_TIMER", timer_id = TIMER_FROST_BREATH, duration = 5000 },
        { action = "SET_TIMER", timer_id = TIMER_TELEPORT, duration = 30000 },
        { action = "SET_TIMER", timer_id = TIMER_REFLECT, duration = 22000 },
        { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = 7000 },
        { action = "SET_TIMER", timer_id = TIMER_ENRAGE_CHECK, duration = 2000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Enrage at 26% HP
    if input.phase == PHASE_NORMAL and input.health_pct < 0.26 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ENRAGE, target = "self" })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_ENRAGED })
    end

    -- Mana Storm (AoE mana drain)
    if input:IsTimerReady(TIMER_MANA_STORM) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MANA_STORM, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MANA_STORM, duration = 26000 })
    end

    -- Chill (AoE frost damage + slow)
    if input:IsTimerReady(TIMER_CHILL) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CHILL, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CHILL, duration = 19000 })
    end

    -- Frost Breath (cone)
    if input:IsTimerReady(TIMER_FROST_BREATH) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FROST_BREATH, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FROST_BREATH, duration = 17000 })
    end

    -- Arcane Vacuum (teleport all players to self)
    if input:IsTimerReady(TIMER_TELEPORT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ARCANE_VACUUM, target = "self" })
        table.insert(actions, { action = "YELL", text = "Come, little ones. Face me!" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TELEPORT, duration = 25000 })
    end

    -- Reflect Magic
    if input:IsTimerReady(TIMER_REFLECT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_REFLECT, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_REFLECT, duration = 27000 })
    end

    -- Cleave
    if input:IsTimerReady(TIMER_CLEAVE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CLEAVE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = 7000 })
    end

    return actions
end

RegisterCreatureAI(6109, boss)
