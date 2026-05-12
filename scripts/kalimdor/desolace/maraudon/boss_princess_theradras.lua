--[[
    @script_type: creature_ai
    @entry: 12201
    @name: boss_princess_theradras

    Princess Theradras - Maraudon
    No vmangos C++ reference available; scripted from known 1.12 behavior.

    The final boss of Maraudon. Daughter of the earth elementals.
    She is an earth elemental lady whose death causes Zaetar's spirit
    to appear (handled by the instance script).

    Mechanics:
    - Dust Field: periodic AoE nature damage + knockback
    - Boulder Bash: stun + damage on tank
    - Earth Shock: melee interrupt spell
    - Repulse: knockback all nearby players
    - Summon Shambling Corpses: summons zombie-like adds
    - Frenzy at 30% HP
]]

local SPELL_DUST_FIELD          = 21909
local SPELL_BOULDER_BASH        = 21832
local SPELL_EARTH_SHOCK         = 22104
local SPELL_REPULSE             = 21908
local SPELL_SUMMON_DEAD_EARTH   = 21878
local SPELL_FRENZY              = 8269

local NPC_SHAMBLING_CORPSE      = 12272

local TIMER_DUST_FIELD          = 1
local TIMER_BOULDER_BASH        = 2
local TIMER_EARTH_SHOCK         = 3
local TIMER_REPULSE             = 4
local TIMER_SUMMON              = 5

local PHASE_NORMAL              = 0
local PHASE_ENRAGED             = 1

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
        { action = "YELL", text = "Intruders! You dare enter my sanctum?!" },
        { action = "SET_TIMER", timer_id = TIMER_DUST_FIELD,   duration = math.random(5000, 10000)  },
        { action = "SET_TIMER", timer_id = TIMER_BOULDER_BASH, duration = math.random(8000, 12000)  },
        { action = "SET_TIMER", timer_id = TIMER_EARTH_SHOCK,  duration = math.random(6000, 10000)  },
        { action = "SET_TIMER", timer_id = TIMER_REPULSE,      duration = math.random(15000, 20000) },
        { action = "SET_TIMER", timer_id = TIMER_SUMMON,       duration = math.random(20000, 30000) },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
    }
end

function boss:OnDeath(input)
    return {
        { action = "YELL", text = "Father... I return to you..." },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Frenzy at 30% HP (one-time)
    if input.phase == PHASE_NORMAL and input.health_pct < 0.30 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FRENZY, target = "self" })
        table.insert(actions, { action = "EMOTE", text = "%s goes into a frenzy!" })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_ENRAGED })
    end

    -- Dust Field (periodic AoE)
    if input:IsTimerReady(TIMER_DUST_FIELD) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DUST_FIELD, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_DUST_FIELD, duration = math.random(8000, 14000) })
    end

    -- Boulder Bash (stun tank)
    if input:IsTimerReady(TIMER_BOULDER_BASH) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BOULDER_BASH, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BOULDER_BASH, duration = math.random(10000, 18000) })
    end

    -- Earth Shock
    if input:IsTimerReady(TIMER_EARTH_SHOCK) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_EARTH_SHOCK, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EARTH_SHOCK, duration = math.random(8000, 14000) })
    end

    -- Repulse (knockback)
    if input:IsTimerReady(TIMER_REPULSE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_REPULSE, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_REPULSE, duration = math.random(20000, 30000) })
    end

    -- Summon Shambling Corpses
    if input:IsTimerReady(TIMER_SUMMON) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUMMON_DEAD_EARTH, target = "self" })
        table.insert(actions, { action = "SPAWN_CREATURE_NEAR_SELF", entry = NPC_SHAMBLING_CORPSE, distance = 10.0 })
        table.insert(actions, { action = "SPAWN_CREATURE_NEAR_SELF", entry = NPC_SHAMBLING_CORPSE, distance = 10.0 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SUMMON, duration = math.random(25000, 35000) })
    end

    return actions
end

RegisterCreatureAI(12201, boss)
