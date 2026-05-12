--[[
    @script_type: creature_ai
    @entry: 15516, 15984
    @name: boss_sartura

    Battleguard Sartura - Temple of Ahn'Qiraj
    Verified against vmangos boss_sartura.cpp

    Sartura (15516):
    - Whirlwind: urand(8,12)s init, 15s duration, random target switching every urand(1,2)s
    - After WW: next WW in urand(5,10)s; aggro reset back to urand(3,7)s
    - Cleave: 4s init, repeat urand(3,4)s (not during WW)
    - Aggro reset: urand(5,7.5)s init; non-WW urand(3,7)s; WW urand(1,2)s
    - Enrage at 20% HP (SPELL_ENRAGE 26527)
    - Hard Enrage after 10 minutes (SPELL_ENRAGEHARD 27680)

    Royal Guard (15984):
    - Whirlwind: urand(8,10)s init, 8s duration, random targets every urand(1,2)s
    - After WW: next WW in urand(2,6)s
    - Knockback: urand(6,12)s init, repeat urand(8,14)s
    - Aggro reset: urand(5,7.5)s
]]

local SPELL_WHIRLWIND       = 26083
local SPELL_CLEAVE          = 25174   -- was missing
local SPELL_ENRAGE          = 26527   -- was 28747 (wrong)
local SPELL_ENRAGEHARD      = 27680   -- was 28798 (wrong)

local SPELL_GUARD_WHIRLWIND  = 26038
local SPELL_KNOCKBACK        = 19813   -- was 26027 (wrong)

-- Sartura timers
local TIMER_WHIRLWIND       = 1
local TIMER_WHIRLWIND_END   = 2
local TIMER_AGGRO_RESET     = 3
local TIMER_HARD_ENRAGE     = 4
local TIMER_CLEAVE          = 5

-- Guard timers (same IDs, separate AI)
local TIMER_GUARD_WW        = 1
local TIMER_GUARD_WW_END    = 2
local TIMER_GUARD_AGGRO     = 3
local TIMER_GUARD_KNOCKBACK = 4

--------------------------------------------------------------------------------
-- Battleguard Sartura (15516)
--------------------------------------------------------------------------------
local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SCRIPT_TEXT", text_id = 11442 },  -- SAY_AGGRO
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_WHIRLWIND,   duration = math.random(8000, 12000) },
        { action = "SET_TIMER", timer_id = TIMER_AGGRO_RESET, duration = math.random(5000, 7500) },
        { action = "SET_TIMER", timer_id = TIMER_HARD_ENRAGE, duration = 600000 },
        { action = "SET_TIMER", timer_id = TIMER_CLEAVE,      duration = 4000 },
        { action = "SET_CUSTOM_DATA", key = "enraged",      value = 0 },
        { action = "SET_CUSTOM_DATA", key = "hard_enraged", value = 0 },
        { action = "SET_CUSTOM_DATA", key = "whirlwind",    value = 0 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_WHIRLWIND,   duration = math.random(8000, 12000) },
        { action = "SET_TIMER", timer_id = TIMER_AGGRO_RESET, duration = math.random(5000, 7500) },
        { action = "SET_TIMER", timer_id = TIMER_HARD_ENRAGE, duration = 600000 },
        { action = "SET_TIMER", timer_id = TIMER_CLEAVE,      duration = 4000 },
        { action = "SET_CUSTOM_DATA", key = "enraged",      value = 0 },
        { action = "SET_CUSTOM_DATA", key = "hard_enraged", value = 0 },
        { action = "SET_CUSTOM_DATA", key = "whirlwind",    value = 0 },
    }
end

function boss:OnKilledUnit(input)
    return { { action = "SCRIPT_TEXT", text_id = 11443 } }  -- SAY_SLAY
end

