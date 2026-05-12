--[[
    @script_type: instance
    @map_id: 70
    @name: instance_uldaman

    Uldaman - Instance Script
    Ported from vmangos instance_uldaman.cpp

    Manages:
    - 3 encounter states (Ironaya door, Stone Keepers, Archaedas)
    - GUID tracking for Archaedas, Ironaya, altar GOs, and doors
    - Keystone game object tracking
    - Ancient Vault door (opens after Ironaya seal door sequence)
]]

-- ============================================================
-- Encounter type IDs
-- ============================================================
local ULDAMAN_ENCOUNTER_IRONAYA_DOOR  = 0
local ULDAMAN_ENCOUNTER_STONE_KEEPERS = 1
local ULDAMAN_ENCOUNTER_ARCHAEDAS     = 2

-- Encounter state values
local NOT_STARTED = 0
local IN_PROGRESS = 1
local DONE        = 2

-- ============================================================
-- NPC entries
-- ============================================================
local NPC_ARCHAEDAS        = 2748
local NPC_STONE_KEEPER     = 4857
local NPC_IRONAYA          = 7228
local NPC_EARTHEN_CUSTODIAN = 7309
local NPC_EARTHEN_HALLSHAPER = 7077
local NPC_EARTHEN_GUARDIAN  = 7076
local NPC_VAULT_WARDER      = 10120

-- ============================================================
-- Game Object entries
-- ============================================================
local GO_ALTAR_ARCHAEDAS                = 133234
local GO_ALTAR_KEEPERS                  = 130511
local GO_IRONAYA_SEAL_DOOR              = 124372
local GO_ARCHAEDAS_TEMPLE_DOOR          = 141869
local GO_ALTAR_OF_THE_KEEPER_TEMPLE_DOOR = 124367
local GO_ANCIENT_VAULT_DOOR             = 124369
local GO_KEYSTONE                       = 124371

-- GUID data IDs
local DATA_ARCHAEDAS         = 100
local DATA_IRONAYA           = 101
local DATA_ALTAR_ARCHAEDAS   = 102
local DATA_ALTAR_KEEPERS     = 103
local DATA_IRONAYA_DOOR      = 104
local DATA_TEMPLE_DOOR       = 105
local DATA_KEEPER_DOOR       = 106
local DATA_VAULT_DOOR        = 107
local DATA_KEYSTONE          = 108

-- ============================================================
-- Instance script table
-- ============================================================
local instance = {}

function instance:OnCreatureCreate(input, entry, guid)
    local actions = {}

    if entry == NPC_ARCHAEDAS then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ARCHAEDAS, guid = guid })
    elseif entry == NPC_IRONAYA then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_IRONAYA, guid = guid })
    end

    return actions
end

function instance:OnCreatureDeath(input, entry, guid)
    local actions = {}

    if entry == NPC_IRONAYA then
        table.insert(actions, { action = "SET_DATA", data_id = ULDAMAN_ENCOUNTER_IRONAYA_DOOR, value = DONE })

    elseif entry == NPC_ARCHAEDAS then
        table.insert(actions, { action = "SET_DATA", data_id = ULDAMAN_ENCOUNTER_ARCHAEDAS, value = DONE })
        -- Open the Ancient Vault door (loot room)
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_VAULT_DOOR })
    end

    return actions
end

function instance:OnGameObjectCreate(input, entry, guid)
    local actions = {}

    if entry == GO_ALTAR_ARCHAEDAS then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ALTAR_ARCHAEDAS, guid = guid })
    elseif entry == GO_ALTAR_KEEPERS then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ALTAR_KEEPERS, guid = guid })
    elseif entry == GO_IRONAYA_SEAL_DOOR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_IRONAYA_DOOR, guid = guid })
    elseif entry == GO_ARCHAEDAS_TEMPLE_DOOR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_TEMPLE_DOOR, guid = guid })
    elseif entry == GO_ALTAR_OF_THE_KEEPER_TEMPLE_DOOR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_KEEPER_DOOR, guid = guid })
    elseif entry == GO_ANCIENT_VAULT_DOOR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_VAULT_DOOR, guid = guid })
        if input:GetData(ULDAMAN_ENCOUNTER_ARCHAEDAS) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_VAULT_DOOR })
        end
    elseif entry == GO_KEYSTONE then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_KEYSTONE, guid = guid })
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

RegisterInstanceScript(70, instance)
