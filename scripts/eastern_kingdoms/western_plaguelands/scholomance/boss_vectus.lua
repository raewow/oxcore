--[[
    @script_type: creature_ai
    @entry: 10432
    @name: boss_vectus

    Vectus - Scholomance (Viewing Room)
    Ported from vmangos boss_vectus.cpp

    Uses Flamestrike (self-cast AoE) and Blast Wave on current target.
    Linked with Marduk Blackpool (10433) and Scholomance Students (10475) as
    part of the Dawn's Gambit event - when the game object (177304) is destroyed,
    Vectus and Marduk turn hostile. Full event logic requires Phase D (zone scripts).
]]

-- Spells
local SPELL_FLAMESTRIKE = 18399  -- AoE fire damage, self-cast
local SPELL_BLAST_WAVE  = 16046  -- AoE knockback + fire damage on target

-- Timer IDs
local TIMER_FLAMESTRIKE = 1
local TIMER_BLAST_WAVE  = 2

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_FLAMESTRIKE, duration = 2000 },
        { action = "SET_TIMER", timer_id = TIMER_BLAST_WAVE,  duration = 14000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Flamestrike (self-cast AoE, repeats every 30s)
    if input:IsTimerReady(TIMER_FLAMESTRIKE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FLAMESTRIKE, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FLAMESTRIKE, duration = 30000 })
    end

    -- Blast Wave (current target, repeats every 12s)
    if input:IsTimerReady(TIMER_BLAST_WAVE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BLAST_WAVE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BLAST_WAVE, duration = 12000 })
    end

    return actions
end

RegisterCreatureAI(10432, boss)
