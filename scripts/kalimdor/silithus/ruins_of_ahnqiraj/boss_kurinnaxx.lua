--[[
    @script_type: creature_ai
    @entry: 15348
    @name: boss_kurinnaxx

    Kurinnaxx - Ruins of Ahn'Qiraj
    Verified against vmangos boss_kurinnaxx.cpp

    Abilities:
    - Mortal Wound: 7s init, 9s repeat (on tank)
    - Sand Trap: 7s init, urand(5100,7000)ms repeat (on random player; summons GO_TRAP 180647)
    - Wide Slash: 15s init, urand(10000,20000)ms repeat
    - Trash: 10s init, urand(10000,20000)ms repeat
    - Enrage: At 30% HP (one-time)
]]

local SPELL_MORTAL_WOUND    = 25646
local SPELL_SUMMON_SANDTRAP = 26524   -- was 25648 (wrong); spawns GO trap 180647
local SPELL_WIDE_SLASH      = 25814
local SPELL_TRASH           = 3391
local SPELL_ENRAGE          = 26527

local TIMER_MORTAL_WOUND    = 1
local TIMER_SANDTRAP        = 2
local TIMER_WIDE_SLASH      = 3
local TIMER_TRASH           = 4

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_MORTAL_WOUND, duration = 7000 },
        { action = "SET_TIMER", timer_id = TIMER_SANDTRAP,     duration = 7000 },
        { action = "SET_TIMER", timer_id = TIMER_WIDE_SLASH,   duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_TRASH,        duration = 10000 },
        { action = "SET_CUSTOM_DATA", key = "enraged", value = 0 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_CUSTOM_DATA", key = "enraged", value = 0 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Enrage at 30% HP (one-time)
    local enraged = input:GetCustomData("enraged") or 0
    if enraged == 0 and input.health_pct and input.health_pct <= 0.30 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ENRAGE, target = "self" })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "enraged", value = 1 })
    end

    -- Mortal Wound (9s repeat)
    if input:IsTimerReady(TIMER_MORTAL_WOUND) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MORTAL_WOUND, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MORTAL_WOUND, duration = 9000 })
    end

    -- Sand Trap on random player (urand(5100,7000) repeat)
    if input:IsTimerReady(TIMER_SANDTRAP) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUMMON_SANDTRAP, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SANDTRAP, duration = math.random(5100, 7000) })
    end

    -- Wide Slash (urand(10000,20000) repeat)
    if input:IsTimerReady(TIMER_WIDE_SLASH) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_WIDE_SLASH, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WIDE_SLASH, duration = math.random(10000, 20000) })
    end

    -- Trash (urand(10000,20000) repeat)
    if input:IsTimerReady(TIMER_TRASH) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TRASH, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TRASH, duration = math.random(10000, 20000) })
    end

    return actions
end

RegisterCreatureAI(15348, boss)
