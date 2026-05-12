--[[
    @script_type: creature_ai
    @entry: 10808
    @name: boss_timmy_the_cruel

    Timmy the Cruel - Stratholme
    Ported from MaNGOS boss_timmy_the_cruel.cpp

    Mechanics:
    - Ravenous Claw on current target every 15s (initial 10s)
]]

local SPELL_RAVENOUS_CLAW = 17470

local TIMER_RAVENOUS_CLAW = 1

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_RAVENOUS_CLAW, duration = 10000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Ravenous Claw
    if input:IsTimerReady(TIMER_RAVENOUS_CLAW) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_RAVENOUS_CLAW, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_RAVENOUS_CLAW, duration = 15000 })
    end

    return actions
end

RegisterCreatureAI(10808, boss)
