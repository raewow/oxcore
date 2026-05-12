--[[
    @script_type: creature_ai
    @entry: 15370
    @name: boss_buru

    Buru the Gorger - Ruins of Ahn'Qiraj
    Verified against vmangos boss_buru.cpp

    Abilities:
    - Dismember: 1s init, 6s repeat (stacking bleed; phase 1 only)
    - Gain Speed (1834): 30s init, 30s repeat (stacking speed; phase 1 only; resets on target change)
    - Full Speed (1557): Cast as part of transform sequence, phase 2 only
    - Thorns (25640): Cast on aggro; removed on transform
    - Creeping Plague: 6s repeat (phase 2 only)
    - Buru Transform (24721): Visual transform at 20% HP

    Phases:
    - Phase 1 (EGG): Fixates target, uses Dismember + Gain Speed
    - Phase 2 (TRANSFORM): At 20% HP; spawns 3 hatchlings, uses Creeping Plague
]]

local SPELL_CREEPING_PLAGUE     = 20512
local SPELL_DISMEMBER           = 96
local SPELL_GAIN_SPEED          = 1834   -- was SPELL_GATHERING_SPEED; this is the correct stacking speed buff
local SPELL_FULL_SPEED          = 1557
local SPELL_THORNS              = 25640
local SPELL_BURU_TRANSFORM      = 24721

local NPC_BURU_EGG              = 15514
local NPC_HATCHLING             = 15521

-- Hatchling spawn locations on transform (from vmangos AddPop array)
local HATCH_LOCS = {
    { x = -9312.0, y = 1281.0, z = -62.0 },
    { x = -9268.0, y = 1249.0, z = -62.0 },
    { x = -9263.0, y = 1292.0, z = -63.0 },
}

local PHASE_EGG                 = 1
local PHASE_TRANSFORM           = 2

local TIMER_DISMEMBER           = 1
local TIMER_GAIN_SPEED          = 2
local TIMER_CREEPING_PLAGUE     = 3

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_PHASE", phase = PHASE_EGG },
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "CAST_SPELL", spell_id = SPELL_THORNS, target = "self" },
        { action = "SET_TIMER", timer_id = TIMER_DISMEMBER,       duration = 1000 },
        { action = "SET_TIMER", timer_id = TIMER_GAIN_SPEED,      duration = 30000 },
        { action = "SET_TIMER", timer_id = TIMER_CREEPING_PLAGUE, duration = 6000 },
        { action = "SET_CUSTOM_DATA", key = "hatched", value = 0 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_PHASE", phase = PHASE_EGG },
        { action = "SET_CUSTOM_DATA", key = "hatched", value = 0 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Phase transition at 20% HP
    if input.phase == PHASE_EGG and input.health_pct and input.health_pct < 0.20 then
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_TRANSFORM })
        table.insert(actions, { action = "REMOVE_AURA", spell_id = SPELL_THORNS })
        table.insert(actions, { action = "REMOVE_AURA", spell_id = SPELL_GAIN_SPEED })
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BURU_TRANSFORM, target = "self" })
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FULL_SPEED, target = "self" })
        table.insert(actions, { action = "RESET_THREAT" })
        return actions
    end

    if input.phase == PHASE_EGG then
        -- Dismember (6s repeat)
        if input:IsTimerReady(TIMER_DISMEMBER) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DISMEMBER, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_DISMEMBER, duration = 6000 })
        end

        -- Gain Speed (stacking; 30s repeat; speed reset when target changes)
        if input:IsTimerReady(TIMER_GAIN_SPEED) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_GAIN_SPEED, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_GAIN_SPEED, duration = 30000 })
        end

    elseif input.phase == PHASE_TRANSFORM then
        -- Spawn 3 hatchlings once on first update in transform phase
        local hatched = input:GetCustomData("hatched") or 0
        if hatched == 0 then
            for _, loc in ipairs(HATCH_LOCS) do
                table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_HATCHLING,
                    x = loc.x, y = loc.y, z = loc.z, o = 0, summon_type = 1, duration = 30000 })
            end
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = "hatched", value = 1 })
        end

        -- Creeping Plague (6s repeat)
        if input:IsTimerReady(TIMER_CREEPING_PLAGUE) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CREEPING_PLAGUE, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CREEPING_PLAGUE, duration = 6000 })
        end
    end

    return actions
end

RegisterCreatureAI(15370, boss)
