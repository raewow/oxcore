--[[
    @script_type: creature_ai
    @entry: 6490
    @name: boss_azshir_the_sleepless

    Boss Azshir the Sleepless - Scarlet Monastery (Graveyard)
    Ported from MaNGOS boss_azshir_the_sleepless.cpp
]]

local SPELL_CALL_OF_THE_GRAVE = 17831
local SPELL_TERRIFY           = 7399
local SPELL_SOUL_SIPHON       = 7290

local TIMER_CALL_OF_THE_GRAVE = 1
local TIMER_TERRIFY           = 2
local TIMER_SOUL_SIPHON       = 3

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_SOUL_SIPHON, duration = 1 },
        { action = "SET_TIMER", timer_id = TIMER_CALL_OF_THE_GRAVE, duration = 30000 },
        { action = "SET_TIMER", timer_id = TIMER_TERRIFY, duration = 20000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_SOUL_SIPHON, duration = 1 },
        { action = "SET_TIMER", timer_id = TIMER_CALL_OF_THE_GRAVE, duration = 30000 },
        { action = "SET_TIMER", timer_id = TIMER_TERRIFY, duration = 20000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Soul Siphon below 50% HP
    if input.health_pct <= 0.50 and input:IsTimerReady(TIMER_SOUL_SIPHON) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SOUL_SIPHON, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SOUL_SIPHON, duration = 20000 })
    end

    -- Call of the Grave
    if input:IsTimerReady(TIMER_CALL_OF_THE_GRAVE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CALL_OF_THE_GRAVE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CALL_OF_THE_GRAVE, duration = 30000 })
    end

    -- Terrify
    if input:IsTimerReady(TIMER_TERRIFY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TERRIFY, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TERRIFY, duration = 20000 })
    end

    return actions
end

RegisterCreatureAI(6490, boss)
