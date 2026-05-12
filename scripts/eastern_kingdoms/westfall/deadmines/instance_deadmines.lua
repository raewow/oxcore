--[[
    @script_type: instance
    @map_id: 36
    @name: instance_deadmines

    Deadmines - Instance Script
    Ported from vmangos instance_deadmines.cpp

    Manages:
    - 1 encounter type (Defias end door)
    - GUID tracking for Iron Clad Door, Defias Cannon, and boss NPCs
    - Three doors that open in sequence after Rhahkzor/Gilnid kills
]]

-- ============================================================
-- Encounter type IDs
-- ============================================================
local TYPE_DEFIAS_ENDDOOR = 1

-- Encounter state values
local NOT_STARTED = 0
local IN_PROGRESS = 1
local DONE        = 2

-- ============================================================
-- NPC entries
-- ============================================================
local NPC_MR_SMITE  = 646
local NPC_RHAHKZOR  = 644
local NPC_GILDNID   = 1763
local NPC_SNEED     = 643

-- ============================================================
-- Game Object entries
-- ============================================================
local GO_DOOR1       = 13965
local GO_DOOR2       = 16400
local GO_DOOR3       = 16399
local GO_DOOR_LEVER  = 101833
local GO_IRON_CLAD   = 16397
local GO_DEFIAS_CANNON = 16398

-- GUID data IDs
local DATA_IRON_CLAD   = 100
local DATA_CANNON      = 101
local DATA_MR_SMITE    = 102
local DATA_RHAHKZOR    = 103
local DATA_GILDNID     = 104
local DATA_DOOR1       = 110
local DATA_DOOR2       = 111
local DATA_DOOR3       = 112
local DATA_DOOR_LEVER  = 113

-- ============================================================
-- Instance script table
-- ============================================================
local instance = {}

function instance:OnCreatureCreate(input, entry, guid)
    local actions = {}

    if entry == NPC_RHAHKZOR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_RHAHKZOR, guid = guid })
    elseif entry == NPC_GILDNID then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GILDNID, guid = guid })
    elseif entry == NPC_MR_SMITE then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_MR_SMITE, guid = guid })
    end

    return actions
end

function instance:OnCreatureDeath(input, entry, guid)
    local actions = {}

    if entry == NPC_RHAHKZOR then
        -- Open door1 after Rhahkzor
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR1 })

    elseif entry == NPC_GILDNID then
        -- Open door2 after Gilnid
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR2 })
    end

    return actions
end

function instance:OnGameObjectCreate(input, entry, guid)
    local actions = {}

    if entry == GO_IRON_CLAD then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_IRON_CLAD, guid = guid })
    elseif entry == GO_DEFIAS_CANNON then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_CANNON, guid = guid })
    elseif entry == GO_DOOR1 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR1, guid = guid })
    elseif entry == GO_DOOR2 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR2, guid = guid })
    elseif entry == GO_DOOR3 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR3, guid = guid })
    elseif entry == GO_DOOR_LEVER then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_LEVER, guid = guid })
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

RegisterInstanceScript(36, instance)
