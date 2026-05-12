--[[
    @script_type: creature_ai
    @entry: 14510
    @name: boss_marli

    High Priestess Mar'li - Zul'Gurub
    Verified against vmangos boss_marli.cpp

    Phase 1: Troll form - PoisonVolley, DrainLife, SpawnSpiders, Aggrandir (self-buff)
    Phase 2: Spider form - EnvelopingWebs, Charge, CorrosivePoison
    Transforms every 35s; Trash in both phases
]]

local NPC_SPAWN_OF_MARLI    = 15041

local SPELL_CHARGE          = 22911
local SPELL_ENVELOPINGWEBS  = 24110
local SPELL_POISONVOLLEY    = 24099
local SPELL_SPIDER_FORM     = 24084
local SPELL_DRAIN_LIFE      = 24300
local SPELL_CORROSIVE_POISON = 24111
local SPELL_TRANSFORM_BACK  = 24085
local SPELL_TRASH           = 3391
local SPELL_HATCH           = 24083  -- visual on aggro
local SPELL_AGGRANDIR       = 24109  -- self-buff, phase 1 only

local TIMER_POISON_VOLLEY   = 1
local TIMER_DRAIN_LIFE      = 2
local TIMER_SPAWN_SPIDER    = 3
local TIMER_WEBS            = 4
local TIMER_CHARGE          = 5
local TIMER_CORROSIVE       = 6
local TIMER_TRANSFORM       = 7
local TIMER_TRASH           = 8
local TIMER_AGGRANDIR       = 9

-- Phase 1 = troll, Phase 2 = spider

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SCRIPT_TEXT", text_id = 10448 },  -- SAY_SPIDER_SPAWN
        { action = "SET_PHASE", phase = 1 },
        { action = "CAST_SPELL", spell_id = SPELL_HATCH, target = "self" },
        -- Spawn initial 4 spiders from eggs near boss
        { action = "SPAWN_CREATURE_NEAR_SELF", entry = NPC_SPAWN_OF_MARLI },
        { action = "SPAWN_CREATURE_NEAR_SELF", entry = NPC_SPAWN_OF_MARLI },
        { action = "SPAWN_CREATURE_NEAR_SELF", entry = NPC_SPAWN_OF_MARLI },
        { action = "SPAWN_CREATURE_NEAR_SELF", entry = NPC_SPAWN_OF_MARLI },
        { action = "SET_TIMER", timer_id = TIMER_POISON_VOLLEY, duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_DRAIN_LIFE,    duration = 30000 },
        { action = "SET_TIMER", timer_id = TIMER_SPAWN_SPIDER,  duration = 20000 },
        { action = "SET_TIMER", timer_id = TIMER_AGGRANDIR,     duration = 0 },
        { action = "SET_TIMER", timer_id = TIMER_WEBS,          duration = 5000 },
        { action = "SET_TIMER", timer_id = TIMER_CHARGE,        duration = 99999 },
        { action = "SET_TIMER", timer_id = TIMER_CORROSIVE,     duration = 1000 },
        { action = "SET_TIMER", timer_id = TIMER_TRANSFORM,     duration = 35000 },
        { action = "SET_TIMER", timer_id = TIMER_TRASH,         duration = 5000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_PHASE", phase = 1 },
        { action = "SET_TIMER", timer_id = TIMER_POISON_VOLLEY, duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_DRAIN_LIFE,    duration = 30000 },
        { action = "SET_TIMER", timer_id = TIMER_SPAWN_SPIDER,  duration = 20000 },
        { action = "SET_TIMER", timer_id = TIMER_AGGRANDIR,     duration = 0 },
        { action = "SET_TIMER", timer_id = TIMER_WEBS,          duration = 5000 },
        { action = "SET_TIMER", timer_id = TIMER_CHARGE,        duration = 99999 },
        { action = "SET_TIMER", timer_id = TIMER_CORROSIVE,     duration = 1000 },
        { action = "SET_TIMER", timer_id = TIMER_TRANSFORM,     duration = 35000 },
        { action = "SET_TIMER", timer_id = TIMER_TRASH,         duration = 5000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Phase 1: Troll form
    if input.phase == 1 then
        if input:IsTimerReady(TIMER_POISON_VOLLEY) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_POISONVOLLEY, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_POISON_VOLLEY, duration = math.random(10000, 20000) })
        end

        if input:IsTimerReady(TIMER_DRAIN_LIFE) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DRAIN_LIFE, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_DRAIN_LIFE, duration = math.random(20000, 50000) })
        end

        if input:IsTimerReady(TIMER_SPAWN_SPIDER) then
            table.insert(actions, { action = "SPAWN_CREATURE_NEAR_SELF", entry = NPC_SPAWN_OF_MARLI })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SPAWN_SPIDER, duration = math.random(20000, 30000) })
        end

        -- Aggrandir: self-buff; fires immediately then repeats
        if input:IsTimerReady(TIMER_AGGRANDIR) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_AGGRANDIR, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_AGGRANDIR, duration = math.random(10000, 20000) })
        end
    end

    -- Phase 2: Spider form
    if input.phase == 2 then
        -- Enveloping Webs on current target; then Charge at top aggro target
        if input:IsTimerReady(TIMER_WEBS) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ENVELOPINGWEBS, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WEBS, duration = math.random(10000, 15000) })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CHARGE, duration = 1000 })
        end

        -- Charge top-aggro target after webs
        if input:IsTimerReady(TIMER_CHARGE) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CHARGE, target = "current_target" })
            table.insert(actions, { action = "RESET_THREAT" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CHARGE, duration = 99999 })
        end

        if input:IsTimerReady(TIMER_CORROSIVE) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CORROSIVE_POISON, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CORROSIVE, duration = math.random(25000, 35000) })
        end
    end

    -- Transform toggle (both phases), every 35s
    if input:IsTimerReady(TIMER_TRANSFORM) then
        if input.phase == 1 then
            table.insert(actions, { action = "SCRIPT_TEXT", text_id = 10443 })  -- SAY_TRANSFORM
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SPIDER_FORM, target = "self" })
            table.insert(actions, { action = "RESET_THREAT" })
            table.insert(actions, { action = "SET_PHASE", phase = 2 })
        else
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TRANSFORM_BACK, target = "self" })
            table.insert(actions, { action = "SET_PHASE", phase = 1 })
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TRANSFORM, duration = 35000 })
    end

    -- Trash (both phases)
    if input:IsTimerReady(TIMER_TRASH) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TRASH, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TRASH, duration = math.random(10000, 20000) })
    end

    return actions
end

function boss:OnDeath(input)
    return { { action = "SCRIPT_TEXT", text_id = 10459 } }  -- SAY_DEATH
end

RegisterCreatureAI(14510, boss)
