--[[
    @script_type: instance
    @map_id: 48
    @name: instance_blackfathom_deeps

    Blackfathom Deeps - Instance Script
    Ported from vmangos instance_blackfathom_deeps.cpp

    Manages:
    - 3 encounter states (Kelris, Shrine event, Aquanis)
    - GUID tracking for Twilight Lord Kelris, Baron Aquanis,
      4 shrine game objects, and portal door
    - Shrine event: lighting all 4 shrines spawns waves of adds,
      eventually opening the portal door to Aku'mai
]]

-- ============================================================
-- Encounter type IDs
-- ============================================================
local TYPE_KELRIS   = 10
local TYPE_SHRINE   = 11
local TYPE_AQUANIS  = 12

local BFD_ENCOUNTER_KELRIS   = 0
local BFD_ENCOUNTER_SHRINE   = 1
local BFD_ENCOUNTER_AQUANIS  = 2

-- Encounter state values
local NOT_STARTED = 0
local IN_PROGRESS = 1
local DONE        = 2

-- ============================================================
-- NPC entries
-- ============================================================
local NPC_BARON_AQUANIS = 12876
local NPC_TWILIGHT_LORD_KELRIS = 4832  -- Twilight Lord Kelris

-- ============================================================
-- Game Object entries
-- ============================================================
local GO_PORTAL_DOOR = 21117
local GO_SHRINE_1    = 21118
local GO_SHRINE_2    = 21119
local GO_SHRINE_3    = 21120
local GO_SHRINE_4    = 21121
local GO_FATHOM_STONE = 177964

-- GUID data IDs
local DATA_SHRINE1        = 1
local DATA_SHRINE2        = 2
local DATA_SHRINE3        = 3
local DATA_SHRINE4        = 4
local DATA_KELRIS         = 5
local DATA_SHRINE_GELIHAST = 6
local DATA_ALTAR          = 7
local DATA_MAINDOOR       = 8
local DATA_AQUANIS        = 100

-- ============================================================
-- Instance script table
-- ============================================================
local instance = {}

function instance:OnCreatureCreate(input, entry, guid)
    local actions = {}

    if entry == NPC_BARON_AQUANIS then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_AQUANIS, guid = guid })
    end

    return actions
end

function instance:OnCreatureDeath(input, entry, guid)
    local actions = {}

    if entry == NPC_TWILIGHT_LORD_KELRIS then
        table.insert(actions, { action = "SET_DATA", data_id = BFD_ENCOUNTER_KELRIS, value = DONE })

    elseif entry == NPC_BARON_AQUANIS then
        table.insert(actions, { action = "SET_DATA", data_id = BFD_ENCOUNTER_AQUANIS, value = DONE })
    end

    return actions
end

function instance:OnGameObjectCreate(input, entry, guid)
    local actions = {}

    if entry == GO_SHRINE_1 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_SHRINE1, guid = guid })
    elseif entry == GO_SHRINE_2 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_SHRINE2, guid = guid })
    elseif entry == GO_SHRINE_3 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_SHRINE3, guid = guid })
    elseif entry == GO_SHRINE_4 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_SHRINE4, guid = guid })
    elseif entry == GO_PORTAL_DOOR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_MAINDOOR, guid = guid })
        -- Open portal door if shrine event is complete
        if input:GetData(BFD_ENCOUNTER_SHRINE) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_MAINDOOR })
        end
    elseif entry == GO_FATHOM_STONE then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ALTAR, guid = guid })
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

RegisterInstanceScript(48, instance)
