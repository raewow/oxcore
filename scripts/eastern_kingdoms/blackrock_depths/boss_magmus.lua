--[[
    @script_type: creature_ai
    @entry: 9938
    @name: boss_magmus

    Boss Magmus - Blackrock Depths (Iron Hall)
    Ported from MaNGOS boss_magmus.cpp
]]

local SPELL_FIERY_BURST = 13900
local SPELL_WAR_STOMP   = 24375

local TIMER_FIERY_BURST = 1
local TIMER_WAR_STOMP   = 2

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_FIERY_BURST, duration = 5000 },
        { action = "SET_DATA", data_id = BRD.TYPE_IRON_HALL, value = IN_PROGRESS },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_FIERY_BURST, duration = 5000 },
        { action = "SET_DATA", data_id = BRD.TYPE_IRON_HALL, value = FAIL },
    }
end

function boss:OnDeath(input, killer_guid)
    return {
        { action = "SET_DATA", data_id = BRD.TYPE_IRON_HALL, value = DONE },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Fiery Burst on current target
    if input:IsTimerReady(TIMER_FIERY_BURST) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FIERY_BURST, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FIERY_BURST, duration = 6000 })
    end

    -- War Stomp below 51% HP
    if input.health_pct < 0.51 and input:IsTimerReady(TIMER_WAR_STOMP) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_WAR_STOMP, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WAR_STOMP, duration = 8000 })
    end

    return actions
end

RegisterCreatureAI(9938, boss)
