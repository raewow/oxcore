--[[
    @script_type: creature_ai
    @entry: 3976
    @name: boss_mograine

    Scarlet Commander Mograine - Scarlet Monastery (Cathedral)
    Ported from MaNGOS boss_mograine_and_whitemane.cpp

    Mograine fights first. When killed, he fake-deaths and Whitemane enters.
    Whitemane casts Deep Sleep at 50% HP, then resurrects Mograine.
    After resurrection, Mograine casts Lay on Hands on Whitemane and both fight.

    Mograine abilities:
    - Retribution Aura (on aggro)
    - Crusader Strike (periodic)
    - Hammer of Justice (periodic, long CD)

    On fake death: becomes non-attackable, Whitemane engages
    On resurrection: resumes fighting with Lay on Hands on Whitemane
]]

local SPELL_CRUSADER_STRIKE   = 14518
local SPELL_HAMMER_OF_JUSTICE = 5589
local SPELL_LAY_ON_HANDS      = 9257
local SPELL_RETRIBUTION_AURA  = 8990

local TIMER_CRUSADER_STRIKE   = 1
local TIMER_HAMMER_OF_JUSTICE = 2
local TIMER_RESURRECT_DELAY   = 3

local PHASE_FIGHTING          = 1
local PHASE_FAKE_DEATH        = 2
local PHASE_RESURRECTED       = 3

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "YELL", text = "Infidels! They must be purified!" },
        { action = "CAST_SPELL", spell_id = SPELL_RETRIBUTION_AURA, target = "self" },
        { action = "SET_TIMER", timer_id = TIMER_CRUSADER_STRIKE, duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_HAMMER_OF_JUSTICE, duration = 10000 },
        { action = "SET_PHASE", phase = PHASE_FIGHTING },
        { action = "SET_CUSTOM_DATA", key = "fake_death", value = 0 },
        { action = "SET_CUSTOM_DATA", key = "healed_whitemane", value = 0 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_CRUSADER_STRIKE, duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_HAMMER_OF_JUSTICE, duration = 10000 },
        { action = "SET_PHASE", phase = PHASE_FIGHTING },
        { action = "SET_CUSTOM_DATA", key = "fake_death", value = 0 },
        { action = "SET_CUSTOM_DATA", key = "healed_whitemane", value = 0 },
        { action = "REMOVE_UNIT_FLAG", flag = "NON_ATTACKABLE" },
    }
end

function boss:OnKill(input)
    return {
        { action = "SAY", text = "Unworthy." },
    }
end

function boss:OnDamageTaken(input)
    local actions = {}
    local fake_death = input:GetCustomData("fake_death") or 0

    -- Fake death when HP would reach 0 (first time only)
    if fake_death == 0 and input.health_pct < 0.05 then
        table.insert(actions, { action = "SET_HEALTH_PCT", percent = 1 })
        table.insert(actions, { action = "SET_UNIT_FLAG", flag = "NON_ATTACKABLE" })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_FAKE_DEATH })
        table.insert(actions, { action = "SET_MELEE_ATTACK", enabled = false })
        table.insert(actions, { action = "STOP_MOVEMENT" })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "fake_death", value = 1 })
    end

    return actions
end

function boss:OnSpellHit(input)
    local actions = {}

    -- When hit with Scarlet Resurrection (9232), come back to life
    if input.spell_id == 9232 then
        table.insert(actions, { action = "YELL", text = "At your side, milady!" })
        table.insert(actions, { action = "REMOVE_UNIT_FLAG", flag = "NON_ATTACKABLE" })
        table.insert(actions, { action = "SET_MELEE_ATTACK", enabled = true })
        table.insert(actions, { action = "SET_COMBAT_MOVEMENT", enabled = true })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_RESURRECTED })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CRUSADER_STRIKE, duration = 10000 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_HAMMER_OF_JUSTICE, duration = 10000 })
    end

    return actions
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- During fake death, do nothing
    if input.phase == PHASE_FAKE_DEATH then
        return actions
    end

    -- After resurrection, heal Whitemane once with Lay on Hands
    if input.phase == PHASE_RESURRECTED then
        local healed = input:GetCustomData("healed_whitemane") or 0
        if healed == 0 then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_LAY_ON_HANDS, target = "current_target" })
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = "healed_whitemane", value = 1 })
        end
    end

    -- Crusader Strike
    if input:IsTimerReady(TIMER_CRUSADER_STRIKE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CRUSADER_STRIKE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CRUSADER_STRIKE, duration = 10000 })
    end

    -- Hammer of Justice
    if input:IsTimerReady(TIMER_HAMMER_OF_JUSTICE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_HAMMER_OF_JUSTICE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_HAMMER_OF_JUSTICE, duration = 60000 })
    end

    return actions
end

