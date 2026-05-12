--[[
    @script_type: creature_ai
    @entry: 14693
    @name: boss_scorn

    Scorn - Scarlet Monastery (Graveyard, seasonal - Hallow's End)
    No vmangos C++ reference available; scripted from known 1.12 behavior.

    Scorn is a seasonal boss that appears during Hallow's End. He is a large
    skeleton with typical undead abilities.

    - Shadow Bolt: periodic ranged shadow nuke
    - Curse of Agony: periodic DoT on a target
    - Fear: periodic fear
    - Enrage at ~30% HP
]]

local SPELL_SHADOW_BOLT     = 9613
local SPELL_CURSE_OF_AGONY  = 17314
local SPELL_FEAR            = 26070

local TIMER_SHADOW_BOLT     = 1
local TIMER_CURSE_OF_AGONY  = 2
local TIMER_FEAR            = 3

local PHASE_NORMAL          = 0
local PHASE_ENRAGED         = 1

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_BOLT,    duration = math.random(2000, 6000)  },
        { action = "SET_TIMER", timer_id = TIMER_CURSE_OF_AGONY, duration = math.random(8000, 12000) },
        { action = "SET_TIMER", timer_id = TIMER_FEAR,           duration = math.random(15000, 20000) },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Enrage at 30% HP (one-time)
    if input.phase == PHASE_NORMAL and input.health_pct < 0.30 then
        table.insert(actions, { action = "EMOTE", text = "%s becomes enraged!" })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_ENRAGED })
    end

    -- Shadow Bolt
    if input:IsTimerReady(TIMER_SHADOW_BOLT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_BOLT, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_BOLT, duration = math.random(4000, 8000) })
    end

    -- Curse of Agony
    if input:IsTimerReady(TIMER_CURSE_OF_AGONY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CURSE_OF_AGONY, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CURSE_OF_AGONY, duration = math.random(10000, 15000) })
    end

    -- Fear
    if input:IsTimerReady(TIMER_FEAR) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FEAR, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FEAR, duration = math.random(20000, 30000) })
    end

    return actions
end

RegisterCreatureAI(14693, boss)
