--[[
    @script_type: creature_ai
    @entry: 15263
    @name: boss_skeram

    The Prophet Skeram - Temple of Ahn'Qiraj
    Verified against vmangos boss_skeram.cpp

    Abilities:
    - Arcane Explosion: AoE when >4 players in melee range; repeat urand(6,14)s
    - Earth Shock: Ranged spam on current target at range (every 2s)
    - True Fulfillment: Mind control nearest player; repeat urand(20.5,25)s
    - Blink: One of 3 spells (4801/8195/20449) to teleport to platform; urand(10,18)s
    - Split: At 75%, 50%, 25% HP casts SPELL_SUMMON_IMAGES (747) spawning 2 images
]]

local SPELL_ARCANE_EXPLOSION    = 26192  -- was 25679 (wrong)
local SPELL_EARTH_SHOCK         = 26194
local SPELL_TRUE_FULFILLMENT    = 785    -- was 26526 (wrong; that is TF_IMMUNITY buff)
local SPELL_BLINK_0             = 4801   -- teleport to platform 0
local SPELL_BLINK_1             = 8195   -- teleport to platform 1
local SPELL_BLINK_2             = 20449  -- teleport to platform 2
local SPELL_SUMMON_IMAGES       = 747

local TIMER_ARCANE_EXPLOSION    = 1
local TIMER_EARTH_SHOCK         = 2
local TIMER_FULFILLMENT         = 3
local TIMER_BLINK               = 4

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SCRIPT_TEXT", text_id = 11445 },  -- SAY_AGGRO (one of 3 aggro texts)
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_ARCANE_EXPLOSION, duration = math.random(6000, 8000) },
        { action = "SET_TIMER", timer_id = TIMER_EARTH_SHOCK,      duration = 2000 },
        { action = "SET_TIMER", timer_id = TIMER_FULFILLMENT,      duration = math.random(10000, 15000) },
        { action = "SET_TIMER", timer_id = TIMER_BLINK,            duration = math.random(15000, 20000) },
        { action = "SET_CUSTOM_DATA", key = "split_75", value = 0 },
        { action = "SET_CUSTOM_DATA", key = "split_50", value = 0 },
        { action = "SET_CUSTOM_DATA", key = "split_25", value = 0 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_ARCANE_EXPLOSION, duration = math.random(6000, 8000) },
        { action = "SET_TIMER", timer_id = TIMER_EARTH_SHOCK,      duration = 2000 },
        { action = "SET_TIMER", timer_id = TIMER_FULFILLMENT,      duration = math.random(10000, 15000) },
        { action = "SET_TIMER", timer_id = TIMER_BLINK,            duration = math.random(15000, 20000) },
        { action = "SET_CUSTOM_DATA", key = "split_75", value = 0 },
        { action = "SET_CUSTOM_DATA", key = "split_50", value = 0 },
        { action = "SET_CUSTOM_DATA", key = "split_25", value = 0 },
    }
end

function boss:OnKilledUnit(input)
    return { { action = "SCRIPT_TEXT", text_id = 11446 } }  -- SAY_SLAY
end

function boss:OnDeath(input)
    return { { action = "SCRIPT_TEXT", text_id = 11447 } }  -- SAY_DEATH
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Arcane Explosion (AoE near melee); repeat urand(6,14)s
    if input:IsTimerReady(TIMER_ARCANE_EXPLOSION) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ARCANE_EXPLOSION, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ARCANE_EXPLOSION, duration = math.random(6000, 14000) })
    end

    -- Earth Shock (spam at range); every 2s
    if input:IsTimerReady(TIMER_EARTH_SHOCK) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_EARTH_SHOCK, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EARTH_SHOCK, duration = 2000 })
    end

    -- True Fulfillment (mind control nearest); urand(20.5,25)s repeat
    if input:IsTimerReady(TIMER_FULFILLMENT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TRUE_FULFILLMENT, target = "nearest_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FULFILLMENT, duration = math.random(20500, 25000) })
    end

    -- Blink to random platform; urand(10,18)s repeat; reset threat
    if input:IsTimerReady(TIMER_BLINK) then
        local blink_spell = ({ SPELL_BLINK_0, SPELL_BLINK_1, SPELL_BLINK_2 })[math.random(1, 3)]
        table.insert(actions, { action = "CAST_SPELL", spell_id = blink_spell, target = "self" })
        table.insert(actions, { action = "RESET_THREAT" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BLINK, duration = math.random(10000, 18000) })
    end

    -- Split at 75%
    local split_75 = input:GetCustomData("split_75") or 0
    if split_75 == 0 and input.health_pct and input.health_pct < 0.75 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUMMON_IMAGES, target = "self" })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "split_75", value = 1 })
    end

    -- Split at 50%
    local split_50 = input:GetCustomData("split_50") or 0
    if split_50 == 0 and input.health_pct and input.health_pct < 0.50 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUMMON_IMAGES, target = "self" })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "split_50", value = 1 })
    end

    -- Split at 25%
    local split_25 = input:GetCustomData("split_25") or 0
    if split_25 == 0 and input.health_pct and input.health_pct < 0.25 then
        table.insert(actions, { action = "SCRIPT_TEXT", text_id = -1531006 })  -- SAY_SPLIT (negative = not DB text)
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUMMON_IMAGES, target = "self" })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "split_25", value = 1 })
    end

    return actions
end

RegisterCreatureAI(15263, boss)
