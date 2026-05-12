--[[
    @script_type: creature_ai
    @entry: 10439
    @name: boss_ramstein

    Ramstein the Gorger - Stratholme (Undead, post-Slaughterhouse)
    Ported from vmangos boss_ramstein_the_gorger.cpp

    Uses Trample (AoE melee self-cast) and Knockout (knocks back current target,
    wiping their threat).
]]

-- Spells
local SPELL_TRAMPLE  = 5568   -- AoE melee stomp, self-cast
local SPELL_KNOCKOUT = 17307  -- Knockback + stun on current target, resets threat

-- Timer IDs
local TIMER_TRAMPLE  = 1
local TIMER_KNOCKOUT = 2

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_TRAMPLE,  duration = 3000 },
        { action = "SET_TIMER", timer_id = TIMER_KNOCKOUT, duration = 12000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Trample (AoE self-cast, repeats every 7s)
    if input:IsTimerReady(TIMER_TRAMPLE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TRAMPLE, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TRAMPLE, duration = 7000 })
    end

    -- Knockout (current target, wipes their threat, repeats every 10s)
    if input:IsTimerReady(TIMER_KNOCKOUT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_KNOCKOUT, target = "current_target" })
        table.insert(actions, { action = "MODIFY_THREAT", target = "current_target", amount = -100 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_KNOCKOUT, duration = 10000 })
    end

    return actions
end

RegisterCreatureAI(10439, boss)
