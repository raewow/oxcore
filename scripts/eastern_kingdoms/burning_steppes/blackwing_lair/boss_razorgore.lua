--[[
    @script_type: creature_ai
    @entry: 12435
    @name: boss_razorgore

    Razorgore the Untamed - Blackwing Lair (Phase 2: after all eggs destroyed)
    Verified against vmangos boss_razorgore.cpp

    Phase 1 (egg room / mind control via Orb of Domination) is handled by
    the instance script and trigger_orb_of_command AI (C++ only; too complex
    for the Lua action system without SpawnCreature + possession APIs).

    This script covers Phase 2 (Razorgore breaks free and attacks the raid).
]]

local SPELL_CLEAVE          = 19632
local SPELL_WARSTOMP        = 24375  -- AoE stun; reduces threat by 30% on hit (SpellHitTarget)
local SPELL_FIREBALL_VOLLEY = 22425
local SPELL_CONFLAGRATION   = 23023

local TIMER_CLEAVE          = 1
local TIMER_WARSTOMP        = 2
local TIMER_FIREBALL_VOLLEY = 3
local TIMER_CONFLAGRATION   = 4

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_CLEAVE,          duration = 9000 },
        { action = "SET_TIMER", timer_id = TIMER_WARSTOMP,        duration = 22000 },
        { action = "SET_TIMER", timer_id = TIMER_FIREBALL_VOLLEY, duration = 7000 },
        { action = "SET_TIMER", timer_id = TIMER_CONFLAGRATION,   duration = 12000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_CLEAVE,          duration = 9000 },
        { action = "SET_TIMER", timer_id = TIMER_WARSTOMP,        duration = 22000 },
        { action = "SET_TIMER", timer_id = TIMER_FIREBALL_VOLLEY, duration = 7000 },
        { action = "SET_TIMER", timer_id = TIMER_CONFLAGRATION,   duration = 12000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Cleave on current target; repeat every 5-10s
    if input:IsTimerReady(TIMER_CLEAVE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CLEAVE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = math.random(5000, 10000) })
    end

    -- War Stomp (AoE self); reduces threat by 30% on hit; repeat every 25-45s
    if input:IsTimerReady(TIMER_WARSTOMP) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_WARSTOMP, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WARSTOMP, duration = math.random(25000, 45000) })
    end

    -- Fireball Volley (AoE self); repeat every 15-20s
    if input:IsTimerReady(TIMER_FIREBALL_VOLLEY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FIREBALL_VOLLEY, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FIREBALL_VOLLEY, duration = math.random(15000, 20000) })
    end

    -- Conflagration (AoE self); repeat every 15-25s
    if input:IsTimerReady(TIMER_CONFLAGRATION) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CONFLAGRATION, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CONFLAGRATION, duration = math.random(15000, 25000) })
    end

    return actions
end

RegisterCreatureAI(12435, boss)
