--[[
    @script_type: creature_ai
    @entry: 14509
    @name: boss_thekal

    High Priest Thekal - Zul'Gurub
    Ported from MaNGOS boss_thekal.cpp

    Multi-creature script: Thekal (14509), Zealot Lor'Khan (11347), Zealot Zath (11348)

    Phase 1: Troll form with two zealot adds. All three must die within ~10s or they resurrect.
    Phase 2: Tiger form after all three are "dead". Charge, Frenzy, Force Punch, Summon Tigers.
]]

-- Thekal spells
local SPELL_MORTAL_CLEAVE  = 22859
local SPELL_SILENCE        = 23207
local SPELL_FRENZY         = 23342
local SPELL_FORCE_PUNCH    = 24189
local SPELL_CHARGE         = 24408
local SPELL_ENRAGE         = 23537
local SPELL_SUMMON_TIGERS  = 24183
local SPELL_TIGER_FORM     = 24169
local SPELL_RESURRECT      = 24173

-- Zealot Lor'Khan spells
local SPELL_SHIELD         = 25020
local SPELL_BLOODLUST      = 24185
local SPELL_GREATER_HEAL   = 24208
local SPELL_DISARM         = 22691

-- Zealot Zath spells
local SPELL_SWEEPING_STRIKES = 18765
local SPELL_SINISTER_STRIKE  = 15667
local SPELL_GOUGE            = 24698
local SPELL_KICK             = 15614
local SPELL_BLIND            = 21060

-- Thekal timers
local TIMER_MORTAL_CLEAVE  = 1
local TIMER_SILENCE        = 2
local TIMER_FRENZY         = 3
local TIMER_FORCE_PUNCH    = 4
local TIMER_CHARGE         = 5
local TIMER_SUMMON_TIGERS  = 6

-- Lor'Khan timers
local TIMER_LK_SHIELD      = 1
local TIMER_LK_BLOODLUST   = 2
local TIMER_LK_GREATER_HEAL = 3
local TIMER_LK_DISARM      = 4

-- Zath timers
local TIMER_Z_SWEEPING     = 1
local TIMER_Z_SINISTER     = 2
local TIMER_Z_GOUGE        = 3
local TIMER_Z_KICK         = 4
local TIMER_Z_BLIND        = 5

-------------------------------
-- High Priest Thekal
-------------------------------
-- Phase 1 = troll form, Phase 2 = tiger form

local thekal = {}

function thekal:OnEnterCombat(input)
    return {
        { action = "YELL", text = "Hakkar binds my flesh!" },
        { action = "SET_PHASE", phase = 1 },
        { action = "SET_TIMER", timer_id = TIMER_MORTAL_CLEAVE, duration = 4000 },
        { action = "SET_TIMER", timer_id = TIMER_SILENCE, duration = 9000 },
        { action = "SET_TIMER", timer_id = TIMER_FRENZY, duration = 30000 },
        { action = "SET_TIMER", timer_id = TIMER_FORCE_PUNCH, duration = 4000 },
        { action = "SET_TIMER", timer_id = TIMER_CHARGE, duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_SUMMON_TIGERS, duration = 25000 },
    }
end

function thekal:OnReset(input)
    return {
        { action = "SET_PHASE", phase = 1 },
        { action = "DEMORPH" },
    }
end

