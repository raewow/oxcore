--[[
    @script_type: creature_ai
    @entry: 11502
    @name: boss_ragnaros

    Ragnaros - Molten Core
    Ported from vmangos boss_ragnaros.cpp

    Note: Ragnaros is a stationary boss (SetCombatMovement false).
    Submerge/emerge cycle with Sons of Flame spawning.
    Magma Blast fires only when no melee targets in range.
    Full Majordomo death event + Ragnaros summoning needs instance script.
]]

-- Spells (exact from vmangos)
local SPELL_ELEMENTAL_FIRE_AURA = 20563  -- Aura: triggers on every hit
local SPELL_MELT_WEAPON_AURA = 21387     -- Aura: triggers on melee damage taken
local SPELL_WRATH_OF_RAGNAROS = 20566    -- PBAOE knockback, resets threat
local SPELL_MAGMA_BLAST = 20565          -- Ranged attack (no melee targets)
local SPELL_MIGHT_OF_RAGNAROS = 21154    -- Mana drain / summon flame trigger
local SPELL_SUBMERGE_VISUAL = 20567
local SPELL_SUBMERGE_EFFECT = 21859
local SPELL_EMERGE_VISUAL = 20568

-- NPC entries
local NPC_SON_OF_FLAME = 12143

-- Son of Flame spawn positions (from vmangos PositionOfAdds)
local SON_POSITIONS = {
    { x = 848.740356, y = -816.103455, z = -229.775 },
    { x = 852.560791, y = -849.861511, z = -228.560 },
    { x = 808.710632, y = -852.845764, z = -227.551 },
    { x = 786.597107, y = -821.132874, z = -225.876 },
    { x = 796.219116, y = -800.948059, z = -226.175 },
    { x = 821.602539, y = -782.744109, z = -226.007 },
    { x = 844.924744, y = -769.453735, z = -225.521 },
    { x = 839.823364, y = -810.869385, z = -229.683 },
}

-- Yell text IDs (from vmangos)
local SAY_REINFORCEMENTS1 = 8572
local SAY_REINFORCEMENTS2 = 8573
local SAY_HAND = 9426
local SAY_WRATH = 9427
local SAY_KILL = 7626

-- Timer IDs
local TIMER_WRATH = 1
local TIMER_MIGHT = 2
local TIMER_MAGMA_BLAST = 3
local TIMER_SUBMERGE = 4
local TIMER_EMERGE = 5

-- Phase constants
local PHASE_EMERGED = 0
local PHASE_SUBMERGED = 1

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_PHASE", phase = PHASE_EMERGED },
        { action = "SET_COMBAT_MOVEMENT", enabled = false },
        -- Apply passive auras
        { action = "CAST_SPELL", spell_id = SPELL_ELEMENTAL_FIRE_AURA, target = "self" },
        { action = "CAST_SPELL", spell_id = SPELL_MELT_WEAPON_AURA, target = "self" },
        -- Start timers
        { action = "SET_TIMER", timer_id = TIMER_WRATH, duration = 27500 },      -- urand(25000,30000)
        { action = "SET_TIMER", timer_id = TIMER_MIGHT, duration = 12500 },       -- urand(10000,15000)
        { action = "SET_TIMER", timer_id = TIMER_MAGMA_BLAST, duration = 2000 },
        { action = "SET_TIMER", timer_id = TIMER_SUBMERGE, duration = 180000 },   -- 3 minutes
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Submerged phase: wait for emerge timer
    if input.phase == PHASE_SUBMERGED then
        if input:IsTimerReady(TIMER_EMERGE) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_EMERGE_VISUAL, target = "self" })
            table.insert(actions, { action = "SET_PHASE", phase = PHASE_EMERGED })
            -- Reset emerged-phase timers
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WRATH, duration = 27500 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MIGHT, duration = 12500 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MAGMA_BLAST, duration = 2000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SUBMERGE, duration = 180000 })
        end
        return actions
    end

    -- Wrath of Ragnaros (PBAOE knockback, resets threat, repeats urand(25000,30000))
    if input:IsTimerReady(TIMER_WRATH) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_WRATH_OF_RAGNAROS, target = "self" })
        table.insert(actions, { action = "YELL", text_id = SAY_WRATH })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WRATH, duration = 27500 })
    end

    -- Might of Ragnaros (targets mana users, repeats urand(9000,14000))
    if input:IsTimerReady(TIMER_MIGHT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MIGHT_OF_RAGNAROS, target = "random_hostile" })
        table.insert(actions, { action = "YELL", text_id = SAY_HAND })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MIGHT, duration = 11500 })
    end

    -- Magma Blast (fires when no melee targets in range, repeats 2.5s)
    -- Simplified: fires on timer, in practice should check melee range
    if input:IsTimerReady(TIMER_MAGMA_BLAST) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MAGMA_BLAST, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MAGMA_BLAST, duration = 2500 })
    end

    -- Submerge after 3 minutes
    if input:IsTimerReady(TIMER_SUBMERGE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUBMERGE_VISUAL, target = "self" })
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUBMERGE_EFFECT, target = "self" })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_SUBMERGED })
        table.insert(actions, { action = "YELL", text_id = SAY_REINFORCEMENTS1 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EMERGE, duration = 90000 })  -- 1.5 minutes

        -- Spawn 8 Sons of Flame at fixed positions
        for i = 1, 8 do
            table.insert(actions, {
                action = "SPAWN_CREATURE",
                entry = NPC_SON_OF_FLAME,
                x = SON_POSITIONS[i].x,
                y = SON_POSITIONS[i].y,
                z = SON_POSITIONS[i].z,
                o = 0,
                summon_type = 1,
                duration = 90000,
            })
        end

        return actions
    end

    return actions
end

function boss:OnKill(input)
    return {
        { action = "YELL", text_id = SAY_KILL },
    }
end

RegisterCreatureAI(11502, boss)
