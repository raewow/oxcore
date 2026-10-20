--[[
    Undercity zone scripts
    Reference: eastern_kingdoms/undercity/undercity.cpp

    npc_lady_sylvanas_windrunner (entry 10181)
        Combat AI: Summon Skeletons (20464) every 25s, Fade (20672) every 50s
        (then kite at 30yd for 5s), Black Arrow (20733) random target every 15s,
        Multi Shot (20735) random target every 10s, Shoot (20463) main target
        every 10s. On enter combat: plays sound 5886.
]]

local NPC_SYLVANAS      = 10181

local SPELL_SUMMON_SKEL = 20464
local SPELL_FADE        = 20672
local SPELL_BLACK_ARROW = 20733
local SPELL_MULTI_SHOT  = 20735
local SPELL_SHOOT       = 20463

local TIMER_SUMMON_SKEL = 1
local TIMER_FADE        = 2
local TIMER_FADED       = 3   -- kiting at range after Fade
local TIMER_BLACK_ARROW = 4
local TIMER_MULTI_SHOT  = 5
local TIMER_SHOOT       = 6

local npc_sylvanas = {}

local function initial_timers()
    return {
        { action = "SET_TIMER", timer_id = TIMER_SUMMON_SKEL, duration = 25000 },
        { action = "SET_TIMER", timer_id = TIMER_FADE,        duration = 50000 },
        { action = "SET_TIMER", timer_id = TIMER_FADED,       duration = 0 },
        { action = "SET_TIMER", timer_id = TIMER_BLACK_ARROW, duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_MULTI_SHOT,  duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_SHOOT,       duration = 10000 },
    }
end

function npc_sylvanas:OnSpawn(input)
    return initial_timers()
end

function npc_sylvanas:OnReset(input)
    return initial_timers()
end

function npc_sylvanas:OnEnterCombat(input)
    return { { action = "PLAY_SOUND", sound_id = 5886, zone_wide = false } }
end

function npc_sylvanas:OnUpdate(input)
    -- Faded kite phase (stay at range 30yd for 5s after Fade)
    if not input:IsTimerReady(TIMER_FADED) then
        return {}
    end

    if not input.is_in_combat or not input.current_target then
        return {}
    end

    local actions = {}

    if input:IsTimerReady(TIMER_SUMMON_SKEL) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUMMON_SKEL, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SUMMON_SKEL, duration = 25000 })
    end

    if input:IsTimerReady(TIMER_FADE) then
        -- After the kite every ranged attack fires at once.
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FADE, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FADE,        duration = 50000 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FADED,       duration = 5000 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BLACK_ARROW, duration = 0 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MULTI_SHOT,  duration = 0 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHOOT,       duration = 0 })
        return actions
    end

    if input:IsTimerReady(TIMER_BLACK_ARROW) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BLACK_ARROW, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BLACK_ARROW, duration = 15000 })
    end

    if input:IsTimerReady(TIMER_MULTI_SHOT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MULTI_SHOT, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MULTI_SHOT, duration = 10000 })
    end

    if input:IsTimerReady(TIMER_SHOOT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHOOT, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHOOT, duration = 10000 })
    end

    return actions
end

RegisterCreatureAI(NPC_SYLVANAS, npc_sylvanas)
