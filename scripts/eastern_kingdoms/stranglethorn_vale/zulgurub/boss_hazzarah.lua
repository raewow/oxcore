--[[
    @script_type: creature_ai
    @entry: 15083
    @name: boss_hazzarah

    Hazza'rah - Zul'Gurub
    Verified against vmangos boss_hazzarah.cpp

    Notes:
    - Sleep reduces victim threat 100% and switches to new target
    - Illusions spawns 3x mob 15163 near a random player
]]

local SPELL_MANABURN = 26046
local SPELL_SLEEP    = 24664  -- on current target; resets their threat

local NPC_ILLUSION = 15163

local TIMER_MANABURN  = 1
local TIMER_SLEEP     = 2
local TIMER_ILLUSIONS = 3

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_MANABURN,  duration = math.random(4000, 10000) },
        { action = "SET_TIMER", timer_id = TIMER_SLEEP,     duration = math.random(10000, 18000) },
        { action = "SET_TIMER", timer_id = TIMER_ILLUSIONS, duration = math.random(10000, 18000) },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_MANABURN,  duration = math.random(4000, 10000) },
        { action = "SET_TIMER", timer_id = TIMER_SLEEP,     duration = math.random(10000, 18000) },
        { action = "SET_TIMER", timer_id = TIMER_ILLUSIONS, duration = math.random(10000, 18000) },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Mana Burn on random mana user (approximated as random hostile); repeat 8-16s
    if input:IsTimerReady(TIMER_MANABURN) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MANABURN, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MANABURN, duration = math.random(8000, 16000) })
    end

    -- Sleep on current target; resets their threat, switches to new target; repeat 12-20s
    if input:IsTimerReady(TIMER_SLEEP) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SLEEP, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SLEEP, duration = math.random(12000, 20000) })
    end

    -- Illusions: summon 3x illusion mob near random player; repeat 15-25s
    if input:IsTimerReady(TIMER_ILLUSIONS) then
        for _ = 1, 3 do
            table.insert(actions, { action = "SPAWN_CREATURE_NEAR_RANDOM", entry = NPC_ILLUSION })
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ILLUSIONS, duration = math.random(15000, 25000) })
    end

    return actions
end

RegisterCreatureAI(15083, boss)
