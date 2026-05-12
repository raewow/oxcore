--[[
    @script_type: creature_ai
    @entry: 3654
    @name: boss_mutanus

    Mutanus the Devourer - Wailing Caverns
    Based on known abilities (spawned during Disciple of Naralex escort event)

    Mutanus is a large murloc-like creature that appears during the
    awakening ritual for Naralex. He is the final boss of the instance.

    Abilities:
    - Thundercrack: AoE nature damage (8150)
    - Naralex's Nightmare: Sleep effect on a random target (7967)
    - Terrify: Fear on current target (7399)
]]

local SPELL_THUNDERCRACK      = 8150
local SPELL_NARALEXS_NIGHTMARE = 7967
local SPELL_TERRIFY           = 7399

local TIMER_THUNDERCRACK      = 1
local TIMER_NIGHTMARE         = 2
local TIMER_TERRIFY           = 3

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_THUNDERCRACK, duration = math.random(8000, 13000) },
        { action = "SET_TIMER", timer_id = TIMER_NIGHTMARE, duration = math.random(10000, 18000) },
        { action = "SET_TIMER", timer_id = TIMER_TERRIFY, duration = math.random(5000, 8000) },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_THUNDERCRACK, duration = math.random(8000, 13000) },
        { action = "SET_TIMER", timer_id = TIMER_NIGHTMARE, duration = math.random(10000, 18000) },
        { action = "SET_TIMER", timer_id = TIMER_TERRIFY, duration = math.random(5000, 8000) },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Thundercrack (AoE)
    if input:IsTimerReady(TIMER_THUNDERCRACK) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_THUNDERCRACK, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_THUNDERCRACK, duration = math.random(8000, 15000) })
    end

    -- Naralex's Nightmare (sleep on random target)
    if input:IsTimerReady(TIMER_NIGHTMARE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_NARALEXS_NIGHTMARE, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_NIGHTMARE, duration = math.random(15000, 25000) })
    end

    -- Terrify (fear on current target)
    if input:IsTimerReady(TIMER_TERRIFY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TERRIFY, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TERRIFY, duration = math.random(10000, 15000) })
    end

    return actions
end

RegisterCreatureAI(3654, boss)
