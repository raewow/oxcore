--[[
    @script_type: creature_ai
    @entry: 17910
    @name: boss_order_of_silver_hand

    Order of the Silver Hand Bosses - Stratholme (Scarlet side, Slaughterhouse)
    Ported from vmangos boss_order_of_silver_hand.cpp

    Five paladins share this script (entries 17910-17914):
    - Gregor Agamand (17910)
    - Cathela the Seeker (17911)
    - Nemas the Arbiter (17912)
    - Aelmar the Vanquisher (17913)
    - Vicar Hieronymus (17914)

    Each uses Holy Light to heal themselves at <20% HP and Divine Shield at <5%.
    These are required for the Horde paladin epic mount quest (9737).
]]

-- Spells
local SPELL_HOLY_LIGHT   = 25263  -- Heals self when low HP (<20%)
local SPELL_DIVINE_SHIELD = 13874 -- Makes self immune when very low HP (<5%)

-- Timer IDs
local TIMER_HOLY_LIGHT    = 1
local TIMER_DIVINE_SHIELD = 2

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_HOLY_LIGHT,    duration = 20000 },
        { action = "SET_TIMER", timer_id = TIMER_DIVINE_SHIELD, duration = 20000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Holy Light (self-heal at <20% HP, repeats every 20s)
    if input:IsTimerReady(TIMER_HOLY_LIGHT) then
        if input.health_pct < 20 then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_HOLY_LIGHT, target = "self" })
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_HOLY_LIGHT, duration = 20000 })
    end

    -- Divine Shield (self-immunity at <5% HP, repeats every 40s)
    if input:IsTimerReady(TIMER_DIVINE_SHIELD) then
        if input.health_pct < 5 then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DIVINE_SHIELD, target = "self" })
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_DIVINE_SHIELD, duration = 40000 })
    end

    return actions
end

-- Register all 5 paladin entries with the same script
RegisterCreatureAI(17910, boss)  -- Gregor Agamand
RegisterCreatureAI(17911, boss)  -- Cathela the Seeker
RegisterCreatureAI(17912, boss)  -- Nemas the Arbiter
RegisterCreatureAI(17913, boss)  -- Aelmar the Vanquisher
RegisterCreatureAI(17914, boss)  -- Vicar Hieronymus
