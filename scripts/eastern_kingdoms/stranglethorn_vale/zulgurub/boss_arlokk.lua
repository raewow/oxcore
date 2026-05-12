--[[
    @script_type: creature_ai
    @entry: 14515
    @name: boss_arlokk

    High Priestess Arlokk - Zul'Gurub
    Verified against vmangos boss_arlokk.cpp

    Phase 1 (Troll): SWP, Backstab, Gouge, Mark, Vanish/reappear cycle
    Phase 2 (Panther): Rosser(Thrash), Ravage, Gouge, Tourbillon(Whirlwind)
    Vanishes every 35s; reappears in panther form for 45s, then switches back.
]]

local SPELL_SHADOWWORDPAIN    = 24212  -- was 23952 (wrong)
local SPELL_GOUGE             = 12540  -- was 24698 (wrong); reduces threat 80%
local SPELL_MARK              = 24210  -- Arlokk's Mark: summons panthers near marked player
local SPELL_BACKSTAB          = 15582  -- phase 1 melee proc
local SPELL_ROSSER            = 3391   -- Thrash (phase 2 panther)
local SPELL_TOURBILLON        = 15589  -- Whirlwind (phase 2 panther)
local SPELL_RAVAGE            = 24213  -- phase 2 panther
local SPELL_PANTHER_TRANSFORM = 24190

local TIMER_SHADOWWORDPAIN = 1
local TIMER_BACKSTAB       = 2
local TIMER_GOUGE          = 3
local TIMER_MARK           = 4
local TIMER_VANISH         = 5
local TIMER_VISIBLE        = 6  -- how long invisible before reappearing
local TIMER_SWITCH_TROLL   = 7  -- how long in panther form before switching back
local TIMER_ROSSER         = 8
local TIMER_RAVAGE         = 9
local TIMER_TOURBILLON     = 10

local PHASE_TROLL   = 1
local PHASE_PANTHER = 2
local PHASE_VANISHED = 3

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SCRIPT_TEXT", text_id = 10461 },  -- SAY_AGGRO
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_PHASE", phase = PHASE_TROLL },
        { action = "SET_TIMER", timer_id = TIMER_SHADOWWORDPAIN, duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_BACKSTAB,       duration = 6000 },
        { action = "SET_TIMER", timer_id = TIMER_GOUGE,          duration = 14000 },
        { action = "SET_TIMER", timer_id = TIMER_MARK,           duration = 35000 },
        { action = "SET_TIMER", timer_id = TIMER_VANISH,         duration = 35000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_PHASE", phase = PHASE_TROLL },
        { action = "SET_TIMER", timer_id = TIMER_SHADOWWORDPAIN, duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_BACKSTAB,       duration = 6000 },
        { action = "SET_TIMER", timer_id = TIMER_GOUGE,          duration = 14000 },
        { action = "SET_TIMER", timer_id = TIMER_MARK,           duration = 35000 },
        { action = "SET_TIMER", timer_id = TIMER_VANISH,         duration = 35000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- PHASE 1: Troll Form
    if input.phase == PHASE_TROLL then
        if input:IsTimerReady(TIMER_SHADOWWORDPAIN) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOWWORDPAIN, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOWWORDPAIN, duration = 15000 })
        end

        if input:IsTimerReady(TIMER_BACKSTAB) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BACKSTAB, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BACKSTAB, duration = math.random(6000, 12000) })
        end

        if input:IsTimerReady(TIMER_GOUGE) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_GOUGE, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_GOUGE, duration = math.random(17000, 27000) })
        end

        if input:IsTimerReady(TIMER_MARK) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MARK, target = "random_hostile" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MARK, duration = 15000 })
        end

        -- Vanish to panther form
        if input:IsTimerReady(TIMER_VANISH) then
            table.insert(actions, { action = "SET_VISIBILITY", visible = false })
            table.insert(actions, { action = "SET_PHASE", phase = PHASE_VANISHED })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_VISIBLE, duration = math.random(35000, 50000) })
        end
    end

    -- PHASE 3: Vanished -> reappear as panther
    if input.phase == PHASE_VANISHED then
        if input:IsTimerReady(TIMER_VISIBLE) then
            table.insert(actions, { action = "SET_VISIBILITY", visible = true })
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_PANTHER_TRANSFORM, target = "self" })
            table.insert(actions, { action = "SET_PHASE", phase = PHASE_PANTHER })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ROSSER,       duration = math.random(5000, 9000) })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_RAVAGE,       duration = 15000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_GOUGE,        duration = 14000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TOURBILLON,   duration = 4000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SWITCH_TROLL, duration = 45000 })
        end
    end

    -- PHASE 2: Panther Form
    if input.phase == PHASE_PANTHER then
        if input:IsTimerReady(TIMER_ROSSER) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ROSSER, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ROSSER, duration = math.random(5000, 9000) })
        end

        if input:IsTimerReady(TIMER_RAVAGE) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_RAVAGE, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_RAVAGE, duration = 16000 })
        end

        if input:IsTimerReady(TIMER_GOUGE) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_GOUGE, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_GOUGE, duration = math.random(17000, 27000) })
        end

        if input:IsTimerReady(TIMER_TOURBILLON) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TOURBILLON, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TOURBILLON, duration = 16000 })
        end

        -- Switch back to troll form after 45s in panther; vanish again after 30s
        if input:IsTimerReady(TIMER_SWITCH_TROLL) then
            table.insert(actions, { action = "SET_PHASE", phase = PHASE_TROLL })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOWWORDPAIN, duration = 8000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BACKSTAB,       duration = 6000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_GOUGE,          duration = 14000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MARK,           duration = 35000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_VANISH,         duration = 30000 })
        end
    end

    return actions
end

function boss:OnDeath(input)
    return { { action = "SCRIPT_TEXT", text_id = 10450 } }  -- SAY_DEATH
end

RegisterCreatureAI(14515, boss)
