--[[
    @script_type: creature_ai
    @entry: 9156
    @name: boss_ambassador_flamelash

    Ambassador Flamelash - Blackrock Depths
    Ported from MaNGOS boss_ambassador_flamelash.cpp
]]

local SPELL_FIREBLAST      = 15573
local NPC_BURNING_SPIRIT   = 9178

local TIMER_FIREBLAST = 1
local TIMER_SPIRITS   = 2

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_FIREBLAST, duration = 2000 },
        { action = "SET_TIMER", timer_id = TIMER_SPIRITS, duration = 24000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_FIREBLAST, duration = 2000 },
        { action = "SET_TIMER", timer_id = TIMER_SPIRITS, duration = 24000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Fire Blast
    if input:IsTimerReady(TIMER_FIREBLAST) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FIREBLAST, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FIREBLAST, duration = 7000 })
    end

    -- Summon 4 Burning Spirits
    if input:IsTimerReady(TIMER_SPIRITS) then
        table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_BURNING_SPIRIT })
        table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_BURNING_SPIRIT })
        table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_BURNING_SPIRIT })
        table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_BURNING_SPIRIT })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SPIRITS, duration = 30000 })
    end

    return actions
end

RegisterCreatureAI(9156, boss)
