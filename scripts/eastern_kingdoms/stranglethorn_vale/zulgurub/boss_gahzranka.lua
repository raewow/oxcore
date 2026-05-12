--[[
    @script_type: creature_ai
    @entry: 15114
    @name: boss_gahzranka

    Gahz'ranka - Zul'Gurub
    Verified against vmangos boss_gahzranka.cpp
]]

local SPELL_FROSTBREATH  = 16099
local SPELL_MASSIVEGEYSER = 22421  -- resets threat after cast
local SPELL_SLAM         = 24326

local TIMER_FROSTBREATH   = 1
local TIMER_MASSIVEGEYSER = 2
local TIMER_SLAM          = 3

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_FROSTBREATH,   duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_MASSIVEGEYSER, duration = 25000 },
        { action = "SET_TIMER", timer_id = TIMER_SLAM,          duration = 17000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_FROSTBREATH,   duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_MASSIVEGEYSER, duration = 25000 },
        { action = "SET_TIMER", timer_id = TIMER_SLAM,          duration = 17000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Frost Breath (AoE cone self); repeat every 8-20s
    if input:IsTimerReady(TIMER_FROSTBREATH) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FROSTBREATH, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FROSTBREATH, duration = math.random(8000, 20000) })
    end

    -- Massive Geyser on random target; resets threat after; repeat every 16-24s
    if input:IsTimerReady(TIMER_MASSIVEGEYSER) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MASSIVEGEYSER, target = "random_hostile" })
        table.insert(actions, { action = "RESET_THREAT" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MASSIVEGEYSER, duration = math.random(16000, 24000) })
    end

    -- Slam on current target; repeat every 12-20s
    if input:IsTimerReady(TIMER_SLAM) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SLAM, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SLAM, duration = math.random(12000, 20000) })
    end

    return actions
end

RegisterCreatureAI(15114, boss)
