--[[
    @script_type: creature_ai
    @entry: 9568
    @name: boss_overlord_wyrmthalak

    Boss Overlord Wyrmthalak - Lower Blackrock Spire
    Ported from MaNGOS boss_overlord_wyrmthalak.cpp
]]

local SPELL_BLAST_WAVE = 11130
local SPELL_SHOUT      = 23511
local SPELL_CLEAVE     = 20691
local SPELL_KNOCK_AWAY = 20686

local NPC_SPIRESTONE_WARLORD     = 9216
local NPC_SMOLDERTHORN_BERSERKER = 9268

local TIMER_BLAST_WAVE = 1
local TIMER_SHOUT      = 2
local TIMER_CLEAVE     = 3
local TIMER_KNOCK_AWAY = 4

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_BLAST_WAVE, duration = 20000 },
        { action = "SET_TIMER", timer_id = TIMER_SHOUT, duration = 2000 },
        { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = 6000 },
        { action = "SET_TIMER", timer_id = TIMER_KNOCK_AWAY, duration = 12000 },
        { action = "SET_CUSTOM_DATA", key = "summoned", value = 0 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_BLAST_WAVE, duration = 20000 },
        { action = "SET_TIMER", timer_id = TIMER_SHOUT, duration = 2000 },
        { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = 6000 },
        { action = "SET_TIMER", timer_id = TIMER_KNOCK_AWAY, duration = 12000 },
        { action = "SET_CUSTOM_DATA", key = "summoned", value = 0 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Blast Wave (self-cast AoE)
    if input:IsTimerReady(TIMER_BLAST_WAVE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BLAST_WAVE, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BLAST_WAVE, duration = 20000 })
    end

    -- Shout (self-cast AoE)
    if input:IsTimerReady(TIMER_SHOUT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHOUT, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHOUT, duration = 10000 })
    end

    -- Cleave
    if input:IsTimerReady(TIMER_CLEAVE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CLEAVE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = 7000 })
    end

    -- Knock Away (self-cast AoE)
    if input:IsTimerReady(TIMER_KNOCK_AWAY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_KNOCK_AWAY, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_KNOCK_AWAY, duration = 14000 })
    end

    -- Summon adds at 51% HP (once)
    local summoned = input:GetCustomData("summoned") or 0
    if summoned == 0 and input.health_pct < 0.51 then
        table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_SPIRESTONE_WARLORD, x = -39.355, y = -513.456, z = 88.472, o = 4.680, summon_type = "TIMED_DESPAWN_OUT_OF_COMBAT", duration = 10000 })
        table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_SMOLDERTHORN_BERSERKER, x = -49.876, y = -511.897, z = 88.195, o = 4.613, summon_type = "TIMED_DESPAWN_OUT_OF_COMBAT", duration = 10000 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "summoned", value = 1 })
    end

    return actions
end

RegisterCreatureAI(9568, boss)
