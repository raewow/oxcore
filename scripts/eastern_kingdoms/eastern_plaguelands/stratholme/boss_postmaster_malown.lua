--[[
    @script_type: creature_ai
    @entry: 11143
    @name: boss_postmaster_malown

    Postmaster Malown - Stratholme
    Ported from MaNGOS boss_postmaster_malown.cpp

    Mechanics:
    - Wailing Dead on current target every 19s
    - Backhand (stun) on current target every 8s
    - Curse of Weakness on current target every 20s (low chance)
    - Curse of Tongues on current target every 22s (low chance)
    - Call of the Grave on current target every 25s (low chance)
]]

local SPELL_WAILING_DEAD      = 7713
local SPELL_BACKHAND          = 6253
local SPELL_CURSE_OF_WEAKNESS = 8552
local SPELL_CURSE_OF_TONGUES  = 12889
local SPELL_CALL_OF_THE_GRAVE = 17831

local TIMER_WAILING_DEAD      = 1
local TIMER_BACKHAND          = 2
local TIMER_CURSE_OF_WEAKNESS = 3
local TIMER_CURSE_OF_TONGUES  = 4
local TIMER_CALL_OF_THE_GRAVE = 5

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_WAILING_DEAD, duration = 19000 },
        { action = "SET_TIMER", timer_id = TIMER_BACKHAND, duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_CURSE_OF_WEAKNESS, duration = 20000 },
        { action = "SET_TIMER", timer_id = TIMER_CURSE_OF_TONGUES, duration = 22000 },
        { action = "SET_TIMER", timer_id = TIMER_CALL_OF_THE_GRAVE, duration = 25000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Wailing Dead
    if input:IsTimerReady(TIMER_WAILING_DEAD) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_WAILING_DEAD, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WAILING_DEAD, duration = 19000 })
    end

    -- Backhand
    if input:IsTimerReady(TIMER_BACKHAND) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BACKHAND, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BACKHAND, duration = 8000 })
    end

    -- Curse of Weakness
    if input:IsTimerReady(TIMER_CURSE_OF_WEAKNESS) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CURSE_OF_WEAKNESS, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CURSE_OF_WEAKNESS, duration = 20000 })
    end

    -- Curse of Tongues
    if input:IsTimerReady(TIMER_CURSE_OF_TONGUES) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CURSE_OF_TONGUES, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CURSE_OF_TONGUES, duration = 22000 })
    end

    -- Call of the Grave
    if input:IsTimerReady(TIMER_CALL_OF_THE_GRAVE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CALL_OF_THE_GRAVE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CALL_OF_THE_GRAVE, duration = 25000 })
    end

    return actions
end

RegisterCreatureAI(11143, boss)
