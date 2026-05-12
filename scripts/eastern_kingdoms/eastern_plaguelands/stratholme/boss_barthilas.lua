--[[
    @script_type: creature_ai
    @entry: 10437
    @name: boss_barthilas

    Magistrate Barthilas - Stratholme (Undead side, roaming)
    Ported from vmangos boss_magistrate_barthilas.cpp

    Stacks Furious Anger on himself every 4s (max 25 stacks).
    Uses Draining Blow, Crowd Pummel, and Mighty Blow on the current target.
]]

-- Spells
local SPELL_DRAINING_BLOW = 16793  -- Drains health from target
local SPELL_CROWD_PUMMEL  = 10887  -- AoE stun around caster
local SPELL_MIGHTY_BLOW   = 14099  -- Heavy melee hit on target
local SPELL_FURIOUS_ANGER = 16791  -- Self-buff stacking damage boost (max 25 stacks)

-- Timer IDs
local TIMER_DRAINING_BLOW = 1
local TIMER_CROWD_PUMMEL  = 2
local TIMER_MIGHTY_BLOW   = 3
local TIMER_FURIOUS_ANGER = 4

local boss = {}

function boss:OnEnterCombat(input)
    input.anger_count = 0
    return {
        { action = "SET_TIMER", timer_id = TIMER_DRAINING_BLOW, duration = 16000 },
        { action = "SET_TIMER", timer_id = TIMER_CROWD_PUMMEL,  duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_MIGHTY_BLOW,   duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_FURIOUS_ANGER, duration = 5000 },
    }
end

function boss:OnReset(input)
    input.anger_count = 0
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Furious Anger (self-buff stacking, repeats every 4s, max 25 stacks)
    if input:IsTimerReady(TIMER_FURIOUS_ANGER) then
        if (input.anger_count or 0) < 25 then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FURIOUS_ANGER, target = "self" })
            input.anger_count = (input.anger_count or 0) + 1
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FURIOUS_ANGER, duration = 4000 })
    end

    -- Draining Blow (current target, repeats every 15s)
    if input:IsTimerReady(TIMER_DRAINING_BLOW) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DRAINING_BLOW, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_DRAINING_BLOW, duration = 15000 })
    end

    -- Crowd Pummel (current target, repeats every 15s)
    if input:IsTimerReady(TIMER_CROWD_PUMMEL) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CROWD_PUMMEL, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CROWD_PUMMEL, duration = 15000 })
    end

    -- Mighty Blow (current target, repeats every 20s)
    if input:IsTimerReady(TIMER_MIGHTY_BLOW) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MIGHTY_BLOW, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MIGHTY_BLOW, duration = 20000 })
    end

    return actions
end

RegisterCreatureAI(10437, boss)
