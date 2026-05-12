--[[
    @script_type: creature_ai
    @entry: 12264
    @name: boss_shazzrah

    Shazzrah - Molten Core
    Ported from vmangos boss_shazzrah.cpp
]]

-- Spells (exact from vmangos)
local SPELL_ARCANEEXPLOSION = 19712
local SPELL_SHAZZRAHCURSE = 19713
local SPELL_DEADENMAGIC = 19714      -- Self-buff, reduces magic damage taken
local SPELL_COUNTERSPELL = 19715
local SPELL_GATE_DUMMY = 23138       -- Gate of Shazzrah (blink + threat reset)

-- Timer IDs
local TIMER_ARCANE_EXPLOSION = 1
local TIMER_CURSE = 2
local TIMER_DEADEN_MAGIC = 3
local TIMER_COUNTERSPELL = 4
local TIMER_BLINK = 5

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_ARCANE_EXPLOSION, duration = 2000 },
        { action = "SET_TIMER", timer_id = TIMER_CURSE, duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_DEADEN_MAGIC, duration = 5000 },
        { action = "SET_TIMER", timer_id = TIMER_COUNTERSPELL, duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_BLINK, duration = 27500 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Arcane Explosion (repeats urand(3000,5000))
    if input:IsTimerReady(TIMER_ARCANE_EXPLOSION) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ARCANEEXPLOSION, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ARCANE_EXPLOSION, duration = 4000 })
    end

    -- Shazzrah's Curse (repeats every 20s)
    if input:IsTimerReady(TIMER_CURSE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHAZZRAHCURSE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CURSE, duration = 20000 })
    end

    -- Deaden Magic self-buff (repeats urand(7000,14000))
    if input:IsTimerReady(TIMER_DEADEN_MAGIC) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DEADENMAGIC, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_DEADEN_MAGIC, duration = 10500 })
    end

    -- Counterspell (repeats urand(16000,18000))
    if input:IsTimerReady(TIMER_COUNTERSPELL) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_COUNTERSPELL, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_COUNTERSPELL, duration = 17000 })
    end

    -- Gate of Shazzrah (blink + threat reset, repeats urand(25000,35000))
    if input:IsTimerReady(TIMER_BLINK) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_GATE_DUMMY, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BLINK, duration = 30000 })
    end

    return actions
end

RegisterCreatureAI(12264, boss)
