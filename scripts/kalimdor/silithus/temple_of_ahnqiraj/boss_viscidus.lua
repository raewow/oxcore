--[[
    @script_type: creature_ai
    @entry: 15299
    @name: boss_viscidus

    Viscidus - Temple of Ahn'Qiraj
    Verified against vmangos boss_viscidus.cpp

    Note: The original SD2 script was a placeholder (0% complete).
    This implements the basic spell rotation from the defined spells.

    Mechanics:
    - Poison Shock: AoE nature damage
    - Poison Bolt Volley: Ranged poison volley
    - Toxin: Spawns toxin clouds
    - Frost damage phases: Slowed -> Frozen -> Shatter (requires melee hits while frozen)
    - Rejoin: Globs reform into Viscidus if not killed fast enough
]]

local SPELL_POISON_SHOCK        = 25993
local SPELL_POISONBOLT_VOLLEY   = 25991
local SPELL_TOXIN               = 26575

-- Freeze debuffs (applied by frost damage from players, tracked by the boss)
local SPELL_VISCIDUS_SLOWED     = 26034
local SPELL_VISCIDUS_SLOWED_MORE = 26036
local SPELL_VISCIDUS_FREEZE     = 25937

local SPELL_VISCIDUS_EXPLODE    = 25938
local SPELL_REJOIN_VISCIDUS     = 25896

local TIMER_POISON_SHOCK        = 1
local TIMER_POISONBOLT          = 2
local TIMER_TOXIN               = 3

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_POISON_SHOCK, duration = math.random(7000, 12000) },
        { action = "SET_TIMER", timer_id = TIMER_POISONBOLT, duration = math.random(10000, 15000) },
        { action = "SET_TIMER", timer_id = TIMER_TOXIN, duration = math.random(30000, 40000) },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Poison Shock
    if input:IsTimerReady(TIMER_POISON_SHOCK) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_POISON_SHOCK, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_POISON_SHOCK, duration = math.random(7000, 12000) })
    end

    -- Poison Bolt Volley
    if input:IsTimerReady(TIMER_POISONBOLT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_POISONBOLT_VOLLEY, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_POISONBOLT, duration = math.random(10000, 15000) })
    end

    -- Toxin Cloud
    if input:IsTimerReady(TIMER_TOXIN) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TOXIN, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TOXIN, duration = math.random(30000, 40000) })
    end

    return actions
end

RegisterCreatureAI(15299, boss)
