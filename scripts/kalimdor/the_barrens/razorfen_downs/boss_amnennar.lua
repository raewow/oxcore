--[[
    @script_type: creature_ai
    @entry: 7358
    @name: boss_amnennar

    Amnennar the Coldbringer - Razorfen Downs
    Ported from MaNGOS boss_amnennar_the_coldbringer.cpp

    Abilities:
    - Amnennar's Wrath: Shadow damage on current target
    - Frostbolt: Periodic frost bolt
    - Frost Nova: Periodic AoE freeze
    - Frost Spectres: Summons frost spectres at 60% and 30% HP
    - Yell at 50% HP
]]

local SPELL_AMNENNARS_WRATH  = 13009
local SPELL_FROSTBOLT        = 15530
local SPELL_FROST_NOVA       = 15531
local SPELL_FROST_SPECTRES   = 12642

local TIMER_WRATH            = 1
local TIMER_FROSTBOLT        = 2
local TIMER_FROST_NOVA       = 3

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "YELL", text = "You'll never leave this place!" },
        { action = "SET_TIMER", timer_id = TIMER_WRATH, duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_FROSTBOLT, duration = 1000 },
        { action = "SET_TIMER", timer_id = TIMER_FROST_NOVA, duration = math.random(10000, 15000) },
        { action = "SET_CUSTOM_DATA", key = "spectres_60", value = 0 },
        { action = "SET_CUSTOM_DATA", key = "spectres_30", value = 0 },
        { action = "SET_CUSTOM_DATA", key = "yell_50", value = 0 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_WRATH, duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_FROSTBOLT, duration = 1000 },
        { action = "SET_TIMER", timer_id = TIMER_FROST_NOVA, duration = math.random(10000, 15000) },
        { action = "SET_CUSTOM_DATA", key = "spectres_60", value = 0 },
        { action = "SET_CUSTOM_DATA", key = "spectres_30", value = 0 },
        { action = "SET_CUSTOM_DATA", key = "yell_50", value = 0 },
    }
end

function boss:OnKill(input)
    return {
        { action = "SAY", text = "Too easy!" },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Amnennar's Wrath
    if input:IsTimerReady(TIMER_WRATH) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_AMNENNARS_WRATH, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WRATH, duration = 12000 })
    end

    -- Frostbolt
    if input:IsTimerReady(TIMER_FROSTBOLT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FROSTBOLT, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FROSTBOLT, duration = 8000 })
    end

    -- Frost Nova
    if input:IsTimerReady(TIMER_FROST_NOVA) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FROST_NOVA, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FROST_NOVA, duration = 15000 })
    end

    -- Frost Spectres at 60% HP
    local spectres_60 = input:GetCustomData("spectres_60") or 0
    if spectres_60 == 0 and input.health_pct < 0.60 then
        table.insert(actions, { action = "YELL", text = "Come, spirits, attend your master!" })
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FROST_SPECTRES, target = "current_target" })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "spectres_60", value = 1 })
    end

    -- Yell at 50% HP
    local yell_50 = input:GetCustomData("yell_50") or 0
    if yell_50 == 0 and input.health_pct < 0.50 then
        table.insert(actions, { action = "YELL", text = "I am the hand of the Lich King!" })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "yell_50", value = 1 })
    end

    -- Frost Spectres at 30% HP
    local spectres_30 = input:GetCustomData("spectres_30") or 0
    if spectres_30 == 0 and input.health_pct < 0.30 then
        table.insert(actions, { action = "YELL", text = "To me, my servants!" })
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FROST_SPECTRES, target = "current_target" })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "spectres_30", value = 1 })
    end

    return actions
end

RegisterCreatureAI(7358, boss)
