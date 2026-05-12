--[[
    @script_type: creature_ai
    @entry: 7800
    @name: boss_thermaplugg

    Mekgineer Thermaplugg - Gnomeregan
    Ported from MaNGOS boss_thermaplugg.cpp

    Phase 1: Knock Away (single target), Activate Bomb periodically
    Phase 2 (below 50% HP): Knock Away AOE, faster bomb activation
    Bomb faces spawn Walking Bomb NPCs that follow Thermaplugg
]]

local SPELL_KNOCK_AWAY       = 10101
local SPELL_KNOCK_AWAY_AOE   = 11130
local SPELL_ACTIVATE_BOMB_A  = 11511
local SPELL_ACTIVATE_BOMB_B  = 11795

local TIMER_KNOCK_AWAY       = 1
local TIMER_ACTIVATE_BOMB    = 2

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "YELL", text = "Usurpers! Gnomeregan is mine!" },
        { action = "SET_TIMER", timer_id = TIMER_KNOCK_AWAY, duration = math.random(17000, 20000) },
        { action = "SET_TIMER", timer_id = TIMER_ACTIVATE_BOMB, duration = math.random(10000, 15000) },
        { action = "SET_CUSTOM_DATA", key = "phase_two", value = 0 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_KNOCK_AWAY, duration = math.random(17000, 20000) },
        { action = "SET_TIMER", timer_id = TIMER_ACTIVATE_BOMB, duration = math.random(10000, 15000) },
        { action = "SET_CUSTOM_DATA", key = "phase_two", value = 0 },
    }
end

function boss:OnKill(input)
    return {
        { action = "SAY", text = "My, my... what have you gotten yourself into?" },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    local phase_two = input:GetCustomData("phase_two") or 0

    -- Transition to phase 2 at 50% HP
    if phase_two == 0 and input.health_pct < 0.50 then
        table.insert(actions, { action = "YELL", text = "Now you'll see the full power of my creation!" })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "phase_two", value = 1 })
    end

    -- Knock Away
    if input:IsTimerReady(TIMER_KNOCK_AWAY) then
        if phase_two == 1 then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_KNOCK_AWAY_AOE, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_KNOCK_AWAY, duration = 12000 })
        else
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_KNOCK_AWAY, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_KNOCK_AWAY, duration = math.random(17000, 20000) })
        end
    end

    -- Activate Bomb
    if input:IsTimerReady(TIMER_ACTIVATE_BOMB) then
        if phase_two == 1 then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ACTIVATE_BOMB_B, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ACTIVATE_BOMB, duration = math.random(6000, 12000) })
        else
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ACTIVATE_BOMB_A, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ACTIVATE_BOMB, duration = math.random(12000, 17000) })
        end
    end

    return actions
end

RegisterCreatureAI(7800, boss)
