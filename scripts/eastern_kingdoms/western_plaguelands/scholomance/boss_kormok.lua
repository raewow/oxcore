--[[
    @script_type: creature_ai
    @entry: 10426
    @name: boss_kormok

    Kormok - Scholomance
    Ported from vmangos boss_kormok.cpp

    Uses Shadow Bolt Volley on target and Bone Shield on self.
    Periodically summons 4 Skeletal Minions (entry 16119).
    At <26% HP, spawns 2 Bone Mages (entry 16120) once.
]]

-- Spells
local SPELL_SHADOW_BOLT_VOLLEY = 20741  -- AoE shadow damage (frontal cone), target
local SPELL_BONE_SHIELD        = 27688  -- Absorb shield on self

-- Add NPC entries
local NPC_SKELETAL_MINION = 16119  -- regular minion adds
local NPC_BONE_MAGE       = 16120  -- mage adds (phase 2, spawned at <26% HP)

-- Timer IDs
local TIMER_SHADOW_VOLLEY = 1
local TIMER_BONE_SHIELD   = 2
local TIMER_SUMMON_MINION = 3

local boss = {}

function boss:OnEnterCombat(input)
    input.mages_spawned = false
    return {
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_VOLLEY, duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_BONE_SHIELD,   duration = 2000 },
        { action = "SET_TIMER", timer_id = TIMER_SUMMON_MINION, duration = 15000 },
    }
end

function boss:OnReset(input)
    input.mages_spawned = false
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Shadow Bolt Volley (current target, repeats every 15s)
    if input:IsTimerReady(TIMER_SHADOW_VOLLEY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_BOLT_VOLLEY, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_VOLLEY, duration = 15000 })
    end

    -- Bone Shield (self, repeats every 45s)
    if input:IsTimerReady(TIMER_BONE_SHIELD) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BONE_SHIELD, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BONE_SHIELD, duration = 45000 })
    end

    -- Summon 4 Skeletal Minions (repeats every 12s)
    if input:IsTimerReady(TIMER_SUMMON_MINION) then
        for i = 1, 4 do
            table.insert(actions, {
                action = "SPAWN_CREATURE",
                entry = NPC_SKELETAL_MINION,
                x = 0, y = 0, z = 0, o = 0,
                summon_type = "timed_or_corpse_despawn",
                duration = 120000,
            })
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SUMMON_MINION, duration = 12000 })
    end

    -- Spawn 2 Bone Mages at <26% HP (once)
    if input.health_pct < 26 and not input.mages_spawned then
        input.mages_spawned = true
        for i = 1, 2 do
            table.insert(actions, {
                action = "SPAWN_CREATURE",
                entry = NPC_BONE_MAGE,
                x = 0, y = 0, z = 0, o = 0,
                summon_type = "timed_or_corpse_despawn",
                duration = 120000,
            })
        end
    end

    return actions
end

RegisterCreatureAI(10426, boss)
