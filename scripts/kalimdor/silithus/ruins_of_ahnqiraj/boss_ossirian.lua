--[[
    @script_type: creature_ai
    @entry: 15339
    @name: boss_ossirian

    Ossirian the Unscarred - Ruins of Ahn'Qiraj
    Verified against vmangos boss_ossirian.cpp

    Abilities:
    - Supreme (25176): Cast on aggro + every 45s (or 25s initial; removed when crystal weakness applied)
    - War Stomp (25188): 25s init, urand(25,35)s repeat
    - Enveloping Winds/Cyclone (25189): 20s init, 15s repeat
    - Curse of Tongues/Silence (25195): 30s init, urand(10,20)s repeat

    Crystal interaction (weakness spells 25177-25183) is handled server-side via
    instance script (ossirian_crystalAI) — not modeled here.
]]

local SPELL_SILENCE     = 25195   -- SPELL_CURSE_OF_TONGUES in C++
local SPELL_CYCLONE     = 25189   -- SPELL_ENVELOPING_WINDS in C++
local SPELL_STOMP       = 25188   -- SPELL_WAR_STOMP in C++
local SPELL_SUPREME     = 25176   -- SPELL_STRENGTH_OF_OSSIRIAN in C++

local TIMER_SUPREME     = 1
local TIMER_STOMP       = 2
local TIMER_CYCLONE     = 3
local TIMER_SILENCE     = 4

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SCRIPT_TEXT", text_id = 11449 },  -- SAY_AGGRO
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "CAST_SPELL", spell_id = SPELL_SUPREME, target = "self" },
        { action = "SET_TIMER", timer_id = TIMER_SUPREME,  duration = 25000 },
        { action = "SET_TIMER", timer_id = TIMER_STOMP,    duration = 25000 },
        { action = "SET_TIMER", timer_id = TIMER_CYCLONE,  duration = 20000 },
        { action = "SET_TIMER", timer_id = TIMER_SILENCE,  duration = 30000 },
    }
end

function boss:OnKill(input)
    return {
        { action = "SCRIPT_TEXT", text_id = 11450 },  -- SAY_SLAY
    }
end

function boss:OnDeath(input)
    return {
        { action = "SCRIPT_TEXT", text_id = 11451 },  -- SAY_DEATH
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Reapply Supreme when weakness expires (45s after last weakness; or timer fires)
    if input:IsTimerReady(TIMER_SUPREME) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUPREME, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SUPREME, duration = 45000 })
    end

    -- War Stomp (urand(25,35)s repeat)
    if input:IsTimerReady(TIMER_STOMP) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_STOMP, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_STOMP, duration = math.random(25000, 35000) })
    end

    -- Enveloping Winds on current target (15s repeat); C++ resets threat on hit
    if input:IsTimerReady(TIMER_CYCLONE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CYCLONE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CYCLONE, duration = 15000 })
    end

    -- Curse of Tongues (urand(10,20)s repeat)
    if input:IsTimerReady(TIMER_SILENCE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SILENCE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SILENCE, duration = math.random(10000, 20000) })
    end

    return actions
end

RegisterCreatureAI(15339, boss)
