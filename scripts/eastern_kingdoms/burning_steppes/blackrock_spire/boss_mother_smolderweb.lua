--[[
    @script_type: creature_ai
    @entry: 10596
    @name: boss_mother_smolderweb

    Mother Smolderweb - Lower Blackrock Spire
    Ported from MaNGOS boss_mother_smolderweb.cpp
]]

local SPELL_CRYSTALIZE              = 16104
local SPELL_MOTHERSMILK             = 16468
local SPELL_SUMMON_SPIRE_SPIDERLING = 16103

local TIMER_CRYSTALIZE   = 1
local TIMER_MOTHERSMILK  = 2

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_CRYSTALIZE, duration = 20000 },
        { action = "SET_TIMER", timer_id = TIMER_MOTHERSMILK, duration = 10000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_CRYSTALIZE, duration = 20000 },
        { action = "SET_TIMER", timer_id = TIMER_MOTHERSMILK, duration = 10000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Crystalize
    if input:IsTimerReady(TIMER_CRYSTALIZE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CRYSTALIZE, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CRYSTALIZE, duration = 15000 })
    end

    -- Mother's Milk
    if input:IsTimerReady(TIMER_MOTHERSMILK) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MOTHERSMILK, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MOTHERSMILK, duration = math.random(5000, 12500) })
    end

    return actions
end

-- Summon spiderlings on death
function boss:OnDamageTaken(input)
    local actions = {}
    if input.health_pct <= 0.01 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUMMON_SPIRE_SPIDERLING, target = "self" })
    end
    return actions
end

RegisterCreatureAI(10596, boss)
