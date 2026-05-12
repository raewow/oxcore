--[[
    @script_type: creature_ai
    @entry: 11983
    @name: boss_firemaw

    Firemaw - Blackwing Lair
    Verified against vmangos boss_firemaw.cpp
]]

local SPELL_SHADOW_FLAME = 22539
local SPELL_WING_BUFFET  = 23339  -- reduces hitter threat by 50% (SpellHitTarget in C++)
local SPELL_FLAME_BUFFET = 23341  -- stacking fire debuff; cast every ~1.8-3s

local TIMER_SHADOW_FLAME = 1
local TIMER_WING_BUFFET  = 2
local TIMER_FLAME_BUFFET = 3

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_FLAME, duration = 16000 },
        { action = "SET_TIMER", timer_id = TIMER_WING_BUFFET,  duration = 30000 },
        { action = "SET_TIMER", timer_id = TIMER_FLAME_BUFFET, duration = 2000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_FLAME, duration = 16000 },
        { action = "SET_TIMER", timer_id = TIMER_WING_BUFFET,  duration = 30000 },
        { action = "SET_TIMER", timer_id = TIMER_FLAME_BUFFET, duration = 2000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Shadow Flame (AoE cone self); repeats every 16s
    if input:IsTimerReady(TIMER_SHADOW_FLAME) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_FLAME, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_FLAME, duration = 16000 })
    end

    -- Wing Buffet on current target; reduces threat by 50%; repeats every 30s
    if input:IsTimerReady(TIMER_WING_BUFFET) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_WING_BUFFET, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WING_BUFFET, duration = 30000 })
    end

    -- Flame Buffet (stacking debuff on self = AoE aura); repeats every 1800-3000ms
    if input:IsTimerReady(TIMER_FLAME_BUFFET) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FLAME_BUFFET, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FLAME_BUFFET, duration = math.random(1800, 3000) })
    end

    return actions
end

RegisterCreatureAI(11983, boss)
