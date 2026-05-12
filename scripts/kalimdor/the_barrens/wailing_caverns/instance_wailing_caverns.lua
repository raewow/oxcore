--[[
    @script_type: instance
    @map_id: 43
    @name: instance_wailing_caverns

    Wailing Caverns - Instance Script
    Ported from vmangos instance_wailing_caverns.cpp

    Manages:
    - 6 encounter types (Anacondra, Cobrahn, Pythas, Serpentis,
      Disciple of Naralex, Mutanus the Devourer)
    - GUID tracking for Disciple, Naralex, Anacondra, Serpentis
    - When all 4 sleepers (Anacondra/Cobrahn/Pythas/Serpentis) are killed,
      Disciple of Naralex advances to SPECIAL state (gossip changes)
    - Darkmoon Faire chest (GO_DMF_CHEST) tracked but visibility
      controlled per-player via areatrigger (Phase D)
]]

-- ============================================================
-- Encounter type IDs
-- ============================================================
local TYPE_ANACONDRA   = 0
local TYPE_COBRAHN     = 1
local TYPE_PYTHAS      = 2
local TYPE_SERPENTIS   = 3
local TYPE_DISCIPLE    = 4
local TYPE_MUTANUS     = 5

-- Extra data ID (GUID slot)
local DATA_NARALEX     = 6

-- Encounter state values
local NOT_STARTED = 0
local IN_PROGRESS = 1
local DONE        = 2
local SPECIAL     = 3

-- ============================================================
-- NPC entries
-- ============================================================
local NPC_DISCIPLE_OF_NARALEX = 3678
local NPC_NARALEX             = 3679
local NPC_LORD_COBRAHN        = 3669
local NPC_LADY_ANACONDRA      = 3671
local NPC_LORD_PYTHAS         = 3670
local NPC_LORD_SERPENTIS      = 3673
local NPC_MUTANUS             = 5765

-- ============================================================
-- Game Object entries
-- ============================================================
local GO_DMF_CHEST = 180055

-- GUID data IDs
local DATA_DISCIPLE  = 100
local DATA_NARALEX_GUID = 101
local DATA_ANACONDRA = 102
local DATA_SERPENTIS = 103
local DATA_DMF_CHEST = 104

-- ============================================================
-- Module-level state
-- ============================================================
-- Track which sleepers have been killed to trigger Disciple gossip
local sleepers_killed = 0

-- ============================================================
-- Instance script table
-- ============================================================
local instance = {}

function instance:OnCreatureCreate(input, entry, guid)
    local actions = {}

    if entry == NPC_DISCIPLE_OF_NARALEX then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DISCIPLE, guid = guid })
    elseif entry == NPC_NARALEX then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_NARALEX_GUID, guid = guid })
    elseif entry == NPC_LADY_ANACONDRA then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ANACONDRA, guid = guid })
    elseif entry == NPC_LORD_SERPENTIS then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_SERPENTIS, guid = guid })
    end

    return actions
end

function instance:OnCreatureDeath(input, entry, guid)
    local actions = {}

    if entry == NPC_LADY_ANACONDRA then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_ANACONDRA, value = DONE })
        sleepers_killed = sleepers_killed + 1

    elseif entry == NPC_LORD_COBRAHN then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_COBRAHN, value = DONE })
        sleepers_killed = sleepers_killed + 1

    elseif entry == NPC_LORD_PYTHAS then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_PYTHAS, value = DONE })
        sleepers_killed = sleepers_killed + 1

    elseif entry == NPC_LORD_SERPENTIS then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_SERPENTIS, value = DONE })
        sleepers_killed = sleepers_killed + 1

    elseif entry == NPC_MUTANUS then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_MUTANUS, value = DONE })
        sleepers_killed = 0
        return actions
    end

    -- When all 4 sleepers are dead, advance Disciple to SPECIAL state
    if sleepers_killed >= 4
        and input:GetData(TYPE_ANACONDRA) == DONE
        and input:GetData(TYPE_COBRAHN) == DONE
        and input:GetData(TYPE_PYTHAS) == DONE
        and input:GetData(TYPE_SERPENTIS) == DONE
        and input:GetData(TYPE_DISCIPLE) == NOT_STARTED
    then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_DISCIPLE, value = SPECIAL })
    end

    return actions
end

function instance:OnGameObjectCreate(input, entry, guid)
    local actions = {}

    if entry == GO_DMF_CHEST then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DMF_CHEST, guid = guid })
    end

    return actions
end

function instance:OnLoad(input)
    sleepers_killed = 0
    return {}
end

function instance:OnPlayerEnter(input, player_guid)
    return {}
end

function instance:Update(input)
    return {}
end

RegisterInstanceScript(43, instance)
