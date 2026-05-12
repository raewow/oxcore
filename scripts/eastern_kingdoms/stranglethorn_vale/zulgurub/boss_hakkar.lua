--[[
    @script_type: creature_ai
    @entry: 14834
    @name: boss_hakkar

    Hakkar the Soulflayer - Zul'Gurub
    Verified against vmangos boss_hakkar.cpp

    Notes:
    - Blood Siphon mechanic (stun raid then heal) fires every 90s
    - Cause Insanity redirects victim to attack raid (requires MC API; approximated)
    - Will of Hakkar is a mind control-like effect
    - Aspects fire only if their corresponding High Priest is alive (instance script tracks this)
    - Berserk at 10 minutes
]]

local SPELL_BLOODSIPHON_STUN   = 24324  -- stuns all players, then heals Hakkar via 24322
local SPELL_CORRUPTEDBLOOD     = 24328  -- DoT on random target; repeat 14-16s
local SPELL_CAUSEINSANITY      = 24327  -- MC effect on current victim; repeat 20-25s
local SPELL_WILLOFHAKKAR       = 24178  -- MC on random target
local SPELL_BERSERK            = 27680  -- enrage at 10 minutes

-- Aspect spells (only if high priest for that aspect is alive)
local SPELL_ASPECT_OF_JEKLIK   = 24687  -- silence 4s; repeat 10-14s
local SPELL_ASPECT_OF_VENOXIS  = 24688  -- poison; repeat 8s
local SPELL_ASPECT_OF_MARLI    = 24686  -- stun 5s; repeat 10s
local SPELL_ASPECT_OF_THEKAL   = 24689  -- enrage; repeat 15s
local SPELL_ASPECT_OF_ARLOKK   = 24690  -- vanish; repeat 10-15s

local TIMER_BLOODSIPHON    = 1
local TIMER_CORRUPTEDBLOOD = 2
local TIMER_CAUSEINSANITY  = 3
local TIMER_WILLOFHAKKAR   = 4
local TIMER_BERSERK        = 5
local TIMER_JEKLIK         = 6
local TIMER_VENOXIS        = 7
local TIMER_MARLI          = 8
local TIMER_THEKAL         = 9
local TIMER_ARLOKK         = 10

local PHASE_NORMAL  = 1
local PHASE_BERSERK = 2

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SCRIPT_TEXT", text_id = 10447 },  -- SAY_AGGRO
        { action = "SET_PHASE", phase = PHASE_NORMAL },
        { action = "SET_TIMER", timer_id = TIMER_BLOODSIPHON,    duration = 90000 },
        { action = "SET_TIMER", timer_id = TIMER_CORRUPTEDBLOOD, duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_CAUSEINSANITY,  duration = 17000 },
        { action = "SET_TIMER", timer_id = TIMER_WILLOFHAKKAR,   duration = 17000 },
        { action = "SET_TIMER", timer_id = TIMER_BERSERK,        duration = 600000 },
        { action = "SET_TIMER", timer_id = TIMER_JEKLIK,         duration = 4000 },
        { action = "SET_TIMER", timer_id = TIMER_VENOXIS,        duration = 7000 },
        { action = "SET_TIMER", timer_id = TIMER_MARLI,          duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_THEKAL,         duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_ARLOKK,         duration = 18000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
        { action = "SET_TIMER", timer_id = TIMER_BLOODSIPHON,    duration = 90000 },
        { action = "SET_TIMER", timer_id = TIMER_CORRUPTEDBLOOD, duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_CAUSEINSANITY,  duration = 17000 },
        { action = "SET_TIMER", timer_id = TIMER_WILLOFHAKKAR,   duration = 17000 },
        { action = "SET_TIMER", timer_id = TIMER_BERSERK,        duration = 600000 },
        { action = "SET_TIMER", timer_id = TIMER_JEKLIK,         duration = 4000 },
        { action = "SET_TIMER", timer_id = TIMER_VENOXIS,        duration = 7000 },
        { action = "SET_TIMER", timer_id = TIMER_MARLI,          duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_THEKAL,         duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_ARLOKK,         duration = 18000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Blood Siphon: stun all, then Hakkar heals; every 90s
    if input:IsTimerReady(TIMER_BLOODSIPHON) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BLOODSIPHON_STUN, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BLOODSIPHON, duration = 90000 })
    end

    -- Corrupted Blood on random target; repeat 14-16s
    if input:IsTimerReady(TIMER_CORRUPTEDBLOOD) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CORRUPTEDBLOOD, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CORRUPTEDBLOOD, duration = math.random(14000, 16000) })
    end

    -- Cause Insanity on current victim; repeat 20-25s
    if input:IsTimerReady(TIMER_CAUSEINSANITY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CAUSEINSANITY, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CAUSEINSANITY, duration = math.random(20000, 25000) })
    end

    -- Will of Hakkar on random target; repeat 30s (approximate)
    if input:IsTimerReady(TIMER_WILLOFHAKKAR) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_WILLOFHAKKAR, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WILLOFHAKKAR, duration = 30000 })
    end

    -- Berserk at 10 minutes; permanent after
    if input:IsTimerReady(TIMER_BERSERK) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BERSERK, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BERSERK, duration = 2000 })
    end

    -- Aspects (active if corresponding high priest is dead = power is available to Hakkar)
    if input:IsTimerReady(TIMER_JEKLIK) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ASPECT_OF_JEKLIK, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_JEKLIK, duration = math.random(10000, 14000) })
    end

    if input:IsTimerReady(TIMER_VENOXIS) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ASPECT_OF_VENOXIS, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_VENOXIS, duration = 8000 })
    end

    if input:IsTimerReady(TIMER_MARLI) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ASPECT_OF_MARLI, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MARLI, duration = 10000 })
    end

    if input:IsTimerReady(TIMER_THEKAL) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ASPECT_OF_THEKAL, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_THEKAL, duration = 15000 })
    end

    if input:IsTimerReady(TIMER_ARLOKK) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ASPECT_OF_ARLOKK, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ARLOKK, duration = math.random(10000, 15000) })
    end

    return actions
end

RegisterCreatureAI(14834, boss)
