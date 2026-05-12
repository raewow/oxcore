--[[
    @script_type: creature_ai
    @entry: 12118
    @name: boss_lucifron

    Lucifron - Molten Core
    Ported from vmangos boss_lucifron.cpp
]]

-- Spells (exact from vmangos)
local SPELL_IMPENDING_DOOM = 19702   -- 2000 Shadow damage after 10s, 40yd radius
local SPELL_LUCIFRONS_CURSE = 19703  -- +100% spell/ability cost, 5min, 40yd radius
local SPELL_SHADOW_SHOCK = 19460     -- Instant shadow damage, 20yd radius

-- Timer IDs
local TIMER_IMPENDING_DOOM = 1
local TIMER_CURSE = 2
local TIMER_SHADOW_SHOCK = 3

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_IMPENDING_DOOM, duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_CURSE, duration = 20000 },
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_SHOCK, duration = 6000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Impending Doom (self-cast AoE, repeats every 20s)
    if input:IsTimerReady(TIMER_IMPENDING_DOOM) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_IMPENDING_DOOM, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_IMPENDING_DOOM, duration = 20000 })
    end

    -- Lucifron's Curse (self-cast AoE, repeats every 15s)
    if input:IsTimerReady(TIMER_CURSE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_LUCIFRONS_CURSE, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CURSE, duration = 15000 })
    end

    -- Shadow Shock (random target, repeats every 6s)
    if input:IsTimerReady(TIMER_SHADOW_SHOCK) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_SHOCK, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_SHOCK, duration = 6000 })
    end

    return actions
end

RegisterCreatureAI(12118, boss)
