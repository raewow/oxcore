--[[
    @script_type: creature_ai
    @entry: 10436
    @name: boss_baroness_anastari

    Baroness Anastari - Stratholme (Undead)
    Ported from vmangos boss_baroness_anastari.cpp

    Note: Possess mechanic (mind control + invisibility) requires instance-level
    state management not yet available in Lua. This port implements the core spells;
    the full possess sequence requires Phase B instance scripting support.
]]

-- Spells
local SPELL_BANSHEE_WAIL  = 16565  -- Shadow damage on current target
local SPELL_BANSHEE_CURSE = 16867  -- Curse on target (debuff)
local SPELL_SILENCE       = 18327  -- AoE silence, self-cast
local SPELL_POSSESS       = 17244  -- Mind control a player (complex mechanic)

-- Timer IDs
local TIMER_BANSHEE_WAIL  = 1
local TIMER_BANSHEE_CURSE = 2
local TIMER_SILENCE       = 3
local TIMER_POSSESS       = 4

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_BANSHEE_WAIL,  duration = 1000 },
        { action = "SET_TIMER", timer_id = TIMER_BANSHEE_CURSE, duration = 11000 },
        { action = "SET_TIMER", timer_id = TIMER_SILENCE,        duration = 13000 },
        { action = "SET_TIMER", timer_id = TIMER_POSSESS,        duration = 20000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Banshee Wail (current target, repeats every 4s)
    if input:IsTimerReady(TIMER_BANSHEE_WAIL) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BANSHEE_WAIL, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BANSHEE_WAIL, duration = 4000 })
    end

    -- Banshee Curse (current target, repeats every 18s)
    if input:IsTimerReady(TIMER_BANSHEE_CURSE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BANSHEE_CURSE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BANSHEE_CURSE, duration = 18000 })
    end

    -- Silence (self AoE, repeats every 13-18s)
    if input:IsTimerReady(TIMER_SILENCE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SILENCE, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SILENCE, duration = math.random(13000, 18000) })
    end

    -- Possess (random hostile player, repeats every 13-18s after release)
    -- Full possess mechanic (go invisible, immune to damage, control player) requires
    -- instance-level support; casting the spell is handled here as a best-effort approximation
    if input:IsTimerReady(TIMER_POSSESS) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_POSSESS, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_POSSESS, duration = math.random(13000, 18000) })
    end

    return actions
end

function boss:OnDeath(input, killer_guid)
    return {}
end

RegisterCreatureAI(10436, boss)