function thekal:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Transition to tiger phase at very low HP (fake death -> tiger form)
    -- In practice this triggers when all 3 NPCs go to 0 HP
    if input.phase == 1 and input.health_pct < 0.05 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TIGER_FORM, target = "self" })
        table.insert(actions, { action = "SET_PHASE", phase = 2 })
        table.insert(actions, { action = "SET_HEALTH_PERCENT", percent = 100 })
        return actions
    end

    -- Phase 1: Troll form
    if input.phase == 1 then
        -- Mortal Cleave
        if input:IsTimerReady(TIMER_MORTAL_CLEAVE) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MORTAL_CLEAVE, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MORTAL_CLEAVE, duration = 17000 })
        end

        -- Silence
        if input:IsTimerReady(TIMER_SILENCE) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SILENCE, target = "random_hostile" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SILENCE, duration = 22000 })
        end
    end

    -- Phase 2: Tiger form
    if input.phase == 2 then
        -- Charge
        if input:IsTimerReady(TIMER_CHARGE) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CHARGE, target = "random_hostile" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CHARGE, duration = 18000 })
        end

        -- Frenzy
        if input:IsTimerReady(TIMER_FRENZY) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FRENZY, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FRENZY, duration = 30000 })
        end

        -- Force Punch
        if input:IsTimerReady(TIMER_FORCE_PUNCH) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FORCE_PUNCH, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FORCE_PUNCH, duration = 18000 })
        end

        -- Summon Tigers
        if input:IsTimerReady(TIMER_SUMMON_TIGERS) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUMMON_TIGERS, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SUMMON_TIGERS, duration = 12000 })
        end

        -- Enrage below 11% HP
        if input.health_pct < 0.11 then
            if not input:HasAura(SPELL_ENRAGE) then
                table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ENRAGE, target = "self" })
            end
        end
    end

    return actions
end

RegisterCreatureAI(14509, thekal)

-------------------------------
-- Zealot Lor'Khan
-------------------------------
local lorkhan = {}

function lorkhan:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_LK_SHIELD, duration = 1000 },
        { action = "SET_TIMER", timer_id = TIMER_LK_BLOODLUST, duration = 16000 },
        { action = "SET_TIMER", timer_id = TIMER_LK_GREATER_HEAL, duration = 32000 },
        { action = "SET_TIMER", timer_id = TIMER_LK_DISARM, duration = 6000 },
    }
end

function lorkhan:OnReset(input)
    return {}
end

function lorkhan:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Shield
    if input:IsTimerReady(TIMER_LK_SHIELD) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHIELD, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_LK_SHIELD, duration = 61000 })
    end

    -- Bloodlust
    if input:IsTimerReady(TIMER_LK_BLOODLUST) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BLOODLUST, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_LK_BLOODLUST, duration = 24000 })
    end

    -- Greater Heal
    if input:IsTimerReady(TIMER_LK_GREATER_HEAL) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_GREATER_HEAL, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_LK_GREATER_HEAL, duration = 17000 })
    end

    -- Disarm
    if input:IsTimerReady(TIMER_LK_DISARM) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DISARM, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_LK_DISARM, duration = 20000 })
    end

    return actions
end

RegisterCreatureAI(11347, lorkhan)

-------------------------------
-- Zealot Zath
-------------------------------
local zath = {}

function zath:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_Z_SWEEPING, duration = 13000 },
        { action = "SET_TIMER", timer_id = TIMER_Z_SINISTER, duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_Z_GOUGE, duration = 25000 },
        { action = "SET_TIMER", timer_id = TIMER_Z_KICK, duration = 18000 },
        { action = "SET_TIMER", timer_id = TIMER_Z_BLIND, duration = 5000 },
    }
end

function zath:OnReset(input)
    return {}
end

function zath:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Sweeping Strikes
    if input:IsTimerReady(TIMER_Z_SWEEPING) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SWEEPING_STRIKES, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_Z_SWEEPING, duration = 24000 })
    end

    -- Sinister Strike
    if input:IsTimerReady(TIMER_Z_SINISTER) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SINISTER_STRIKE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_Z_SINISTER, duration = 12000 })
    end

    -- Gouge
    if input:IsTimerReady(TIMER_Z_GOUGE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_GOUGE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_Z_GOUGE, duration = 22000 })
    end

    -- Kick
    if input:IsTimerReady(TIMER_Z_KICK) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_KICK, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_Z_KICK, duration = 20000 })
    end

    -- Blind
    if input:IsTimerReady(TIMER_Z_BLIND) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BLIND, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_Z_BLIND, duration = 15000 })
    end

    return actions
end

RegisterCreatureAI(11348, zath)
