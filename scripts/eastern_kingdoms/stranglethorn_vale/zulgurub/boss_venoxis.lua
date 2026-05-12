--[[
    @script_type: creature_ai
    @entry: 14507
    @name: boss_venoxis

    High Priest Venoxis - Zul'Gurub
    Verified against vmangos boss_venoxis.cpp

    Phase 1 (troll, >50% HP): HolyNova, Dispel, HolyFire, Renew, HolyWrath
    Phase 2 (snake, <=50% HP): PoisonCloud, Trash, VenomSpit, Parasitic
    Frenzy (SPELL_FRENZY 8269) at <20% HP
]]

local SPELL_HOLY_NOVA    = 23858
local SPELL_DISPELL      = 23859
local SPELL_HOLY_FIRE    = 23860
local SPELL_RENEW        = 23895
local SPELL_HOLY_WRATH   = 23979
local SPELL_SNAKE_FORM   = 23849
local SPELL_TRASH        = 3391
local SPELL_POISON_CLOUD = 23861
local SPELL_VENOMSPIT    = 23862
local SPELL_FRENZY       = 8269   -- was 23537 (wrong)
local SPELL_PARASITIC    = 23865

local TIMER_HOLY_NOVA    = 1
local TIMER_DISPELL      = 2
local TIMER_HOLY_FIRE    = 3
local TIMER_RENEW        = 4
local TIMER_HOLY_WRATH   = 5
local TIMER_POISON_CLOUD = 6
local TIMER_TRASH        = 7
local TIMER_VENOM_SPIT   = 8
local TIMER_PARASITIC    = 9

local PHASE_TROLL = 1
local PHASE_SNAKE = 2

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_PHASE", phase = PHASE_TROLL },
        { action = "SET_TIMER", timer_id = TIMER_HOLY_NOVA,    duration = 7500 },
        { action = "SET_TIMER", timer_id = TIMER_DISPELL,      duration = 35000 },
        { action = "SET_TIMER", timer_id = TIMER_HOLY_FIRE,    duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_RENEW,        duration = 30500 },
        { action = "SET_TIMER", timer_id = TIMER_HOLY_WRATH,   duration = 30000 },
        { action = "SET_TIMER", timer_id = TIMER_POISON_CLOUD, duration = 2000 },
        { action = "SET_TIMER", timer_id = TIMER_TRASH,        duration = 5000 },
        { action = "SET_TIMER", timer_id = TIMER_VENOM_SPIT,   duration = 5500 },
        { action = "SET_TIMER", timer_id = TIMER_PARASITIC,    duration = 10000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_PHASE", phase = PHASE_TROLL },
        { action = "SET_TIMER", timer_id = TIMER_HOLY_NOVA,    duration = 7500 },
        { action = "SET_TIMER", timer_id = TIMER_DISPELL,      duration = 35000 },
        { action = "SET_TIMER", timer_id = TIMER_HOLY_FIRE,    duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_RENEW,        duration = 30500 },
        { action = "SET_TIMER", timer_id = TIMER_HOLY_WRATH,   duration = 30000 },
        { action = "SET_TIMER", timer_id = TIMER_POISON_CLOUD, duration = 2000 },
        { action = "SET_TIMER", timer_id = TIMER_TRASH,        duration = 5000 },
        { action = "SET_TIMER", timer_id = TIMER_VENOM_SPIT,   duration = 5500 },
        { action = "SET_TIMER", timer_id = TIMER_PARASITIC,    duration = 10000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Phase transition at 50% HP
    if input.phase == PHASE_TROLL and input.health_pct and input.health_pct < 0.50 then
        table.insert(actions, { action = "SCRIPT_TEXT", text_id = 10026 })  -- SAY_TRANSFORM
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SNAKE_FORM, target = "self" })
        table.insert(actions, { action = "RESET_THREAT" })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_SNAKE })
        return actions
    end

    -- Frenzy at 20% HP (any phase)
    if input.health_pct and input.health_pct < 0.20 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FRENZY, target = "self" })
    end

    -- Phase 1: Troll form
    if input.phase == PHASE_TROLL then
        if input:IsTimerReady(TIMER_HOLY_NOVA) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_HOLY_NOVA, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_HOLY_NOVA, duration = math.random(14000, 16000) })
        end

        if input:IsTimerReady(TIMER_DISPELL) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DISPELL, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_DISPELL, duration = math.random(16000, 18000) })
        end

        if input:IsTimerReady(TIMER_HOLY_FIRE) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_HOLY_FIRE, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_HOLY_FIRE, duration = math.random(8000, 12000) })
        end

        if input:IsTimerReady(TIMER_RENEW) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_RENEW, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_RENEW, duration = math.random(20000, 22000) })
        end

        if input:IsTimerReady(TIMER_HOLY_WRATH) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_HOLY_WRATH, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_HOLY_WRATH, duration = math.random(15000, 25000) })
        end
    end

    -- Phase 2: Snake form
    if input.phase == PHASE_SNAKE then
        if input:IsTimerReady(TIMER_POISON_CLOUD) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_POISON_CLOUD, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_POISON_CLOUD, duration = math.random(7000, 10000) })
        end

        if input:IsTimerReady(TIMER_TRASH) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TRASH, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TRASH, duration = math.random(10000, 20000) })
        end

        if input:IsTimerReady(TIMER_VENOM_SPIT) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_VENOMSPIT, target = "random_hostile" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_VENOM_SPIT, duration = math.random(15000, 20000) })
        end

        if input:IsTimerReady(TIMER_PARASITIC) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_PARASITIC, target = "random_hostile" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PARASITIC, duration = 10000 })
        end
    end

    return actions
end

function boss:OnDeath(input)
    return { { action = "SCRIPT_TEXT", text_id = 10460 } }  -- SAY_DEATH
end

RegisterCreatureAI(14507, boss)
