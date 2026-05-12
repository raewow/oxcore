--[[
    @script_type: creature_ai
    @entry: 14601
    @name: boss_ebonroc

    Ebonroc - Blackwing Lair
    Verified against vmangos boss_ebonroc.cpp
]]

local SPELL_SHADOW_FLAME      = 22539
local SPELL_WING_BUFFET       = 23339  -- was 18500 (wrong)
local SPELL_SHADOW_OF_EBONROC = 23340  -- debuff on current target; don't re-apply if present

local TIMER_SHADOW_FLAME      = 1
local TIMER_WING_BUFFET       = 2
local TIMER_SHADOW_OF_EBONROC = 3

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_FLAME,      duration = 16000 },
        { action = "SET_TIMER", timer_id = TIMER_WING_BUFFET,       duration = 30000 },
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_OF_EBONROC, duration = 8000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_FLAME,      duration = 16000 },
        { action = "SET_TIMER", timer_id = TIMER_WING_BUFFET,       duration = 30000 },
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_OF_EBONROC, duration = 8000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Shadow Flame (AoE cone); repeats every 16s
    if input:IsTimerReady(TIMER_SHADOW_FLAME) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_FLAME, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_FLAME, duration = 16000 })
    end

    -- Wing Buffet on current target; reduces threat by 50% (SpellHitTarget in C++); repeats every 30s
    if input:IsTimerReady(TIMER_WING_BUFFET) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_WING_BUFFET, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WING_BUFFET, duration = 30000 })
    end

    -- Shadow of Ebonroc on current target; don't re-apply if aura present; repeats every 8s
    if input:IsTimerReady(TIMER_SHADOW_OF_EBONROC) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_OF_EBONROC, target = "current_target", flags = "no_reapply" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_OF_EBONROC, duration = 8000 })
    end

    return actions
end

RegisterCreatureAI(14601, boss)
