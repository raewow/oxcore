--[[
    @script_type: creature_ai
    @entry: 10400
    @name: boss_rend_blackhand

    Rend Blackhand - Upper Blackrock Spire
    No vmangos C++ reference available; scripted from known 1.12 behavior.

    Rend rides Gyth the dragon during the Warchief encounter. After Gyth is
    killed he dismounts and fights on foot. This script handles the on-foot
    phase only — the mounted phase is part of the Gyth encounter in boss_gyth.lua.

    On foot:
    - Whirlwind: periodic AoE melee spin
    - Strike: periodic heavy melee hit
    - Mortal Strike: periodic heavy strike with healing reduction
    - Frenzy: enrage at ~20% HP (one-time)
]]

local SPELL_WHIRLWIND       = 15576
local SPELL_STRIKE          = 14516
local SPELL_MORTAL_STRIKE   = 16856
local SPELL_FRENZY          = 8269

local TIMER_WHIRLWIND       = 1
local TIMER_STRIKE          = 2
local TIMER_MORTAL_STRIKE   = 3

local PHASE_NORMAL          = 0
local PHASE_ENRAGED         = 1

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
        { action = "YELL", text = "You dare challenge the Warchief?!" },
        { action = "SET_TIMER", timer_id = TIMER_WHIRLWIND,     duration = math.random(8000,  12000) },
        { action = "SET_TIMER", timer_id = TIMER_STRIKE,        duration = math.random(4000,  8000)  },
        { action = "SET_TIMER", timer_id = TIMER_MORTAL_STRIKE, duration = math.random(15000, 20000) },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
    }
end

function boss:OnKill(input)
    return {
        { action = "YELL", text = "None can stand before Rend Blackhand!" },
    }
end

function boss:OnDeath(input)
    return {
        { action = "YELL", text = "The Horde... will... prevail..." },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Frenzy at 20% HP (one-time)
    if input.phase == PHASE_NORMAL and input.health_pct < 0.20 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FRENZY, target = "self" })
        table.insert(actions, { action = "EMOTE", text = "%s goes into a frenzy!" })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_ENRAGED })
    end

    -- Whirlwind
    if input:IsTimerReady(TIMER_WHIRLWIND) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_WHIRLWIND, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WHIRLWIND, duration = math.random(10000, 15000) })
    end

    -- Strike
    if input:IsTimerReady(TIMER_STRIKE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_STRIKE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_STRIKE, duration = math.random(6000, 10000) })
    end

    -- Mortal Strike
    if input:IsTimerReady(TIMER_MORTAL_STRIKE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MORTAL_STRIKE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MORTAL_STRIKE, duration = math.random(15000, 25000) })
    end

    return actions
end

RegisterCreatureAI(10400, boss)
