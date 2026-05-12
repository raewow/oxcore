--[[
    @script_type: creature_ai
    @entry: 10184
    @name: boss_onyxia

    Onyxia - Onyxia's Lair
    Verified against vmangos boss_onyxia.cpp

    Three-phase encounter:
      Phase 1 (>65% HP): Ground combat with melee abilities
      Phase 2 (65%-40% HP): Aerial phase with fireballs, deep breath, whelp spawns
      Phase 3 (<40% HP): Ground combat with bellowing roar + whelp spawns
]]

-- Spell constants
local SPELL_WING_BUFFET       = 18500
local SPELL_FLAME_BREATH      = 18435
local SPELL_CLEAVE            = 19983
local SPELL_TAIL_SWEEP        = 15847
local SPELL_KNOCK_AWAY        = 19633
local SPELL_FIREBALL          = 18392
local SPELL_BELLOWING_ROAR    = 18431
local SPELL_HEATED_GROUND     = 22191

-- NPC entries
local NPC_ONYXIA_WHELP        = 11262

-- Timer IDs
local TIMER_FLAME_BREATH      = 1
local TIMER_TAIL_SWEEP        = 2
local TIMER_CLEAVE            = 3
local TIMER_WING_BUFFET       = 4
local TIMER_KNOCK_AWAY        = 5
local TIMER_FIREBALL          = 6
local TIMER_BELLOWING_ROAR    = 7
local TIMER_WHELP_SPAWN       = 8
local TIMER_PHASE_CHECK       = 9

-- Phase constants
local PHASE_ONE               = 1
local PHASE_TWO               = 2
local PHASE_THREE             = 3

