--[[
    @script_type: creature_ai
    @entry: 9736
    @name: boss_quartermaster_zigris

    Quartermaster Zigris - Lower Blackrock Spire
    Ported from MaNGOS boss_quartermaster_zigris.cpp
]]

local SPELL_SHOOT    = 16496
local SPELL_STUNBOMB = 16497

local TIMER_SHOOT    = 1
local TIMER_STUNBOMB = 2

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_SHOOT, duration = 1000 },
        { action = "SET_TIMER", timer_id = TIMER_STUNBOMB, duration = 16000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_SHOOT, duration = 1000 },
        { action = "SET_TIMER", timer_id = TIMER_STUNBOMB, duration = 16000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Shoot
    if input:IsTimerReady(TIMER_SHOOT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHOOT, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHOOT, duration = 500 })
    end

    -- Stun Bomb
    if input:IsTimerReady(TIMER_STUNBOMB) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_STUNBOMB, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_STUNBOMB, duration = 14000 })
    end

    return actions
end

RegisterCreatureAI(9736, boss)