RegisterCreatureAI(3976, boss)

------------------------------------------------------------------------
-- High Inquisitor Whitemane (entry 3977)
------------------------------------------------------------------------

--[[
    @script_type: creature_ai
    @entry: 3977
    @name: boss_whitemane

    High Inquisitor Whitemane - Scarlet Monastery (Cathedral)

    Whitemane enters after Mograine fake-dies.
    Abilities:
    - Holy Smite (periodic)
    - Power Word: Shield (periodic, on self)
    - Heal (periodic, on self or Mograine if low HP)
    - Deep Sleep (at 50% HP, one-time) - sleeps all players
    - Scarlet Resurrection (after Deep Sleep) - resurrects Mograine
]]

local WM_SPELL_DEEP_SLEEP     = 9256
local WM_SPELL_SCARLET_RES    = 9232
local WM_SPELL_DOMINATE_MIND  = 14515
local WM_SPELL_HOLY_SMITE     = 9481
local WM_SPELL_HEAL           = 12039
local WM_SPELL_PW_SHIELD      = 22187

local WM_TIMER_HOLY_SMITE     = 1
local WM_TIMER_HEAL           = 2
local WM_TIMER_PW_SHIELD      = 3
local WM_TIMER_RESURRECT      = 4

local WM_PHASE_NORMAL         = 1
local WM_PHASE_RESURRECT      = 2
local WM_PHASE_POST_RES       = 3

local whitemane = {}

function whitemane:OnEnterCombat(input)
    return {
        { action = "YELL", text = "Mograine has fallen? You shall pay for this treachery!" },
        { action = "SET_TIMER", timer_id = WM_TIMER_HOLY_SMITE, duration = 6000 },
        { action = "SET_TIMER", timer_id = WM_TIMER_HEAL, duration = 10000 },
        { action = "SET_TIMER", timer_id = WM_TIMER_PW_SHIELD, duration = 15000 },
        { action = "SET_PHASE", phase = WM_PHASE_NORMAL },
        { action = "SET_CUSTOM_DATA", key = "can_resurrect", value = 0 },
    }
end

function whitemane:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = WM_TIMER_HOLY_SMITE, duration = 6000 },
        { action = "SET_TIMER", timer_id = WM_TIMER_HEAL, duration = 10000 },
        { action = "SET_TIMER", timer_id = WM_TIMER_PW_SHIELD, duration = 15000 },
        { action = "SET_PHASE", phase = WM_PHASE_NORMAL },
        { action = "SET_CUSTOM_DATA", key = "can_resurrect", value = 0 },
    }
end

function whitemane:OnKill(input)
    return {
        { action = "SAY", text = "The light has spoken!" },
    }
end

function whitemane:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Resurrect phase: wait then cast resurrection on Mograine
    if input.phase == WM_PHASE_RESURRECT then
        if input:IsTimerReady(WM_TIMER_RESURRECT) then
            table.insert(actions, { action = "SAY", text = "Arise, my champion!" })
            table.insert(actions, { action = "CAST_SPELL", spell_id = WM_SPELL_SCARLET_RES, target = "current_target" })
            table.insert(actions, { action = "SET_PHASE", phase = WM_PHASE_POST_RES })
        end
        return actions
    end

    -- Deep Sleep at 50% HP (one-time) -> begin resurrect sequence
    local can_res = input:GetCustomData("can_resurrect") or 0
    if can_res == 0 and input.health_pct <= 0.50 then
        table.insert(actions, { action = "INTERRUPT_SPELL" })
        table.insert(actions, { action = "CAST_SPELL", spell_id = WM_SPELL_DEEP_SLEEP, target = "current_target" })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "can_resurrect", value = 1 })
        table.insert(actions, { action = "SET_PHASE", phase = WM_PHASE_RESURRECT })
        table.insert(actions, { action = "SET_TIMER", timer_id = WM_TIMER_RESURRECT, duration = 7000 })
        return actions
    end

    -- Holy Smite
    if input:IsTimerReady(WM_TIMER_HOLY_SMITE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = WM_SPELL_HOLY_SMITE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = WM_TIMER_HOLY_SMITE, duration = 6000 })
    end

    -- Heal (self or ally if low HP)
    if input:IsTimerReady(WM_TIMER_HEAL) then
        if input.health_pct <= 0.75 then
            table.insert(actions, { action = "CAST_SPELL", spell_id = WM_SPELL_HEAL, target = "self" })
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = WM_TIMER_HEAL, duration = 13000 })
    end

    -- Power Word: Shield
    if input:IsTimerReady(WM_TIMER_PW_SHIELD) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = WM_SPELL_PW_SHIELD, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = WM_TIMER_PW_SHIELD, duration = 15000 })
    end

    return actions
end

RegisterCreatureAI(3977, whitemane)