-- Whelp spawn locations
local WHELP_SPAWN_1 = { x = -30.127, y = -254.463, z = -89.440, o = 0 }
local WHELP_SPAWN_2 = { x = -30.817, y = -177.106, z = -89.258, o = 0 }

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_PHASE", phase = PHASE_ONE },
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "YELL", text = "How fortuitous. Usually, I must leave my lair to feed." },
        { action = "SET_TIMER", timer_id = TIMER_FLAME_BREATH, duration = math.random(10000, 20000) },
        { action = "SET_TIMER", timer_id = TIMER_TAIL_SWEEP,   duration = 5000 },
        { action = "SET_TIMER", timer_id = TIMER_CLEAVE,       duration = math.random(2000, 5000) },
        { action = "SET_TIMER", timer_id = TIMER_WING_BUFFET,  duration = math.random(10000, 20000) },
        { action = "SET_TIMER", timer_id = TIMER_KNOCK_AWAY,   duration = math.random(15000, 25000) },
        { action = "SET_TIMER", timer_id = TIMER_PHASE_CHECK, duration = 1000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_PHASE", phase = PHASE_ONE },
        { action = "SET_COMBAT_MOVEMENT", enabled = true },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    ---------------------------------------------------------------
    -- PHASE ONE: Ground melee (above 65% HP)
    ---------------------------------------------------------------
    if input.phase == PHASE_ONE then

        -- Transition to Phase 2 at 65% HP
        if input.health_pct < 0.65 then
            table.insert(actions, { action = "SET_PHASE", phase = PHASE_TWO })
            table.insert(actions, { action = "YELL", text = "This meaningless exertion bores me. I'll incinerate you all from above!" })
            table.insert(actions, { action = "SET_COMBAT_MOVEMENT", enabled = false })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FIREBALL, duration = 3000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WHELP_SPAWN, duration = 5000 })
            return actions
        end

        -- Flame Breath (urand(10,20)s repeat)
        if input:IsTimerReady(TIMER_FLAME_BREATH) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FLAME_BREATH, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FLAME_BREATH, duration = math.random(10000, 20000) })
        end

        -- Tail Sweep (3500ms repeat)
        if input:IsTimerReady(TIMER_TAIL_SWEEP) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TAIL_SWEEP, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TAIL_SWEEP, duration = 3500 })
        end

        -- Cleave (urand(2,5)s repeat)
        if input:IsTimerReady(TIMER_CLEAVE) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CLEAVE, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = math.random(2000, 5000) })
        end

        -- Wing Buffet (urand(15,30)s repeat)
        if input:IsTimerReady(TIMER_WING_BUFFET) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_WING_BUFFET, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WING_BUFFET, duration = math.random(15000, 30000) })
        end

        -- Knock Away (urand(15,25)s repeat)
        if input:IsTimerReady(TIMER_KNOCK_AWAY) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_KNOCK_AWAY, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_KNOCK_AWAY, duration = math.random(15000, 25000) })
        end
    end

    ---------------------------------------------------------------
    -- PHASE TWO: Aerial phase (65% to 40% HP)
    ---------------------------------------------------------------
    if input.phase == PHASE_TWO then

        -- Transition to Phase 3 at 40% HP
        if input.health_pct < 0.40 then
            table.insert(actions, { action = "SET_PHASE", phase = PHASE_THREE })
            table.insert(actions, { action = "YELL", text = "It seems you'll need another lesson, mortals!" })
            table.insert(actions, { action = "SET_COMBAT_MOVEMENT", enabled = true })
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BELLOWING_ROAR, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FLAME_BREATH, duration = math.random(10000, 15000) })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TAIL_SWEEP,   duration = 5000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CLEAVE,       duration = math.random(2000, 5000) })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WING_BUFFET,  duration = math.random(10000, 20000) })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_KNOCK_AWAY,   duration = math.random(15000, 25000) })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BELLOWING_ROAR, duration = 10000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WHELP_SPAWN, duration = math.random(1000, 10000) })
            return actions
        end

        -- Fireball on random hostile
        if input:IsTimerReady(TIMER_FIREBALL) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FIREBALL, target = "random_hostile" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FIREBALL, duration = 4000 })
        end

        -- Spawn whelps from both sides (spawns ~16 whelps total over several ticks in C++;
        -- approximated as urand(5,7) per wave every 30s)
        if input:IsTimerReady(TIMER_WHELP_SPAWN) then
            local count = math.random(5, 7)
            for _ = 1, count do
                table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_ONYXIA_WHELP,
                    x = WHELP_SPAWN_1.x, y = WHELP_SPAWN_1.y, z = WHELP_SPAWN_1.z, o = WHELP_SPAWN_1.o,
                    summon_type = 1, duration = 300000 })
                table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_ONYXIA_WHELP,
                    x = WHELP_SPAWN_2.x, y = WHELP_SPAWN_2.y, z = WHELP_SPAWN_2.z, o = WHELP_SPAWN_2.o,
                    summon_type = 1, duration = 300000 })
            end
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WHELP_SPAWN, duration = 30000 })
        end
    end

    ---------------------------------------------------------------
    -- PHASE THREE: Ground with bellowing roar + whelps (<40% HP)
    ---------------------------------------------------------------
    if input.phase == PHASE_THREE then

        -- Flame Breath (urand(10,20)s repeat)
        if input:IsTimerReady(TIMER_FLAME_BREATH) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FLAME_BREATH, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FLAME_BREATH, duration = math.random(10000, 20000) })
        end

        -- Tail Sweep (3500ms repeat)
        if input:IsTimerReady(TIMER_TAIL_SWEEP) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TAIL_SWEEP, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TAIL_SWEEP, duration = 3500 })
        end

        -- Cleave (urand(2,5)s repeat)
        if input:IsTimerReady(TIMER_CLEAVE) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CLEAVE, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = math.random(2000, 5000) })
        end

        -- Wing Buffet (urand(15,30)s repeat)
        if input:IsTimerReady(TIMER_WING_BUFFET) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_WING_BUFFET, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WING_BUFFET, duration = math.random(15000, 30000) })
        end

        -- Knock Away (urand(15,25)s repeat)
        if input:IsTimerReady(TIMER_KNOCK_AWAY) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_KNOCK_AWAY, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_KNOCK_AWAY, duration = math.random(15000, 25000) })
        end

        -- Bellowing Roar (urand(15,30)s repeat; phase 3 only)
        if input:IsTimerReady(TIMER_BELLOWING_ROAR) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BELLOWING_ROAR, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BELLOWING_ROAR, duration = math.random(15000, 30000) })
        end

        -- Spawn single whelp from random side (urand(1,10)s repeat)
        if input:IsTimerReady(TIMER_WHELP_SPAWN) then
            local loc = (math.random(0, 1) == 0) and WHELP_SPAWN_1 or WHELP_SPAWN_2
            table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_ONYXIA_WHELP,
                x = loc.x, y = loc.y, z = loc.z, o = loc.o,
                summon_type = 1, duration = 120000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WHELP_SPAWN, duration = math.random(1000, 10000) })
        end
    end

    return actions
end

RegisterCreatureAI(10184, boss)
