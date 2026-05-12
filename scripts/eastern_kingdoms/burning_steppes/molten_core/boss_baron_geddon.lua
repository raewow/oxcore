--[[
    @script_type: creature_ai
    @entry: 12056
    @name: boss_baron_geddon

    Baron Geddon - Molten Core
    Ported from vmangos boss_baron_geddon.cpp
]]

-- Spells (exact from vmangos)
local SPELL_INFERNO = 19695
local SPELL_IGNITEMANA = 19659
local SPELL_LIVINGBOMB = 20475
local SPELL_ARMAGEDDOM = 20478    -- Below 5% HP, becomes inert

-- Timer IDs
local TIMER_IGNITE_MANA = 1
local TIMER_LIVING_BOMB = 2
local TIMER_INFERNO = 3

local boss = {}
local armageddon_triggered = false

function boss:OnEnterCombat(input)
    armageddon_triggered = false
    return {
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_IGNITE_MANA, duration = 12500 },  -- urand(10000,15000)
        { action = "SET_TIMER", timer_id = TIMER_LIVING_BOMB, duration = 17500 },  -- urand(15000,20000)
        { action = "SET_TIMER", timer_id = TIMER_INFERNO, duration = 21000 },      -- urand(18000,24000)
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    if not armageddon_triggered and input.health_pct < 0.05 then
        armageddon_triggered = true
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ARMAGEDDOM, target = "self" })
        table.insert(actions, { action = "SET_COMBAT_MOVEMENT", enabled = false })
        return actions
    end

    if armageddon_triggered then return actions end

    -- Living Bomb on random player (repeats urand(12000,15000))
    if input:IsTimerReady(TIMER_LIVING_BOMB) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_LIVINGBOMB, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_LIVING_BOMB, duration = 13500 })
    end

    -- Ignite Mana (self-cast AoE, repeats urand(20000,30000))
    if input:IsTimerReady(TIMER_IGNITE_MANA) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_IGNITEMANA, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_IGNITE_MANA, duration = 25000 })
    end

    -- Inferno (self-cast, roots Geddon, escalating AoE, repeats urand(18000,24000))
    if input:IsTimerReady(TIMER_INFERNO) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_INFERNO, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_INFERNO, duration = 21000 })
    end

    return actions
end

RegisterCreatureAI(12056, boss)
