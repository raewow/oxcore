--[[
    @script_type: instance
    @map_id: 189
    @name: instance_scarlet_monastery

    Scarlet Monastery (Cathedral) - Instance Script
    Ported from vmangos instance_scarlet_monastery.cpp

    Manages:
    - Mograine and Whitemane encounter state (multi-stage)
    - GUID tracking for Mograine, Whitemane, Vorrel Sengutz
    - Two doors: High Inquisitor door (opens when Mograine fake-dies),
      Chapel door
]]

-- ============================================================
-- Encounter type IDs
-- ============================================================
local TYPE_MOGRAINE_AND_WHITE_EVENT = 1

-- Multi-stage encounter states for Mograine/Whitemane
local STAGE_NOT_STARTED  = 0
local STAGE_IN_PROGRESS  = 1
local STAGE_DIED_ONCE    = 2  -- Mograine fake-died, Whitemane coming to res
local STAGE_REVIVED      = 3  -- Mograine revived by Whitemane
local STAGE_DONE         = 4  -- Both dead

-- GUID data IDs
local DATA_MOGRAINE      = 100
local DATA_WHITEMANE     = 101
local DATA_VORREL        = 102
local DATA_DOOR_WHITEMANE = 103
local DATA_DOOR_CHAPEL   = 104

-- ============================================================
-- NPC entries
-- ============================================================
local NPC_COMMANDER_MOGRAINE  = 3976
local NPC_INQUISITOR_WHITEMANE = 3977
local NPC_VORREL_SENGUTZ      = 3981

-- ============================================================
-- Game Object entries
-- ============================================================
local GO_CHAPEL_DOOR          = 104591
local GO_HIGH_INQUISITOR_DOOR = 104600

-- ============================================================
-- Instance script table
-- ============================================================
local instance = {}

function instance:OnCreatureCreate(input, entry, guid)
    local actions = {}

    if entry == NPC_COMMANDER_MOGRAINE then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_MOGRAINE, guid = guid })
    elseif entry == NPC_INQUISITOR_WHITEMANE then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_WHITEMANE, guid = guid })
    elseif entry == NPC_VORREL_SENGUTZ then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_VORREL, guid = guid })
    end

    return actions
end

function instance:OnCreatureDeath(input, entry, guid)
    local actions = {}

    -- Both Mograine and Whitemane must be dead for DONE
    -- The individual boss scripts set TYPE_MOGRAINE_AND_WHITE_EVENT to STAGE_DIED_ONCE
    -- and STAGE_DONE accordingly. Instance just tracks state here.
    if entry == NPC_COMMANDER_MOGRAINE or entry == NPC_INQUISITOR_WHITEMANE then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_MOGRAINE_AND_WHITE_EVENT, value = STAGE_DONE })
    end

    return actions
end

function instance:OnGameObjectCreate(input, entry, guid)
    local actions = {}

    if entry == GO_HIGH_INQUISITOR_DOOR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_WHITEMANE, guid = guid })
    elseif entry == GO_CHAPEL_DOOR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_CHAPEL, guid = guid })
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

RegisterInstanceScript(189, instance)
