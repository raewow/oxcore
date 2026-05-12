--[[
    @script_type: creature_ai
    @entry: 9816
    @name: boss_pyroguard_emberseer

    Pyroguard Emberseer - Upper Blackrock Spire
    Ported from MaNGOS boss_pyroguard_emberseer.cpp

    Note: The activation event (growing/intro) is NYI in the C++ source
    and is omitted here. This script covers the combat phase only.
]]

local SPELL_FIRENOVA    = 23462
local SPELL_FLAMEBUFFET = 23341
local SPELL_PYROBLAST   = 20228

local TIMER_FIRENOVA    = 1
local TIMER_FLAMEBUFFET = 2
local TIMER_PYROBLAST   = 3

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_FIRENOVA, duration = 6000 },
        { action = "SET_TIMER", timer_id = TIMER_FLAMEBUFFET, duration = 3000 },
        { action = "SET_TIMER", timer_id = TIMER_PYROBLAST, duration = 14000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_FIRENOVA, duration = 6000 },
        { action = "SET_TIMER", timer_id = TIMER_FLAMEBUFFET, duration = 3000 },
        { action = "SET_TIMER", timer_id = TIMER_PYROBLAST, duration = 14000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Fire Nova
    if input:IsTimerReady(TIMER_FIRENOVA) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FIRENOVA, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FIRENOVA, duration = 6000 })
    end

    -- Flame Buffet
    if input:IsTimerReady(TIMER_FLAMEBUFFET) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FLAMEBUFFET, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FLAMEBUFFET, duration = 14000 })
    end

    -- Pyroblast on random target
    if input:IsTimerReady(TIMER_PYROBLAST) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_PYROBLAST, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PYROBLAST, duration = 15000 })
    end

    return actions
end

RegisterCreatureAI(9816, boss)
