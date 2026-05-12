--[[
    @script_type: creature_ai
    @entry: 8567
    @name: boss_tuten_kash

    Tuten'kash - Razorfen Downs
    No vmangos C++ reference available; scripted from known 1.12 behavior.

    A giant spider boss in Razorfen Downs. He is revealed by ringing
    three gongs in the dungeon (handled by the zone script). Once all
    gongs ring, he bursts from his web sac.

    Mechanics:
    - Web Spray: periodic AoE web/snare on targets in front
    - Curse of Tuten'kash: DoT curse on a random target
    - Summon Skitterer: periodically spawns spider adds
    - Frenzy at 30% HP
]]

local SPELL_WEB_SPRAY           = 15052
local SPELL_CURSE_OF_TUTENKASH  = 15053   -- Curse of Tuten'kash
local SPELL_SUMMON_SKITTERER    = 15056

local NPC_TOMB_SKITTERER        = 8573

local TIMER_WEB_SPRAY           = 1
local TIMER_CURSE               = 2
local TIMER_SUMMON              = 3

local PHASE_NORMAL              = 0
local PHASE_ENRAGED             = 1

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
        { action = "SET_TIMER", timer_id = TIMER_WEB_SPRAY, duration = math.random(5000, 10000)  },
        { action = "SET_TIMER", timer_id = TIMER_CURSE,     duration = math.random(8000, 15000)  },
        { action = "SET_TIMER", timer_id = TIMER_SUMMON,    duration = math.random(15000, 20000) },
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

    -- Frenzy at 30% HP (one-time)
    if input.phase == PHASE_NORMAL and input.health_pct < 0.30 then
        table.insert(actions, { action = "EMOTE", text = "%s goes into a frenzy!" })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_ENRAGED })
    end

    -- Web Spray
    if input:IsTimerReady(TIMER_WEB_SPRAY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_WEB_SPRAY, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WEB_SPRAY, duration = math.random(8000, 14000) })
    end

    -- Curse of Tuten'kash
    if input:IsTimerReady(TIMER_CURSE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CURSE_OF_TUTENKASH, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CURSE, duration = math.random(12000, 20000) })
    end

    -- Summon Tomb Skitterer
    if input:IsTimerReady(TIMER_SUMMON) then
        table.insert(actions, { action = "SPAWN_CREATURE_NEAR_SELF", entry = NPC_TOMB_SKITTERER, distance = 8.0 })
        table.insert(actions, { action = "SPAWN_CREATURE_NEAR_SELF", entry = NPC_TOMB_SKITTERER, distance = 8.0 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SUMMON, duration = math.random(20000, 30000) })
    end

    return actions
end

RegisterCreatureAI(8567, boss)
