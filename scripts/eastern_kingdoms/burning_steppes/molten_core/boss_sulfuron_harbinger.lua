--[[
    @script_type: creature_ai
    @entry: 12098
    @name: boss_sulfuron_harbinger

    Sulfuron Harbinger - Molten Core
    Ported from vmangos boss_sulfuron_harbinger.cpp
]]

-- Spells (exact from vmangos)
local SPELL_DARKSTRIKE = 19777
local SPELL_DEMORALIZINGSHOUT = 19778
local SPELL_INSPIRE = 19779
local SPELL_KNOCKDOWN = 19780
local SPELL_FLAMESPEAR = 19781

-- Timer IDs
local TIMER_DARKSTRIKE = 1
local TIMER_DEMORALIZINGSHOUT = 2
local TIMER_INSPIRE = 3
local TIMER_KNOCKDOWN = 4
local TIMER_FLAMESPEAR = 5

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_DARKSTRIKE, duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_DEMORALIZINGSHOUT, duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_INSPIRE, duration = 13000 },
        { action = "SET_TIMER", timer_id = TIMER_KNOCKDOWN, duration = 6000 },
        { action = "SET_TIMER", timer_id = TIMER_FLAMESPEAR, duration = 2000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Dark Strike (current target, repeats urand(15000,18000) -> 16500)
    if input:IsTimerReady(TIMER_DARKSTRIKE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DARKSTRIKE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_DARKSTRIKE, duration = 16500 })
    end

    -- Demoralizing Shout (repeats urand(15000,20000) -> 17500)
    if input:IsTimerReady(TIMER_DEMORALIZINGSHOUT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DEMORALIZINGSHOUT, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_DEMORALIZINGSHOUT, duration = 17500 })
    end

    -- Inspire (cast on self/friendly, repeats urand(20000,26000) -> 23000)
    if input:IsTimerReady(TIMER_INSPIRE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_INSPIRE, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_INSPIRE, duration = 23000 })
    end

    -- Knockdown (repeats urand(12000,15000) -> 13500)
    if input:IsTimerReady(TIMER_KNOCKDOWN) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_KNOCKDOWN, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_KNOCKDOWN, duration = 13500 })
    end

    -- Flamespear on random target (repeats urand(12000,16000) -> 14000)
    if input:IsTimerReady(TIMER_FLAMESPEAR) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FLAMESPEAR, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FLAMESPEAR, duration = 14000 })
    end

    return actions
end

RegisterCreatureAI(12098, boss)
