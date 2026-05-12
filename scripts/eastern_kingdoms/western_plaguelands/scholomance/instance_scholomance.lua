--[[
    @script_type: instance
    @map_id: 289
    @name: instance_scholomance

    Scholomance - Instance Script
    Ported from vmangos instance_scholomance.cpp

    Manages:
    - 7 boss encounter states (Gandling, Theolen, Malicia, Illuciabarov,
      Alexeibarov, Polkelt, Ravenian) + Kirtonos + Darkreaver
    - Gate GOs for each boss room (open when boss is killed)
    - GUID tracking for Vectus and Marduke (Viewing Room event)
    - Viewing Room door (TYPE_VIEWING_ROOM_DOOR)
]]

-- ============================================================
-- Encounter type IDs
-- ============================================================
local TYPE_GANDLING          = 0
local TYPE_THEOLEN           = 1
local TYPE_MALICIA           = 2
local TYPE_ILLUCIABAROV      = 3
local TYPE_ALEXEIBAROV       = 4
local TYPE_POLKELT           = 5
local TYPE_RAVENIAN          = 6
local TYPE_KIRTONOS          = 7
local TYPE_VIEWING_ROOM_DOOR = 14
local TYPE_DARKREAVER        = 15

-- Encounter state values
local NOT_STARTED = 0
local IN_PROGRESS = 1
local DONE        = 2

-- ============================================================
-- NPC entries
-- ============================================================
local NPC_KIRTONOS = 10506
local NPC_GANDLING = 1853
local NPC_VECTUS   = 10432
local NPC_MARDUKE  = 10433

-- ============================================================
-- Game Object entries
-- ============================================================
local GO_GATE_KIRTONOS     = 175570
local GO_GATE_GANDLING     = 177374
local GO_GATE_MALICIA      = 177375
local GO_GATE_THEOLEN      = 177377
local GO_GATE_POLKELT      = 177376
local GO_GATE_RAVENIAN     = 177372
local GO_GATE_BAROV        = 177373
local GO_GATE_ILLUCIA      = 177371
local GO_VIEWING_ROOM_DOOR = 175167

-- GUID data IDs
local DATA_GATE_KIRTONOS     = 100
local DATA_GATE_GANDLING     = 101
local DATA_GATE_MALICIA      = 102
local DATA_GATE_THEOLEN      = 103
local DATA_GATE_POLKELT      = 104
local DATA_GATE_RAVENIAN     = 105
local DATA_GATE_BAROV        = 106
local DATA_GATE_ILLUCIA      = 107
local DATA_VIEWING_ROOM_DOOR = 108
local DATA_VECTUS            = 110
local DATA_MARDUKE           = 111

-- ============================================================
-- Instance script table
-- ============================================================
local instance = {}

function instance:OnCreatureCreate(input, entry, guid)
    local actions = {}

    if entry == NPC_VECTUS then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_VECTUS, guid = guid })
    elseif entry == NPC_MARDUKE then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_MARDUKE, guid = guid })
    end

    return actions
end

function instance:OnCreatureDeath(input, entry, guid)
    local actions = {}

    if entry == NPC_KIRTONOS then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_KIRTONOS, value = DONE })
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_GATE_KIRTONOS })

    elseif entry == NPC_GANDLING then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_GANDLING, value = DONE })
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_GATE_GANDLING })
    end

    return actions
end

function instance:OnGameObjectCreate(input, entry, guid)
    local actions = {}

    if entry == GO_GATE_KIRTONOS then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GATE_KIRTONOS, guid = guid })
        if input:GetData(TYPE_KIRTONOS) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_GATE_KIRTONOS })
        end

    elseif entry == GO_GATE_GANDLING then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GATE_GANDLING, guid = guid })
        if input:GetData(TYPE_GANDLING) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_GATE_GANDLING })
        end

    elseif entry == GO_GATE_MALICIA then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GATE_MALICIA, guid = guid })
        if input:GetData(TYPE_MALICIA) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_GATE_MALICIA })
        end

    elseif entry == GO_GATE_THEOLEN then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GATE_THEOLEN, guid = guid })
        if input:GetData(TYPE_THEOLEN) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_GATE_THEOLEN })
        end

    elseif entry == GO_GATE_POLKELT then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GATE_POLKELT, guid = guid })
        if input:GetData(TYPE_POLKELT) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_GATE_POLKELT })
        end

    elseif entry == GO_GATE_RAVENIAN then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GATE_RAVENIAN, guid = guid })
        if input:GetData(TYPE_RAVENIAN) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_GATE_RAVENIAN })
        end

    elseif entry == GO_GATE_BAROV then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GATE_BAROV, guid = guid })
        if input:GetData(TYPE_ALEXEIBAROV) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_GATE_BAROV })
        end

    elseif entry == GO_GATE_ILLUCIA then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GATE_ILLUCIA, guid = guid })
        if input:GetData(TYPE_ILLUCIABAROV) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_GATE_ILLUCIA })
        end

    elseif entry == GO_VIEWING_ROOM_DOOR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_VIEWING_ROOM_DOOR, guid = guid })
        if input:GetData(TYPE_VIEWING_ROOM_DOOR) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_VIEWING_ROOM_DOOR })
        end
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

RegisterInstanceScript(289, instance)
