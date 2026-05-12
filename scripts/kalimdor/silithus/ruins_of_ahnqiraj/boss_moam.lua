--[[
    @script_type: creature_ai
    @entry: 15340
    @name: boss_moam

    Moam - Ruins of Ahn'Qiraj
    Verified against vmangos boss_moam.cpp

    Abilities:
    - Trample: 6s init, 15s repeat
    - Drain Mana (25676): 5s init, 7s repeat (casts on self, drains nearby players)
    - Energize (stone form): Every 90s, gains SPELL_ENERGIZE aura + summons 3 mana fiends
      Stone form lasts until mana is full (90s fallback), then casts Arcane Eruption
    - Arcane Eruption: On stone form end, AoE blast
    - Summon Mana Fiend: 3 fiends on each energize transition (every 90s)

    Phases:
    - PHASE_ATTACKING (0): Normal combat
    - PHASE_ENERGIZING (1): Stone form, channels mana; exits when mana full or 90s timeout
]]

local SPELL_TRAMPLE             = 15550
local SPELL_DRAIN_MANA          = 25676   -- was 25671 (wrong)
local SPELL_ARCANE_ERUPTION     = 25672
local SPELL_SUMMON_MANA_FIEND   = 25681   -- x3 summons; was using 3 different IDs (wrong)
local SPELL_ENERGIZE            = 25685

local PHASE_ATTACKING           = 0
local PHASE_ENERGIZING          = 1

local TIMER_TRAMPLE             = 1
local TIMER_DRAIN_MANA          = 2
local TIMER_SUMMON_MANAFIENDS   = 3
local TIMER_STONE_TIMEOUT       = 4

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_PHASE", phase = PHASE_ATTACKING },
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_TRAMPLE,           duration = 6000 },
        { action = "SET_TIMER", timer_id = TIMER_DRAIN_MANA,        duration = 5000 },
        { action = "SET_TIMER", timer_id = TIMER_SUMMON_MANAFIENDS, duration = 90000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    if input.phase == PHASE_ATTACKING then
        -- Drain Mana (repeat 7s)
        if input:IsTimerReady(TIMER_DRAIN_MANA) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DRAIN_MANA, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_DRAIN_MANA, duration = 7000 })
        end

        -- Trample (repeat 15s)
        if input:IsTimerReady(TIMER_TRAMPLE) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TRAMPLE, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TRAMPLE, duration = 15000 })
        end

        -- Energize (stone form) + summon 3 mana fiends every 90s
        if input:IsTimerReady(TIMER_SUMMON_MANAFIENDS) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ENERGIZE, target = "self" })
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUMMON_MANA_FIEND, target = "self" })
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUMMON_MANA_FIEND, target = "self" })
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUMMON_MANA_FIEND, target = "self" })
            table.insert(actions, { action = "SET_PHASE", phase = PHASE_ENERGIZING })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_STONE_TIMEOUT, duration = 90000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SUMMON_MANAFIENDS, duration = 90000 })
        end

    elseif input.phase == PHASE_ENERGIZING then
        -- 90s timeout fallback for stone form
        if input:IsTimerReady(TIMER_STONE_TIMEOUT) then
            table.insert(actions, { action = "REMOVE_AURA", spell_id = SPELL_ENERGIZE })
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ARCANE_ERUPTION, target = "current_target" })
            table.insert(actions, { action = "SET_PHASE", phase = PHASE_ATTACKING })
        end
    end

    return actions
end

RegisterCreatureAI(15340, boss)
