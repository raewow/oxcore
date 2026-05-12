--[[
    @script_type: creature_ai
    @entry: 9028
    @name: boss_grizzle

    Boss Grizzle - Blackrock Depths (Ring of Law)
    Ported from MaNGOS boss_grizzle.cpp
]]

local SPELL_GROUND_TREMOR = 6524
local SPELL_FRENZY        = 8269

local TIMER_GROUND_TREMOR = 1
local TIMER_FRENZY        = 2

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_GROUND_TREMOR, duration = 12000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_GROUND_TREMOR, duration = 12000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Ground Tremor
    if input:IsTimerReady(TIMER_GROUND_TREMOR) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_GROUND_TREMOR, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_GROUND_TREMOR, duration = 8000 })
    end

    -- Frenzy below 51% HP
    if input.health_pct < 0.51 and input:IsTimerReady(TIMER_FRENZY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FRENZY, target = "self" })
        table.insert(actions, { action = "EMOTE", emote_id = EMOTE_GENERIC_FRENZY_KILL })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FRENZY, duration = 15000 })
    end

    return actions
end

RegisterCreatureAI(9028, boss)
