--[[
    @script_type: creature_ai
    @entry: 4543
    @name: boss_bloodmage_thalnos

    Bloodmage Thalnos - Scarlet Monastery (Graveyard)
    No vmangos C++ reference available; scripted from known 1.12 behavior.

    An undead Scarlet Crusade mage. Casts Shadow Bolt, Flame Spike,
    and summons Skeleton adds on death.
    Enrages at low health.
]]

local SPELL_SHADOW_BOLT     = 9613
local SPELL_FLAME_SPIKE     = 8814
local SPELL_BLOOD_PACT      = 8256

local NPC_SKELETON          = 4542

local TIMER_SHADOW_BOLT     = 1
local TIMER_FLAME_SPIKE     = 2

local PHASE_NORMAL          = 0
local PHASE_ENRAGED         = 1

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_BOLT, duration = math.random(4000, 8000)  },
        { action = "SET_TIMER", timer_id = TIMER_FLAME_SPIKE, duration = math.random(6000, 12000) },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
    }
end

function boss:OnDeath(input)
    return {
        { action = "SPAWN_CREATURE_NEAR_SELF", entry = NPC_SKELETON, distance = 5.0 },
        { action = "SPAWN_CREATURE_NEAR_SELF", entry = NPC_SKELETON, distance = 5.0 },
        { action = "SPAWN_CREATURE_NEAR_SELF", entry = NPC_SKELETON, distance = 5.0 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Enrage at 40% HP (one-time)
    if input.phase == PHASE_NORMAL and input.health_pct < 0.40 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BLOOD_PACT, target = "self" })
        table.insert(actions, { action = "EMOTE", text = "%s lets out an unearthly shriek!" })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_ENRAGED })
    end

    -- Shadow Bolt
    if input:IsTimerReady(TIMER_SHADOW_BOLT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_BOLT, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_BOLT, duration = math.random(5000, 10000) })
    end

    -- Flame Spike
    if input:IsTimerReady(TIMER_FLAME_SPIKE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FLAME_SPIKE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FLAME_SPIKE, duration = math.random(8000, 14000) })
    end

    return actions
end

RegisterCreatureAI(4543, boss)
