--[[
    @script_type: creature_ai
    @entry: 11981
    @name: boss_flamegor

    Flamegor - Blackwing Lair
    Verified against vmangos boss_flamegor.cpp
]]

local SPELL_SHADOW_FLAME = 22539
local SPELL_WING_BUFFET  = 23339  -- reduces hitter threat by 50%
local SPELL_FRENZY       = 23342  -- periodically triggers Fire Nova

local TIMER_SHADOW_FLAME = 1
local TIMER_WING_BUFFET  = 2
local TIMER_FRENZY       = 3

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_FLAME, duration = 16000 },
        { action = "SET_TIMER", timer_id = TIMER_WING_BUFFET,  duration = 30000 },
        { action = "SET_TIMER", timer_id = TIMER_FRENZY,       duration = 10000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_FLAME, duration = 16000 },
        { action = "SET_TIMER", timer_id = TIMER_WING_BUFFET,  duration = 30000 },
        { action = "SET_TIMER", timer_id = TIMER_FRENZY,       duration = 10000 },
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

    -- Frenzy (self-buff that periodically triggers Fire Nova); emote + cast; repeats every 10s
    if input:IsTimerReady(TIMER_FRENZY) then
        table.insert(actions, { action = "EMOTE", emote_id = 1191 })  -- EMOTE_GENERIC_FRENZY
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FRENZY, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FRENZY, duration = 10000 })
    end

    return actions
end

RegisterCreatureAI(11981, boss)
