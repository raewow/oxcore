--[[
    @script_type: creature_ai
    @entry: 12397
    @name: boss_lord_kazzak

    Lord Kazzak - Blasted Lands (World Boss)
    No vmangos C++ reference available; verified against MaNGOS boss_kruul.cpp

    Demon world boss. Enrages after 3 minutes (Supreme mode).
    Heals to full when any nearby player dies (Capture Soul).
    Marks random mana users with Mark of Kazzak (mana drain + AoE on expire).
]]

-- Spell constants
local SPELL_SHADOW_VOLLEY        = 21341
local SPELL_CLEAVE               = 20691
local SPELL_THUNDERCLAP          = 26554
local SPELL_VOID_BOLT            = 21066
local SPELL_MARK_OF_KAZZAK       = 21056
local SPELL_CAPTURE_SOUL         = 21053
local SPELL_TWISTED_REFLECTION   = 21063
local SPELL_BERSERK              = 21340

-- Timer IDs
local TIMER_SHADOW_VOLLEY        = 1
local TIMER_CLEAVE               = 2
local TIMER_THUNDERCLAP          = 3
local TIMER_VOID_BOLT            = 4
local TIMER_MARK_OF_KAZZAK       = 5
local TIMER_TWISTED_REFLECTION   = 6
local TIMER_BERSERK              = 7

-- Phase constants
local PHASE_NORMAL               = 0
local PHASE_BERSERK              = 1

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
        { action = "YELL", text = "All mortals will perish!" },
        { action = "CAST_SPELL", spell_id = SPELL_CAPTURE_SOUL, target = "self" },
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_VOLLEY, duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = 7000 },
        { action = "SET_TIMER", timer_id = TIMER_THUNDERCLAP, duration = 18000 },
        { action = "SET_TIMER", timer_id = TIMER_VOID_BOLT, duration = 30000 },
        { action = "SET_TIMER", timer_id = TIMER_MARK_OF_KAZZAK, duration = 25000 },
        { action = "SET_TIMER", timer_id = TIMER_TWISTED_REFLECTION, duration = 33000 },
        { action = "SET_TIMER", timer_id = TIMER_BERSERK, duration = 180000 },
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

    -- Berserk (Supreme) after 3 minutes
    if input.phase == PHASE_NORMAL and input:IsTimerReady(TIMER_BERSERK) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BERSERK, target = "self" })
        table.insert(actions, { action = "YELL", text = "The Legion will conquer all!" })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_BERSERK })
    end

    -- Shadow Volley (AoE shadow bolt, faster in berserk)
    if input:IsTimerReady(TIMER_SHADOW_VOLLEY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_VOLLEY, target = "self" })
        if input.phase == PHASE_BERSERK then
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_VOLLEY, duration = 2000 })
        else
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_VOLLEY, duration = 17000 })
        end
    end

    -- Cleave
    if input:IsTimerReady(TIMER_CLEAVE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CLEAVE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = 10000 })
    end

    -- Thunderclap
    if input:IsTimerReady(TIMER_THUNDERCLAP) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_THUNDERCLAP, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_THUNDERCLAP, duration = 12000 })
    end

    -- Void Bolt (single target nuke)
    if input:IsTimerReady(TIMER_VOID_BOLT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_VOID_BOLT, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_VOID_BOLT, duration = 21000 })
    end

    -- Mark of Kazzak (on random mana user - drains mana, AoE on expire)
    if input:IsTimerReady(TIMER_MARK_OF_KAZZAK) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MARK_OF_KAZZAK, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MARK_OF_KAZZAK, duration = 20000 })
    end

    -- Twisted Reflection (heals Kazzak when target takes damage)
    if input:IsTimerReady(TIMER_TWISTED_REFLECTION) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TWISTED_REFLECTION, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TWISTED_REFLECTION, duration = 15000 })
    end

    return actions
end

RegisterCreatureAI(12397, boss)
