--[[
    @script_type: creature_ai
    @entry: 12258
    @name: boss_celebras

    Celebras the Cursed - Maraudon
    Ported from vmangos boss_celebras_the_cursed.cpp

    Druid-style boss using Wrath, Entangling Roots, and Corrupt Forces.
    After death, becomes Celebras the Redeemed (escort NPC for scepter quest 7046);
    that escort logic requires Phase D/E scripting support.
]]

-- Spells
local SPELL_WRATH            = 21807  -- Nature damage on random target
local SPELL_ENTANGLING_ROOTS = 12747  -- Roots current target in place
local SPELL_CORRUPT_FORCES   = 21968  -- AoE debuff on self (buff for nearby adds)

-- Timer IDs
local TIMER_WRATH            = 1
local TIMER_ENTANGLING_ROOTS = 2
local TIMER_CORRUPT_FORCES   = 3

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_WRATH,            duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_ENTANGLING_ROOTS, duration = 2000 },
        { action = "SET_TIMER", timer_id = TIMER_CORRUPT_FORCES,   duration = 30000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Wrath (random hostile target, repeats every 8s)
    if input:IsTimerReady(TIMER_WRATH) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_WRATH, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WRATH, duration = 8000 })
    end

    -- Entangling Roots (current target, repeats every 20s)
    if input:IsTimerReady(TIMER_ENTANGLING_ROOTS) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ENTANGLING_ROOTS, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ENTANGLING_ROOTS, duration = 20000 })
    end

    -- Corrupt Forces (self-cast AoE buff, repeats every 20s)
    if input:IsTimerReady(TIMER_CORRUPT_FORCES) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CORRUPT_FORCES, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CORRUPT_FORCES, duration = 20000 })
    end

    return actions
end

RegisterCreatureAI(12258, boss)
