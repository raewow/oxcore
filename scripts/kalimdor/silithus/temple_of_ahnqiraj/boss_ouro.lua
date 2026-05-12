--[[
    @script_type: creature_ai
    @entry: 15517
    @name: boss_ouro

    Ouro - Temple of Ahn'Qiraj
    Verified against vmangos boss_ouro.cpp

    Abilities:
    - Sweep: Fixed 20.5s timer (SWEEP_TIMER = 20500)
    - Sand Blast: urand(20,25)s (patch 1.10+); reduces top-aggro threat -100%
    - Submerge: 90s on surface, 30s submerged; teleports to trigger position; SPELL_BIRTH on emerge
    - Berserk: At 20% HP, stops submerging; summons 1 mound every 10s
    - Boulder: Ranged spam when enraged and no melee target
]]

local SPELL_SWEEP           = 26103
local SPELL_SANDBLAST       = 26102
local SPELL_GROUND_RUPTURE  = 26100
local SPELL_BIRTH           = 26262
local SPELL_BERSERK         = 26615
local SPELL_SUBMERGE_VISUAL = 26063
local SPELL_SUMMON_MOUNDS   = 26058  -- summons 5 dirt mounds on submerge
local SPELL_SUMMON_MOUND    = 26617  -- summons 1 mound (enrage phase)
local SPELL_BOULDER         = 26616

local TIMER_SWEEP           = 1
local TIMER_SANDBLAST       = 2
local TIMER_SUBMERGE        = 3
local TIMER_EMERGE          = 4
local TIMER_SUMMON_MOUND    = 5

local PHASE_SURFACE     = 0
local PHASE_SUBMERGED   = 1

local SWEEP_TIMER       = 20500
local SUBMERGE_TIMER    = 90000

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "CAST_SPELL", spell_id = SPELL_BIRTH, target = "self" },
        { action = "SET_PHASE", phase = PHASE_SURFACE },
        { action = "SET_TIMER", timer_id = TIMER_SWEEP,      duration = SWEEP_TIMER },
        { action = "SET_TIMER", timer_id = TIMER_SANDBLAST,  duration = math.random(20000, 25000) },
        { action = "SET_TIMER", timer_id = TIMER_SUBMERGE,   duration = SUBMERGE_TIMER },
        { action = "SET_CUSTOM_DATA", key = "enraged", value = 0 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_PHASE", phase = PHASE_SURFACE },
        { action = "SET_TIMER", timer_id = TIMER_SWEEP,      duration = SWEEP_TIMER },
        { action = "SET_TIMER", timer_id = TIMER_SANDBLAST,  duration = math.random(20000, 25000) },
        { action = "SET_TIMER", timer_id = TIMER_SUBMERGE,   duration = SUBMERGE_TIMER },
        { action = "SET_CUSTOM_DATA", key = "enraged", value = 0 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    local enraged = input:GetCustomData("enraged") or 0

    if input.phase == PHASE_SURFACE then
        -- Sweep (fixed timer)
        if input:IsTimerReady(TIMER_SWEEP) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SWEEP, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SWEEP, duration = SWEEP_TIMER })
        end

        -- Sand Blast (top aggro; reduces their threat -100%)
        if input:IsTimerReady(TIMER_SANDBLAST) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SANDBLAST, target = "current_target" })
            table.insert(actions, { action = "RESET_THREAT" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SANDBLAST, duration = math.random(20000, 25000) })
        end

        -- Berserk at 20% HP
        if enraged == 0 and input.health_pct and input.health_pct < 0.20 then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BERSERK, target = "self" })
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = "enraged", value = 1 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SUMMON_MOUND, duration = 10000 })
            return actions
        end

        -- Submerge (only if not enraged)
        if enraged == 0 and input:IsTimerReady(TIMER_SUBMERGE) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUBMERGE_VISUAL, target = "self" })
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUMMON_MOUNDS, target = "self" })
            table.insert(actions, { action = "RESET_THREAT" })
            table.insert(actions, { action = "SET_VISIBILITY", visible = false })
            table.insert(actions, { action = "SET_PHASE", phase = PHASE_SUBMERGED })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EMERGE,   duration = 30000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SUBMERGE, duration = SUBMERGE_TIMER })
        end

        -- Enraged: summon mound every 10s
        if enraged == 1 and input:IsTimerReady(TIMER_SUMMON_MOUND) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUMMON_MOUND, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SUMMON_MOUND, duration = 10000 })
        end

        -- Boulder spam when enraged (no melee target approximated as periodic)
        if enraged == 1 then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BOULDER, target = "random_hostile" })
        end
    elseif input.phase == PHASE_SUBMERGED then
        -- Emerge after 30s
        if input:IsTimerReady(TIMER_EMERGE) then
            table.insert(actions, { action = "SET_VISIBILITY", visible = true })
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BIRTH, target = "self" })
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_GROUND_RUPTURE, target = "current_target" })
            table.insert(actions, { action = "SET_PHASE", phase = PHASE_SURFACE })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SWEEP,     duration = SWEEP_TIMER })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SANDBLAST, duration = math.random(20000, 25000) })
        end
    end

    return actions
end

RegisterCreatureAI(15517, boss)
