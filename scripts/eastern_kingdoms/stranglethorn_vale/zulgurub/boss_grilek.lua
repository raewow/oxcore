--[[
    @script_type: creature_ai
    @entry: 15082
    @name: boss_grilek

    Gri'lek - Zul'Gurub
    Verified against vmangos boss_grilek.cpp
]]

local SPELL_AVATAR       = 24646  -- AoE stun; reduces victim threat 50% on hit
local SPELL_GROUNDTREMOR = 6524

local TIMER_AVATAR       = 1
local TIMER_GROUNDTREMOR = 2

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_AVATAR,       duration = math.random(15000, 25000) },
        { action = "SET_TIMER", timer_id = TIMER_GROUNDTREMOR, duration = math.random(8000, 16000) },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_AVATAR,       duration = math.random(15000, 25000) },
        { action = "SET_TIMER", timer_id = TIMER_GROUNDTREMOR, duration = math.random(8000, 16000) },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Avatar on current victim; reduces victim threat 50%; repeat every 25-35s
    if input:IsTimerReady(TIMER_AVATAR) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_AVATAR, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_AVATAR, duration = math.random(25000, 35000) })
    end

    -- Ground Tremor (AoE self); repeat every 12-16s
    if input:IsTimerReady(TIMER_GROUNDTREMOR) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_GROUNDTREMOR, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_GROUNDTREMOR, duration = math.random(12000, 16000) })
    end

    return actions
end

RegisterCreatureAI(15082, boss)
