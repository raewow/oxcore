--[[
    @script_type: creature_ai
    @entry: 10503
    @name: boss_jandice_barov

    Jandice Barov - Scholomance
    Ported from MaNGOS boss_jandice_barov.cpp

    Mechanics:
    - Curse of Blood on current target every 30s (initial 15s)
    - Illusion phase every 25s (initial 30s): goes invisible, spawns 10
      Illusion of Jandice Barov (entry 11439), reappears after 3s
]]

local SPELL_CURSE_OF_BLOOD = 24673

local NPC_ILLUSION = 11439

local TIMER_CURSE_OF_BLOOD = 1
local TIMER_ILLUSION       = 2
local TIMER_REAPPEAR       = 3

local PHASE_NORMAL    = 0
local PHASE_INVISIBLE = 1

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_CURSE_OF_BLOOD, duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_ILLUSION, duration = 30000 },
        { action = "SET_PHASE", phase = PHASE_NORMAL },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Reappear after invisibility
    if input.phase == PHASE_INVISIBLE then
        if input:IsTimerReady(TIMER_REAPPEAR) then
            table.insert(actions, { action = "SET_PHASE", phase = PHASE_NORMAL })
            table.insert(actions, { action = "SET_FACTION", faction = 14 })
            table.insert(actions, { action = "REMOVE_UNIT_FLAG", flag = "NOT_SELECTABLE" })
            table.insert(actions, { action = "MORPH", display_id = 11073 })
        end
        return actions
    end

    -- Curse of Blood
    if input:IsTimerReady(TIMER_CURSE_OF_BLOOD) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CURSE_OF_BLOOD, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CURSE_OF_BLOOD, duration = 30000 })
    end

    -- Illusion phase
    if input:IsTimerReady(TIMER_ILLUSION) then
        table.insert(actions, { action = "INTERRUPT_SPELL" })
        table.insert(actions, { action = "SET_FACTION", faction = 35 })
        table.insert(actions, { action = "SET_UNIT_FLAG", flag = "NOT_SELECTABLE" })
        table.insert(actions, { action = "MORPH", display_id = 11686 })
        table.insert(actions, { action = "RESET_THREAT" })

        -- Spawn 10 illusions
        for i = 1, 10 do
            table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_ILLUSION, target = "random_hostile" })
        end

        table.insert(actions, { action = "SET_PHASE", phase = PHASE_INVISIBLE })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_REAPPEAR, duration = 3000 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ILLUSION, duration = 25000 })
    end

    return actions
end

RegisterCreatureAI(10503, boss)
