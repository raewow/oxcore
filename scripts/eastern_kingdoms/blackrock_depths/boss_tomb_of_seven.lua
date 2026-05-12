--[[
    @script_type: creature_ai
    @entry: 9039
    @name: boss_doomrel

    Doom'rel - Blackrock Depths (Tomb of Seven)
    Ported from MaNGOS boss_tomb_of_seven.cpp

    This is the final boss of the Tomb of Seven event. The wave-spawning
    of the other six dwarves is instance-script driven (gossip interaction
    starts the event, each dwarf turns hostile in sequence).
    This script covers Doom'rel's combat AI only.
]]

local SPELL_SHADOWBOLTVOLLEY  = 15245
local SPELL_IMMOLATE          = 12742
local SPELL_CURSEOFWEAKNESS   = 12493
local SPELL_DEMONARMOR        = 13787
local SPELL_SUMMON_VOIDWALKERS = 15092

local TIMER_SHADOWVOLLEY      = 1
local TIMER_IMMOLATE          = 2
local TIMER_CURSEOFWEAKNESS   = 3
local TIMER_DEMONARMOR        = 4

-- Phase constants
local PHASE_NORMAL    = 1
local PHASE_SUMMONED  = 2

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
        { action = "SET_TIMER", timer_id = TIMER_SHADOWVOLLEY, duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_IMMOLATE, duration = 18000 },
        { action = "SET_TIMER", timer_id = TIMER_CURSEOFWEAKNESS, duration = 5000 },
        { action = "SET_TIMER", timer_id = TIMER_DEMONARMOR, duration = 16000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
        { action = "SET_TIMER", timer_id = TIMER_SHADOWVOLLEY, duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_IMMOLATE, duration = 18000 },
        { action = "SET_TIMER", timer_id = TIMER_CURSEOFWEAKNESS, duration = 5000 },
        { action = "SET_TIMER", timer_id = TIMER_DEMONARMOR, duration = 16000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Shadow Bolt Volley
    if input:IsTimerReady(TIMER_SHADOWVOLLEY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOWBOLTVOLLEY, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOWVOLLEY, duration = 12000 })
    end

    -- Immolate on random target
    if input:IsTimerReady(TIMER_IMMOLATE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_IMMOLATE, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_IMMOLATE, duration = 25000 })
    end

    -- Curse of Weakness
    if input:IsTimerReady(TIMER_CURSEOFWEAKNESS) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CURSEOFWEAKNESS, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CURSEOFWEAKNESS, duration = 45000 })
    end

    -- Demon Armor
    if input:IsTimerReady(TIMER_DEMONARMOR) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DEMONARMOR, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_DEMONARMOR, duration = 300000 })
    end

    -- Summon Voidwalkers at 50% health (one-time)
    if input.phase == PHASE_NORMAL and input.health_pct <= 0.50 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUMMON_VOIDWALKERS, target = "self" })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_SUMMONED })
    end

    return actions
end

RegisterCreatureAI(9039, boss)
