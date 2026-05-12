--[[
    @script_type: creature_ai
    @entry: 3974
    @name: boss_houndmaster_loksey

    Boss Houndmaster Loksey - Scarlet Monastery (Library)
    Ported from MaNGOS boss_houndmaster_loksey.cpp
]]

local SPELL_SUMMON_SCARLET_HOUND = 17164
local SPELL_BLOODLUST            = 6742

local TIMER_BLOODLUST = 1

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SAY", text = "Release the hounds!" },
        { action = "CAST_SPELL", spell_id = SPELL_SUMMON_SCARLET_HOUND, target = "self" },
        { action = "SET_TIMER", timer_id = TIMER_BLOODLUST, duration = 20000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_BLOODLUST, duration = 20000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    if input:IsTimerReady(TIMER_BLOODLUST) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BLOODLUST, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BLOODLUST, duration = 20000 })
    end

    return actions
end

RegisterCreatureAI(3974, boss)
