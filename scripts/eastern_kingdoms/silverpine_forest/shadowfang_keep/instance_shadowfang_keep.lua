--[[
    @script_type: instance
    @map_id: 33
    @name: instance_shadowfang_keep

    Shadowfang Keep - Instance Script
    Ported from vmangos instance_shadowfang_keep.cpp

    Manages:
    - 5 encounter types (Free NPC, Rethilgore, Fenrus, Nandos, Voidwalker count)
    - GUID tracking for key NPCs and doors
    - Three doors: Courtyard door (quest), Sorcerer door (Fenrus death),
      Arugal door (Nandos death)
]]

-- ============================================================
-- Encounter type IDs
-- ============================================================
local TYPE_FREE_NPC    = 1
local TYPE_RETHILGORE  = 2
local TYPE_FENRUS      = 3
local TYPE_NANDOS      = 4
local TYPE_INTRO       = 5
local TYPE_VOIDWALKER  = 6

-- Encounter state values
local NOT_STARTED = 0
local IN_PROGRESS = 1
local DONE        = 2

-- ============================================================
-- NPC entries
-- ============================================================
local NPC_BARON_SILVERLAINE  = 3887
local NPC_CMD_SPRINGVALE     = 4278
local NPC_ASH                = 3850  -- prisoner NPC
local NPC_ADA                = 3849  -- prisoner NPC
local NPC_ARUGAL             = 10000
local NPC_ARCHMAGE_ARUGAL    = 4275
local NPC_FENRUS             = 4274
local NPC_VINCENT            = 4444
local NPC_NANDOS             = 3927
local NPC_WOLF_GUARD         = 3854

-- ============================================================
-- Game Object entries
-- ============================================================
local GO_COURTYARD_DOOR = 18895
local GO_SORCERER_DOOR  = 18972
local GO_ARUGAL_DOOR    = 18971
local GO_ARUGAL_FOCUS   = 18973

-- GUID data IDs
local DATA_ASH            = 100
local DATA_ADA            = 101
local DATA_BARON_SILVER   = 102
local DATA_CMD_SPRINGVALE = 103
local DATA_FENRUS         = 104
local DATA_VINCENT        = 105
local DATA_NANDOS         = 106
local DATA_DOOR_COURTYARD = 110
local DATA_DOOR_SORCERER  = 111
local DATA_DOOR_ARUGAL    = 112
local DATA_ARUGAL_FOCUS   = 113

-- ============================================================
-- Instance script table
-- ============================================================
local instance = {}

function instance:OnCreatureCreate(input, entry, guid)
    local actions = {}

    if entry == NPC_ASH then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ASH, guid = guid })
    elseif entry == NPC_ADA then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ADA, guid = guid })
    elseif entry == NPC_BARON_SILVERLAINE then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_BARON_SILVER, guid = guid })
    elseif entry == NPC_CMD_SPRINGVALE then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_CMD_SPRINGVALE, guid = guid })
    elseif entry == NPC_FENRUS then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_FENRUS, guid = guid })
    elseif entry == NPC_VINCENT then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_VINCENT, guid = guid })
    elseif entry == NPC_NANDOS then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_NANDOS, guid = guid })
    end

    return actions
end

function instance:OnCreatureDeath(input, entry, guid)
    local actions = {}

    if entry == NPC_FENRUS then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_FENRUS, value = DONE })
        -- Open sorcerer door (Archmage Arugal's area)
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_SORCERER })

    elseif entry == NPC_NANDOS then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_NANDOS, value = DONE })
        -- Open Arugal's final door
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_ARUGAL })
    end

    return actions
end

function instance:OnGameObjectCreate(input, entry, guid)
    local actions = {}

    if entry == GO_COURTYARD_DOOR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_COURTYARD, guid = guid })
    elseif entry == GO_SORCERER_DOOR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_SORCERER, guid = guid })
        if input:GetData(TYPE_FENRUS) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_SORCERER })
        end
    elseif entry == GO_ARUGAL_DOOR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_ARUGAL, guid = guid })
        if input:GetData(TYPE_NANDOS) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_ARUGAL })
        end
    elseif entry == GO_ARUGAL_FOCUS then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ARUGAL_FOCUS, guid = guid })
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

RegisterInstanceScript(33, instance)
