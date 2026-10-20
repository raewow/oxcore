--[[
    Azshara zone scripts
    Reference: kalimdor/azshara/azshara.cpp

    mob_maws (entry 15341)
        Out-of-combat: patrols a looping path around the Bay of Storms via
        MovementInform (MovePoint chain). Handled by DB waypoints.
        In-combat: casts Rampage (25744), Frenzy (19812), and at <20% HP also
        Dark Water (25743). On death broadcasts a server-wide emote.

    go_bay_of_storms (entry 180670)
        GameObjectAI that plays a cycling custom anim every 3-8s.
        GO AI is not yet supported in Lua; deferred.
]]

local MOB_MAWS                  = 15341

local SPELL_DARK_WATER          = 25743
local SPELL_FRENZY              = 19812
local SPELL_RAMPAGE             = 25744

local RAMPAGE_TIMER_INIT_MIN    = 20000
local RAMPAGE_TIMER_INIT_MAX    = 120000
local RAMPAGE_TIMER_MAX_P2      = 12000
local FRENZY_TIMER              = 25000
local FRENZY_TIMER_P2           = 15000
local DARK_WATER_TIMER          = 15000
local EMOTE_THE_BEAST_RETURNS   = 11160  -- broadcast text

local TIMER_RAMPAGE             = 1
local TIMER_FRENZY              = 2
local TIMER_DARK_WATER          = 3
local DATA_PHASE_TWO            = "phase_two"

local mob_maws = {}

local function rand_between(lo, hi)
    return lo + math.floor(math.random() * (hi - lo + 1))
end

local function maws_start()
    return {
        { action = "SET_TIMER", timer_id = TIMER_RAMPAGE,
          duration = rand_between(RAMPAGE_TIMER_INIT_MIN, RAMPAGE_TIMER_INIT_MAX) },
        { action = "SET_TIMER", timer_id = TIMER_FRENZY,     duration = FRENZY_TIMER },
        { action = "SET_TIMER", timer_id = TIMER_DARK_WATER, duration = DARK_WATER_TIMER },
        { action = "SET_CUSTOM_DATA", key = DATA_PHASE_TWO, value = 0 },
    }
end

function mob_maws:OnSpawn(input)
    return maws_start()
end

function mob_maws:OnReset(input)
    return maws_start()
end

function mob_maws:OnUpdate(input)
    if not input.is_in_combat or not input.current_target then
        return {}
    end

    local actions = {}
    local phase_two = input:GetCustomData(DATA_PHASE_TWO) == 1

    -- Phase 2 transition at <20% HP: the timers shorten, and running ones are cut down to match
    if not phase_two and input.health_pct < 20.0 then
        phase_two = true
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE_TWO, value = 1 })
        if input:GetTimer(TIMER_RAMPAGE) > RAMPAGE_TIMER_MAX_P2 then
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_RAMPAGE, duration = RAMPAGE_TIMER_MAX_P2 })
        end
        if input:GetTimer(TIMER_FRENZY) > FRENZY_TIMER_P2 then
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FRENZY, duration = FRENZY_TIMER_P2 })
        end
    end

    if input:IsTimerReady(TIMER_RAMPAGE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_RAMPAGE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_RAMPAGE,
            duration = phase_two and RAMPAGE_TIMER_MAX_P2 or rand_between(20000, 120000) })
    end

    if input:IsTimerReady(TIMER_FRENZY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FRENZY, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FRENZY,
            duration = phase_two and FRENZY_TIMER_P2 or FRENZY_TIMER })
    end

    -- Dark Water (phase 2 only, self-cast)
    if phase_two and input:IsTimerReady(TIMER_DARK_WATER) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DARK_WATER, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_DARK_WATER, duration = DARK_WATER_TIMER })
    end

    return actions
end

function mob_maws:OnDeath(input, killer_guid)
    return { { action = "SCRIPT_TEXT", text_id = EMOTE_THE_BEAST_RETURNS } }
end

RegisterCreatureAI(MOB_MAWS, mob_maws)
