--[[
    @script_type: creature_ai
    @entry: 11382
    @name: boss_mandokir

    Bloodlord Mandokir - Zul'Gurub
    Verified against vmangos boss_mandokir.cpp

    Notes:
    - Watch: Mandokir watches a random player; if they move/act, he Decapitates them
    - Charge: targets farthest player (8-40 yard range), followed by Fear within 4 yards
    - Level Up: on each kill, Mandokir gains a level via SPELL_LEVEL_UP
    - Ohgan the raptor (entry 14988) must be killed before Mandokir can be killed
    - Chained Spirits (entry 15117) resurrect dead players
    - SPELL_WATCH mechanic requires server-side movement tracking (noted as deferred)
]]

local SPELL_CHARGE        = 24408  -- was 24315 (wrong)
local SPELL_FEAR          = 19134  -- was 29321 (wrong); AoE fear after charge
local SPELL_WHIRLWIND     = 13736  -- was 24236 (wrong)
local SPELL_MORTAL_STRIKE = 16856  -- was 24573 (wrong); on victims <50% HP
local SPELL_ENRAGE        = 24318  -- was 23537 (wrong)
local SPELL_WATCH         = 24314  -- marks random player; decapitate if they move
local SPELL_DECAPITATE    = 24315  -- instant kill on watched player who moved
local SPELL_LEVEL_UP      = 24312  -- cast on kill
local SPELL_OVERPOWER     = 24407  -- proc when victim dodges

-- Ohgan spells (entry 14988 raptor companion)
local SPELL_SUNDERARMOR   = 24317
local SPELL_THRASH        = 3391
local SPELL_EXECUTE       = 7160

local TIMER_WATCH         = 1
local TIMER_CHARGE        = 2
local TIMER_FEAR          = 3
local TIMER_WHIRLWIND     = 4
local TIMER_MORTAL_STRIKE = 5

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SCRIPT_TEXT", text_id = 10446 },  -- SAY_AGGRO
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_WATCH,         duration = 33000 },
        { action = "SET_TIMER", timer_id = TIMER_CHARGE,        duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_FEAR,          duration = 1000 },
        { action = "SET_TIMER", timer_id = TIMER_WHIRLWIND,     duration = 20000 },
        { action = "SET_TIMER", timer_id = TIMER_MORTAL_STRIKE, duration = 1000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_WATCH,         duration = 33000 },
        { action = "SET_TIMER", timer_id = TIMER_CHARGE,        duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_FEAR,          duration = 1000 },
        { action = "SET_TIMER", timer_id = TIMER_WHIRLWIND,     duration = 20000 },
        { action = "SET_TIMER", timer_id = TIMER_MORTAL_STRIKE, duration = 1000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Watch: marks a random player; server checks movement/actions; repeat 20s
    if input:IsTimerReady(TIMER_WATCH) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_WATCH, target = "random_hostile" })
        table.insert(actions, { action = "SCRIPT_TEXT", text_id = 10604 })  -- SAY_WATCH
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WATCH, duration = 20000 })
    end

    -- Charge: targets far player; followed by AoE fear; repeat 30-35s
    if input:IsTimerReady(TIMER_CHARGE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CHARGE, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CHARGE, duration = math.random(30000, 35000) })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FEAR, duration = 100 })
    end

    -- Fear after charge (within 4 yards); repeat 24s normally
    if input:IsTimerReady(TIMER_FEAR) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FEAR, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FEAR, duration = 24000 })
    end

    -- Whirlwind (AoE self); repeat 18s
    if input:IsTimerReady(TIMER_WHIRLWIND) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_WHIRLWIND, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WHIRLWIND, duration = 18000 })
    end

    -- Mortal Strike: only when current victim is below 50% HP; repeat 15s
    if input:IsTimerReady(TIMER_MORTAL_STRIKE) then
        if input.target_health_pct and input.target_health_pct < 0.50 then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MORTAL_STRIKE, target = "current_target" })
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MORTAL_STRIKE, duration = 15000 })
    end

    return actions
end

function boss:OnKilledUnit(input)
    return {
        { action = "CAST_SPELL", spell_id = SPELL_LEVEL_UP, target = "self" },
        { action = "SCRIPT_TEXT", text_id = 10505 },  -- SAY_DING_KILL
    }
end

RegisterCreatureAI(11382, boss)
