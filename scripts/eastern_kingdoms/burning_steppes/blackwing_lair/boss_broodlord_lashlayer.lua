--[[
    @script_type: creature_ai
    @entry: 12017
    @name: boss_broodlord

    Broodlord Lashlayer - Blackwing Lair
    Verified against vmangos boss_broodlord_lashlayer.cpp
]]

local SPELL_CLEAVE        = 15284  -- was 26350 (wrong)
local SPELL_BLAST_WAVE    = 23331
local SPELL_MORTAL_STRIKE = 24573  -- patch >= 1.8; pre-1.8 uses 23847
local SPELL_KNOCK_AWAY    = 18670  -- was 25778 (wrong)

local TIMER_CLEAVE        = 1
local TIMER_BLAST_WAVE    = 2
local TIMER_MORTAL_STRIKE = 3
local TIMER_KNOCK_AWAY    = 4

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "YELL", text_id = 9967 },
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_CLEAVE,        duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_BLAST_WAVE,    duration = 20000 },
        { action = "SET_TIMER", timer_id = TIMER_MORTAL_STRIKE, duration = 25000 },
        { action = "SET_TIMER", timer_id = TIMER_KNOCK_AWAY,    duration = math.random(20000, 25000) },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_CLEAVE,        duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_BLAST_WAVE,    duration = 20000 },
        { action = "SET_TIMER", timer_id = TIMER_MORTAL_STRIKE, duration = 25000 },
        { action = "SET_TIMER", timer_id = TIMER_KNOCK_AWAY,    duration = math.random(20000, 25000) },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Cleave on current target; repeat every 13-20s
    if input:IsTimerReady(TIMER_CLEAVE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CLEAVE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = math.random(13000, 20000) })
    end

    -- Blast Wave (AoE self); repeat every 20-35s
    if input:IsTimerReady(TIMER_BLAST_WAVE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BLAST_WAVE, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BLAST_WAVE, duration = math.random(20000, 35000) })
    end

    -- Mortal Strike on current target; repeat every 20-30s
    if input:IsTimerReady(TIMER_MORTAL_STRIKE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MORTAL_STRIKE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MORTAL_STRIKE, duration = math.random(20000, 30000) })
    end

    -- Knock Away on current target; also reduces threat by 50% (handled by server)
    -- repeat every 12-25s
    if input:IsTimerReady(TIMER_KNOCK_AWAY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_KNOCK_AWAY, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_KNOCK_AWAY, duration = math.random(12000, 25000) })
    end

    return actions
end

RegisterCreatureAI(12017, boss)
