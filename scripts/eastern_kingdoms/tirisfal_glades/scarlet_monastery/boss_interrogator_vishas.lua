--[[
    @script_type: creature_ai
    @entry: 3983
    @name: boss_interrogator_vishas

    Interrogator Vishas - Scarlet Monastery (Graveyard)
    Ported from vmangos boss_interrogator_vishas.cpp
]]

-- Spells
local SPELL_SHADOW_WORD_PAIN = 2767  -- Shadow damage over time on target

-- Timer IDs
local TIMER_SHADOW_WORD_PAIN = 1
local TIMER_YELL_60 = 2
local TIMER_YELL_30 = 3

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SAY", text = "Tell me... tell me everything!" },
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_WORD_PAIN, duration = 5000 },
        { action = "SET_TIMER", timer_id = TIMER_YELL_60, duration = 1000 },
        { action = "SET_TIMER", timer_id = TIMER_YELL_30, duration = 1000 },
    }
end

function boss:OnKill(input, victim_guid)
    return {
        { action = "SAY", text = "Purged by pain!" },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Shadow Word: Pain on current target, repeats every 5-15s
    if input:IsTimerReady(TIMER_SHADOW_WORD_PAIN) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_WORD_PAIN, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_WORD_PAIN, duration = math.random(5000, 15000) })
    end

    -- Yell at 60% health
    if input:IsTimerReady(TIMER_YELL_60) then
        if input.health_pct <= 60 and not input.yelled_60 then
            table.insert(actions, { action = "SAY", text = "Naughty secrets!" })
            input.yelled_60 = true
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_YELL_60, duration = 1000 })
    end

    -- Yell at 30% health
    if input:IsTimerReady(TIMER_YELL_30) then
        if input.health_pct <= 30 and not input.yelled_30 then
            table.insert(actions, { action = "SAY", text = "I'll rip the secrets from your flesh!" })
            input.yelled_30 = true
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_YELL_30, duration = 1000 })
    end

    return actions
end

RegisterCreatureAI(3983, boss)
