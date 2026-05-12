--[[
    @script_type: creature_ai
    @entry: 7273
    @name: boss_gahzrilla

    Gahz'rilla - Zul'Farrak
    No dedicated MaNGOS SD2 boss script exists (only gong summon event);
    created from known Vanilla spell data.

    Mechanics:
    - Gahz'rilla Slam (AoE knockback + damage)
    - Freeze Solid (frost damage + stun)
    - Massive Geyser (AoE water damage + knockback)
]]

local SPELL_GAHZRILLA_SLAM = 11902
local SPELL_FREEZE_SOLID   = 11836
local SPELL_MASSIVE_GEYSER = 21403

local TIMER_SLAM    = 1
local TIMER_FREEZE  = 2
local TIMER_GEYSER  = 3

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_SLAM, duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_FREEZE, duration = 16000 },
        { action = "SET_TIMER", timer_id = TIMER_GEYSER, duration = 22000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Gahz'rilla Slam
    if input:IsTimerReady(TIMER_SLAM) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_GAHZRILLA_SLAM, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SLAM, duration = 15000 })
    end

    -- Freeze Solid
    if input:IsTimerReady(TIMER_FREEZE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FREEZE_SOLID, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FREEZE, duration = 18000 })
    end

    -- Massive Geyser
    if input:IsTimerReady(TIMER_GEYSER) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MASSIVE_GEYSER, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_GEYSER, duration = 25000 })
    end

    return actions
end

RegisterCreatureAI(7273, boss)
