--[[
    @script_type: instance
    @map_id: 249
    @name: instance_onyxia_lair

    Onyxia's Lair - Instance Script
    Ported from vmangos instance_onyxia_lair.cpp

    Manages:
    - Single boss encounter state for Onyxia
]]

-- ============================================================
-- Encounter type IDs
-- ============================================================
local DATA_ONYXIA_EVENT = 0

-- Encounter state values
local NOT_STARTED = 0
local IN_PROGRESS = 1
local DONE        = 2

-- ============================================================
-- NPC entries
-- ============================================================
local NPC_ONYXIA = 10184

-- ============================================================
-- Instance script table
-- ============================================================
local instance = {}

function instance:OnCreatureCreate(input, entry, guid)
    local actions = {}

    if entry == NPC_ONYXIA then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ONYXIA_EVENT, guid = guid })
    end

    return actions
end

function instance:OnCreatureDeath(input, entry, guid)
    local actions = {}

    if entry == NPC_ONYXIA then
        table.insert(actions, { action = "SET_DATA", data_id = DATA_ONYXIA_EVENT, value = DONE })
    end

    return actions
end

function instance:OnCreatureEnterCombat(input, entry, guid)
    local actions = {}

    if entry == NPC_ONYXIA then
        table.insert(actions, { action = "SET_DATA", data_id = DATA_ONYXIA_EVENT, value = IN_PROGRESS })
    end

    return actions
end

function instance:OnLoad(input)
    return {}
end

function instance:OnPlayerEnter(input, player_guid)
    return {}
end

function instance:Update(input)
    return {}
end

RegisterInstanceScript(249, instance)
