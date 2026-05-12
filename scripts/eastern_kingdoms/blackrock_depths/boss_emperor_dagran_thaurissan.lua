--[[
    @script_type: creature_ai
    @entry: 9019
    @name: boss_emperor_dagran_thaurissan

    Emperor Dagran Thaurissan - Blackrock Depths
    Ported from MaNGOS boss_emperor_dagran_thaurissan.cpp

    Also includes Princess Moira Bronzebeard (entry 8929) AI.
]]

-- Emperor spells
local SPELL_HANDOFTHAURISSAN = 17492
local SPELL_AVATAROFFLAME    = 15636

-- Emperor timers
local TIMER_HANDOFTHAURISSAN = 1
local TIMER_AVATAROFFLAME    = 2

local emperor = {}

function emperor:OnEnterCombat(input)
    return {
        { action = "SAY", text = "Come to aid the Throne!" },
        { action = "SET_TIMER", timer_id = TIMER_HANDOFTHAURISSAN, duration = 4000 },
        { action = "SET_TIMER", timer_id = TIMER_AVATAROFFLAME, duration = 25000 },
    }
end

function emperor:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_HANDOFTHAURISSAN, duration = 4000 },
        { action = "SET_TIMER", timer_id = TIMER_AVATAROFFLAME, duration = 25000 },
    }
end

function emperor:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Hand of Thaurissan on random target
    if input:IsTimerReady(TIMER_HANDOFTHAURISSAN) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_HANDOFTHAURISSAN, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_HANDOFTHAURISSAN, duration = 5000 })
    end

    -- Avatar of Flame
    if input:IsTimerReady(TIMER_AVATAROFFLAME) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_AVATAROFFLAME, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_AVATAROFFLAME, duration = 18000 })
    end

    return actions
end

function emperor:OnKill(input)
    return {
        { action = "SAY", text = "I sentence you to death!" },
    }
end

RegisterCreatureAI(9019, emperor)

------------------------------------------------------------------------
-- Princess Moira Bronzebeard
------------------------------------------------------------------------

local SPELL_HEAL           = 15586
local SPELL_MINDBLAST      = 15587
local SPELL_SHADOWWORDPAIN = 15654
local SPELL_SMITE          = 10934

local TIMER_HEAL           = 3
local TIMER_MINDBLAST      = 4
local TIMER_SHADOWWORDPAIN = 5
local TIMER_SMITE          = 6

local moira = {}

function moira:OnEnterCombat(input)
    return {
        { action = "SET_COMBAT_MOVEMENT", enabled = true },
        { action = "SET_TIMER", timer_id = TIMER_HEAL, duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_MINDBLAST, duration = 16000 },
        { action = "SET_TIMER", timer_id = TIMER_SHADOWWORDPAIN, duration = 2000 },
        { action = "SET_TIMER", timer_id = TIMER_SMITE, duration = 8000 },
    }
end

function moira:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_HEAL, duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_MINDBLAST, duration = 16000 },
        { action = "SET_TIMER", timer_id = TIMER_SHADOWWORDPAIN, duration = 2000 },
        { action = "SET_TIMER", timer_id = TIMER_SMITE, duration = 8000 },
    }
end

function moira:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Mind Blast
    if input:IsTimerReady(TIMER_MINDBLAST) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MINDBLAST, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MINDBLAST, duration = 14000 })
    end

    -- Shadow Word: Pain
    if input:IsTimerReady(TIMER_SHADOWWORDPAIN) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOWWORDPAIN, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOWWORDPAIN, duration = 18000 })
    end

    -- Smite
    if input:IsTimerReady(TIMER_SMITE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SMITE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SMITE, duration = 10000 })
    end

    -- Heal (would target Emperor in C++ source, but we target self as fallback)
    if input:IsTimerReady(TIMER_HEAL) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_HEAL, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_HEAL, duration = 10000 })
    end

    return actions
end

RegisterCreatureAI(8929, moira)
