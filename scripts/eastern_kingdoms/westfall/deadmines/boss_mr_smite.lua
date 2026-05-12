--[[
    @script_type: creature_ai
    @entry: 646
    @name: boss_mr_smite

    Mr. Smite - Deadmines
    Ported from MaNGOS boss_mr_smite.cpp

    Phase 1: Has Nimble Reflexes buff, normal melee
    Phase 2 (at 66% HP): Stomps, goes to chest, equips dual axes + Thrash
    Phase 3 (at 33% HP): Stomps, goes to chest, equips hammer + Smite Hammer buff
    During equipment phases, stops attacking and kneels at chest
]]

local SPELL_NIMBLE_REFLEXES  = 6433
local SPELL_SMITE_SLAM       = 6435
local SPELL_SMITE_STOMP      = 6432
local SPELL_SMITE_HAMMER     = 6436
local SPELL_THRASH           = 12787

local TIMER_SLAM             = 1
local TIMER_EQUIP            = 2

-- Phases
local PHASE_1                = 1
local PHASE_2                = 2
local PHASE_3                = 3
local PHASE_EQUIP_START      = 4
local PHASE_EQUIP_PROCESS    = 5
local PHASE_EQUIP_END        = 6

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_PHASE", phase = PHASE_1 },
        { action = "SET_TIMER", timer_id = TIMER_SLAM, duration = 9000 },
        { action = "CAST_SPELL", spell_id = SPELL_NIMBLE_REFLEXES, target = "self" },
        { action = "SET_CUSTOM_DATA", key = "stomp_done", value = 0 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_PHASE", phase = PHASE_1 },
        { action = "SET_TIMER", timer_id = TIMER_SLAM, duration = 9000 },
        { action = "SET_CUSTOM_DATA", key = "stomp_done", value = 0 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    local phase = input.phase

    -- Phase 1: At 66% HP, stomp and begin weapon swap
    if phase == PHASE_1 and input.health_pct < 0.66 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SMITE_STOMP, target = "self" })
        table.insert(actions, { action = "YELL", text = "You landlubbers are tougher than I thought! I'll have to improvise!" })
        table.insert(actions, { action = "REMOVE_AURA", spell_id = SPELL_NIMBLE_REFLEXES })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_EQUIP_START })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EQUIP, duration = 2500 })
        table.insert(actions, { action = "STOP_MOVEMENT" })
        table.insert(actions, { action = "SET_MELEE_ATTACK", enabled = false })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "stomp_done", value = 1 })
        return actions
    end

    -- Phase 2: At 33% HP, stomp and begin weapon swap again
    if phase == PHASE_2 and input.health_pct < 0.33 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SMITE_STOMP, target = "self" })
        table.insert(actions, { action = "YELL", text = "D'ah! Now you're making me angry!" })
        table.insert(actions, { action = "REMOVE_AURA", spell_id = SPELL_THRASH })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_EQUIP_START })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EQUIP, duration = 2500 })
        table.insert(actions, { action = "STOP_MOVEMENT" })
        table.insert(actions, { action = "SET_MELEE_ATTACK", enabled = false })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "stomp_done", value = 2 })
        return actions
    end

    -- Equipment change phases (timed sequence)
    if phase == PHASE_EQUIP_START and input:IsTimerReady(TIMER_EQUIP) then
        -- Walk to chest position (simplified: just kneel in place)
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_EQUIP_PROCESS })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EQUIP, duration = 3000 })
        return actions
    end

    if phase == PHASE_EQUIP_PROCESS and input:IsTimerReady(TIMER_EQUIP) then
        local stomp_done = input:GetCustomData("stomp_done") or 0
        if stomp_done == 2 then
            -- Hammer phase
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SMITE_HAMMER, target = "self" })
        end
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_EQUIP_END })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EQUIP, duration = 1000 })
        return actions
    end

    if phase == PHASE_EQUIP_END and input:IsTimerReady(TIMER_EQUIP) then
        local stomp_done = input:GetCustomData("stomp_done") or 0
        if stomp_done == 2 then
            table.insert(actions, { action = "SET_PHASE", phase = PHASE_3 })
        else
            table.insert(actions, { action = "SET_PHASE", phase = PHASE_2 })
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_THRASH, target = "self" })
        end
        table.insert(actions, { action = "SET_MELEE_ATTACK", enabled = true })
        table.insert(actions, { action = "SET_COMBAT_MOVEMENT", enabled = true })
        return actions
    end

    -- Phase 3: Slam ability
    if phase == PHASE_3 then
        if input:IsTimerReady(TIMER_SLAM) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SMITE_SLAM, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SLAM, duration = 11000 })
        end
    end

    return actions
end

RegisterCreatureAI(646, boss)
