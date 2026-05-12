--[[
    @script_type: creature_ai
    @entry: 1853
    @name: boss_darkmaster_gandling

    Darkmaster Gandling - Scholomance
    No vmangos C++ reference available; scripted from known 1.12 behavior.

    The final boss of Scholomance. A powerful necromancer who teleports
    players into locked side rooms during the fight.

    Mechanics:
    - Arcane Missiles: periodic arcane nuke
    - Shadow Bolt: periodic shadow nuke
    - Summon Bone Minions: summons skeletons periodically
    - Teleport: randomly teleports a player to a locked side chamber
      (in Vanilla the side room traps were gimmick; approximated here)
    - Frenzy enrage at 20% HP
]]

local SPELL_ARCANE_MISSILES = 9238
local SPELL_SHADOW_BOLT     = 17432
local SPELL_SUMMON_STUDENTS = 21496
local SPELL_TELEPORT        = 21473
local SPELL_FRENZY          = 8269

local NPC_BONE_MINION       = 11551

local TIMER_ARCANE_MISSILES = 1
local TIMER_SHADOW_BOLT     = 2
local TIMER_SUMMON          = 3
local TIMER_TELEPORT        = 4

local PHASE_NORMAL          = 0
local PHASE_ENRAGED         = 1

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
        { action = "YELL", text = "Class is in session." },
        { action = "SET_TIMER", timer_id = TIMER_ARCANE_MISSILES, duration = math.random(3000, 6000)   },
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_BOLT,     duration = math.random(6000, 12000)  },
        { action = "SET_TIMER", timer_id = TIMER_SUMMON,          duration = math.random(15000, 20000) },
        { action = "SET_TIMER", timer_id = TIMER_TELEPORT,        duration = math.random(20000, 30000) },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
    }
end

function boss:OnKill(input)
    return {
        { action = "SAY", text = "Excellent..." },
    }
end

function boss:OnDeath(input)
    return {
        { action = "YELL", text = "Class... dismissed..." },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Frenzy at 20% HP (one-time)
    if input.phase == PHASE_NORMAL and input.health_pct < 0.20 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FRENZY, target = "self" })
        table.insert(actions, { action = "EMOTE", text = "%s becomes enraged!" })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_ENRAGED })
    end

    -- Arcane Missiles
    if input:IsTimerReady(TIMER_ARCANE_MISSILES) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ARCANE_MISSILES, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ARCANE_MISSILES, duration = math.random(5000, 10000) })
    end

    -- Shadow Bolt
    if input:IsTimerReady(TIMER_SHADOW_BOLT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_BOLT, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_BOLT, duration = math.random(8000, 14000) })
    end

    -- Summon Bone Minions
    if input:IsTimerReady(TIMER_SUMMON) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUMMON_STUDENTS, target = "self" })
        table.insert(actions, { action = "SPAWN_CREATURE_NEAR_SELF", entry = NPC_BONE_MINION, distance = 8.0 })
        table.insert(actions, { action = "SPAWN_CREATURE_NEAR_SELF", entry = NPC_BONE_MINION, distance = 8.0 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SUMMON, duration = math.random(20000, 30000) })
    end

    -- Teleport (sends a random player to a side room)
    if input:IsTimerReady(TIMER_TELEPORT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TELEPORT, target = "random_hostile" })
        table.insert(actions, { action = "SAY", text = "Let's see how you do in detention!" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TELEPORT, duration = math.random(25000, 35000) })
    end

    return actions
end

RegisterCreatureAI(1853, boss)
