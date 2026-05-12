--[[
    @script_type: creature_ai
    @entry: 15084
    @name: boss_renataki

    Renataki - Zul'Gurub
    Verified against vmangos boss_renataki.cpp

    Notes:
    - Red Lightning cast once on first update (before combat)
    - Goes invisible urand(28,32)s after combat start; reappears after 20s with Ambush
    - Suriner Zone (AoE) every urand(9,11)s while visible
    - Thousand Blades every urand(7,12)s while visible
    - Aggro reduction every urand(7,20)s (modifies threat on current victim)
    - Enrage at 30% HP
]]

local SPELL_TRASH           = 3391
local SPELL_ENRAGE          = 8269
local SPELL_RED_LIGHTNING   = 24240  -- cast once on first update
local SPELL_AMBUSH          = 24337
local SPELL_THOUSANDBLADES  = 24649
local SPELL_SURINER_ZONE    = 24698

local TIMER_INVISIBLE       = 1
local TIMER_VISIBLE         = 2
local TIMER_THOUSANDBLADES  = 3
local TIMER_SURINER         = 4
local TIMER_AGGRO           = 5

-- Phase 1 = visible, Phase 2 = vanished
local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_PHASE", phase = 1 },
        { action = "CAST_SPELL", spell_id = SPELL_RED_LIGHTNING, target = "self" },
        { action = "SET_TIMER", timer_id = TIMER_INVISIBLE,     duration = math.random(28000, 32000) },
        { action = "SET_TIMER", timer_id = TIMER_VISIBLE,       duration = 99999 },
        { action = "SET_TIMER", timer_id = TIMER_THOUSANDBLADES, duration = math.random(4000, 8000) },
        { action = "SET_TIMER", timer_id = TIMER_SURINER,       duration = math.random(9000, 11000) },
        { action = "SET_TIMER", timer_id = TIMER_AGGRO,         duration = math.random(15000, 25000) },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_PHASE", phase = 1 },
        { action = "SET_TIMER", timer_id = TIMER_INVISIBLE,     duration = math.random(28000, 32000) },
        { action = "SET_TIMER", timer_id = TIMER_VISIBLE,       duration = 99999 },
        { action = "SET_TIMER", timer_id = TIMER_THOUSANDBLADES, duration = math.random(4000, 8000) },
        { action = "SET_TIMER", timer_id = TIMER_SURINER,       duration = math.random(9000, 11000) },
        { action = "SET_TIMER", timer_id = TIMER_AGGRO,         duration = math.random(15000, 25000) },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Enrage at 30% HP
    if input.health_pct and input.health_pct < 0.30 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ENRAGE, target = "self" })
    end

    -- Phase 1: Visible
    if input.phase == 1 then
        -- Go invisible
        if input:IsTimerReady(TIMER_INVISIBLE) then
            table.insert(actions, { action = "SET_VISIBILITY", visible = false })
            table.insert(actions, { action = "SET_PHASE", phase = 2 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_VISIBLE, duration = 20000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_INVISIBLE, duration = math.random(30000, 42000) })
        end

        -- Suriner Zone (AoE)
        if input:IsTimerReady(TIMER_SURINER) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SURINER_ZONE, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SURINER, duration = math.random(9000, 11000) })
        end

        -- Thousand Blades
        if input:IsTimerReady(TIMER_THOUSANDBLADES) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_THOUSANDBLADES, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_THOUSANDBLADES, duration = math.random(7000, 12000) })
        end

        -- Aggro reduction on current victim (-50% threat), switch to random target
        if input:IsTimerReady(TIMER_AGGRO) then
            table.insert(actions, { action = "RESET_THREAT" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_AGGRO, duration = math.random(7000, 20000) })
        end
    end

    -- Phase 2: Vanished - reappear and ambush after 20s
    if input.phase == 2 then
        if input:IsTimerReady(TIMER_VISIBLE) then
            table.insert(actions, { action = "SET_VISIBILITY", visible = true })
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_AMBUSH, target = "random_hostile" })
            table.insert(actions, { action = "SET_PHASE", phase = 1 })
        end
    end

    return actions
end

RegisterCreatureAI(15084, boss)