function boss:OnDeath(input)
    return { { action = "SCRIPT_TEXT", text_id = 11444 } }  -- SAY_DEATH
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    local ww = input:GetCustomData("whirlwind") or 0

    if ww == 1 then
        -- During whirlwind: random target switching every urand(1,2)s
        if input:IsTimerReady(TIMER_AGGRO_RESET) then
            table.insert(actions, { action = "RESET_THREAT" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_AGGRO_RESET, duration = math.random(1000, 2000) })
        end

        -- End whirlwind after 15s
        if input:IsTimerReady(TIMER_WHIRLWIND_END) then
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = "whirlwind", value = 0 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WHIRLWIND,   duration = math.random(5000, 10000) })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_AGGRO_RESET, duration = math.random(3000, 7000) })
        end
    else
        -- Start whirlwind
        if input:IsTimerReady(TIMER_WHIRLWIND) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_WHIRLWIND, target = "self" })
            table.insert(actions, { action = "RESET_THREAT" })
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = "whirlwind", value = 1 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WHIRLWIND_END, duration = 15000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_AGGRO_RESET,   duration = math.random(1000, 2000) })
        end

        -- Random target switching outside WW
        if input:IsTimerReady(TIMER_AGGRO_RESET) then
            table.insert(actions, { action = "RESET_THREAT" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_AGGRO_RESET, duration = math.random(3000, 7000) })
        end

        -- Cleave (not during WW)
        if input:IsTimerReady(TIMER_CLEAVE) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CLEAVE, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = math.random(3000, 4000) })
        end
    end

    -- Enrage at 20% HP
    local enraged = input:GetCustomData("enraged") or 0
    if enraged == 0 and input.health_pct and input.health_pct <= 0.20 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ENRAGE, target = "self" })
        table.insert(actions, { action = "EMOTE", emote_id = 2384 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "enraged", value = 1 })
    end

    -- Hard enrage after 10 minutes
    local hard_enraged = input:GetCustomData("hard_enraged") or 0
    if hard_enraged == 0 and input:IsTimerReady(TIMER_HARD_ENRAGE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ENRAGEHARD, target = "self" })
        table.insert(actions, { action = "EMOTE", emote_id = 4428 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "hard_enraged", value = 1 })
    end

    return actions
end

RegisterCreatureAI(15516, boss)

--------------------------------------------------------------------------------
-- Sartura's Royal Guard (15984)
--------------------------------------------------------------------------------
local guard = {}

function guard:OnEnterCombat(input)
    return {
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_GUARD_WW,        duration = math.random(8000, 10000) },
        { action = "SET_TIMER", timer_id = TIMER_GUARD_KNOCKBACK,  duration = math.random(6000, 12000) },
        { action = "SET_TIMER", timer_id = TIMER_GUARD_AGGRO,      duration = math.random(5000, 7500) },
        { action = "SET_CUSTOM_DATA", key = "whirlwind", value = 0 },
    }
end

function guard:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_GUARD_WW,        duration = math.random(8000, 10000) },
        { action = "SET_TIMER", timer_id = TIMER_GUARD_KNOCKBACK,  duration = math.random(6000, 12000) },
        { action = "SET_TIMER", timer_id = TIMER_GUARD_AGGRO,      duration = math.random(5000, 7500) },
        { action = "SET_CUSTOM_DATA", key = "whirlwind", value = 0 },
    }
end

function guard:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    local ww = input:GetCustomData("whirlwind") or 0

    if ww == 1 then
        -- Random target switching during WW
        if input:IsTimerReady(TIMER_GUARD_AGGRO) then
            table.insert(actions, { action = "RESET_THREAT" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_GUARD_AGGRO, duration = math.random(1000, 2000) })
        end

        -- End WW after 8s
        if input:IsTimerReady(TIMER_GUARD_WW_END) then
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = "whirlwind", value = 0 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_GUARD_WW,   duration = math.random(2000, 6000) })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_GUARD_AGGRO, duration = math.random(3000, 7000) })
        end
    else
        -- Start WW
        if input:IsTimerReady(TIMER_GUARD_WW) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_GUARD_WHIRLWIND, target = "self" })
            table.insert(actions, { action = "RESET_THREAT" })
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = "whirlwind", value = 1 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_GUARD_WW_END, duration = 8000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_GUARD_AGGRO,  duration = math.random(1000, 2000) })
        end

        -- Random target
        if input:IsTimerReady(TIMER_GUARD_AGGRO) then
            table.insert(actions, { action = "RESET_THREAT" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_GUARD_AGGRO, duration = math.random(3000, 7000) })
        end

        -- Knockback
        if input:IsTimerReady(TIMER_GUARD_KNOCKBACK) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_KNOCKBACK, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_GUARD_KNOCKBACK, duration = math.random(8000, 14000) })
        end
    end

    return actions
end

RegisterCreatureAI(15984, guard)
