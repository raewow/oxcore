--[[
    @script_type: creature_ai
    @entry: 11982
    @name: boss_magmadar

    Magmadar - Molten Core
    Ported from vmangos boss_magmadar.cpp
]]

-- Spells (exact from vmangos)
local SPELL_FRENZY = 19451           -- Triggers SPELL_LAVA_BREATH (19272)
local SPELL_MAGMASPIT = 19449        -- Self-buff
local SPELL_PANIC = 19408            -- AoE fear
local SPELL_LAVABOMB = 19411         -- On non-mana targets
local SPELL_LAVABOMB_MANA = 20474    -- On mana targets

-- Timer IDs
local TIMER_FRENZY = 1
local TIMER_PANIC = 2
local TIMER_LAVABOMB = 3
local TIMER_LAVABOMB_MANA = 4

local boss = {}

function boss:OnEnterCombat(input)
    -- Apply Magma Spit self-buff if not present
    local actions = {}
    if not input:HasAura(SPELL_MAGMASPIT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MAGMASPIT, target = "self" })
    end
    table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FRENZY, duration = 15000 })
    table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PANIC, duration = 10000 })
    table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_LAVABOMB, duration = 12000 })
    table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_LAVABOMB_MANA, duration = 18000 })
    return actions
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Frenzy (self-buff, repeats urand(15000,20000) -> 17500)
    if input:IsTimerReady(TIMER_FRENZY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FRENZY, target = "self" })
        table.insert(actions, { action = "PLAY_EMOTE", emote_id = 7797 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FRENZY, duration = 17500 })
    end

    -- Panic (AoE fear on victim, repeats urand(30000,35000) -> 32500)
    if input:IsTimerReady(TIMER_PANIC) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_PANIC, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PANIC, duration = 32500 })
    end

    -- Lava Bomb (random, repeats urand(12000,15000) -> 13500)
    if input:IsTimerReady(TIMER_LAVABOMB) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_LAVABOMB, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_LAVABOMB, duration = 13500 })
    end

    -- Lava Bomb Mana (random, repeats urand(12000,15000) -> 13500)
    if input:IsTimerReady(TIMER_LAVABOMB_MANA) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_LAVABOMB_MANA, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_LAVABOMB_MANA, duration = 13500 })
    end

    return actions
end

RegisterCreatureAI(11982, boss)
