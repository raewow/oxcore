--[[
    @script_type: creature_ai
    @entry: 10508
    @name: boss_ras_frostwhisper

    Ras Frostwhisper - Scholomance
    No dedicated MaNGOS SD2 script exists; created from known Vanilla spell data.

    Mechanics:
    - Frost Bolt on current target (single target nuke)
    - Ice Armor self-buff
    - Freeze (AoE frost nova)
    - Fear on random hostile
    - Chill Nova (AoE damage + slow)
    - Frostbolt Volley (AoE)
]]

local SPELL_FROSTBOLT        = 21369
local SPELL_ICE_ARMOR        = 18100
local SPELL_FREEZE           = 18763
local SPELL_FEAR             = 26070
local SPELL_CHILL_NOVA       = 18099
local SPELL_FROSTBOLT_VOLLEY = 8398

local TIMER_FROSTBOLT        = 1
local TIMER_ICE_ARMOR        = 2
local TIMER_FREEZE           = 3
local TIMER_FEAR             = 4
local TIMER_CHILL_NOVA       = 5
local TIMER_FROSTBOLT_VOLLEY = 6

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "CAST_SPELL", spell_id = SPELL_ICE_ARMOR, target = "self" },
        { action = "SET_TIMER", timer_id = TIMER_FROSTBOLT, duration = 2000 },
        { action = "SET_TIMER", timer_id = TIMER_ICE_ARMOR, duration = 60000 },
        { action = "SET_TIMER", timer_id = TIMER_FREEZE, duration = 18000 },
        { action = "SET_TIMER", timer_id = TIMER_FEAR, duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_CHILL_NOVA, duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_FROSTBOLT_VOLLEY, duration = 20000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Frostbolt
    if input:IsTimerReady(TIMER_FROSTBOLT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FROSTBOLT, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FROSTBOLT, duration = 5000 })
    end

    -- Ice Armor refresh
    if input:IsTimerReady(TIMER_ICE_ARMOR) then
        if not input:HasAura(SPELL_ICE_ARMOR) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ICE_ARMOR, target = "self" })
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ICE_ARMOR, duration = 60000 })
    end

    -- Freeze (frost nova)
    if input:IsTimerReady(TIMER_FREEZE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FREEZE, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FREEZE, duration = 18000 })
    end

    -- Fear
    if input:IsTimerReady(TIMER_FEAR) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FEAR, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FEAR, duration = 20000 })
    end

    -- Chill Nova
    if input:IsTimerReady(TIMER_CHILL_NOVA) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CHILL_NOVA, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CHILL_NOVA, duration = 14000 })
    end

    -- Frostbolt Volley
    if input:IsTimerReady(TIMER_FROSTBOLT_VOLLEY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FROSTBOLT_VOLLEY, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FROSTBOLT_VOLLEY, duration = 22000 })
    end

    return actions
end

RegisterCreatureAI(10508, boss)
