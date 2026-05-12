--[[
    @script_type: creature_ai
    @entry: 2748
    @name: boss_archaedas

    Archaedas - Uldaman
    Ported from MaNGOS boss_archaedas.cpp

    Mechanics:
    - Awakening visual on combat start (phased intro via altar activation)
    - Awaken Earthen Dwarf periodically while HP >= 33% (every 9-12s)
    - Awaken Earthen Guardian once at 66% HP
    - Awaken Vault Warder once at 33% HP
    - On kill: yells
    - On death: instance data set to DONE (door opens)
    - Melee only (no direct damage spells)

    Note: The full altar activation intro sequence requires instance script
    support. This script covers the combat AI portion.
]]

local SPELL_GROUND_TREMOR            = 6524
local SPELL_AWAKEN_EARTHEN_GUARDIAN  = 10252
local SPELL_AWAKEN_VAULT_WARDER      = 10258
local SPELL_AWAKEN_EARTHEN_DWARF     = 10259
local SPELL_ARCHAEDAS_AWAKEN_VISUAL  = 10347

local TIMER_AWAKEN_DWARF = 1

local PHASE_NORMAL          = 0
local PHASE_GUARDIANS_AWOKE = 1
local PHASE_WARDERS_AWOKE   = 2

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "CAST_SPELL", spell_id = SPELL_ARCHAEDAS_AWAKEN_VISUAL, target = "self" },
        { action = "YELL", text = "Who dares awaken Archaedas? Who dares the wrath of the makers!" },
        { action = "REMOVE_UNIT_FLAG", flag = "NOT_SELECTABLE" },
        { action = "SET_TIMER", timer_id = TIMER_AWAKEN_DWARF, duration = 10000 },
        { action = "SET_PHASE", phase = PHASE_NORMAL },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Awaken Earthen Dwarves while HP >= 33%
    if input.health_pct >= 0.33 and input:IsTimerReady(TIMER_AWAKEN_DWARF) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_AWAKEN_EARTHEN_DWARF, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_AWAKEN_DWARF, duration = 10000 })
    end

    -- Awaken Earthen Guardians at 66% HP (once)
    if input.health_pct <= 0.66 and input.phase < PHASE_GUARDIANS_AWOKE then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_AWAKEN_EARTHEN_GUARDIAN, target = "self" })
        table.insert(actions, { action = "YELL", text = "Awake ye Earthen Guardians, protect the discs!" })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_GUARDIANS_AWOKE })
    end

    -- Awaken Vault Warders at 33% HP (once)
    if input.health_pct <= 0.33 and input.phase < PHASE_WARDERS_AWOKE then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_AWAKEN_VAULT_WARDER, target = "self" })
        table.insert(actions, { action = "YELL", text = "Rise my servants! Defend the maker!" })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_WARDERS_AWOKE })
    end

    return actions
end

function boss:OnKill(input)
    return {
        { action = "YELL", text = "I am the watcher of the vault. None may steal its secrets!" },
    }
end

RegisterCreatureAI(2748, boss)
