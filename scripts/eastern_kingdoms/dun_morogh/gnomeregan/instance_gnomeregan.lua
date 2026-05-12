--[[
    @script_type: instance
    @map_id: 90
    @name: instance_gnomeregan

    Gnomeregan - Instance Script
    Ported from vmangos instance_gnomeregan.cpp

    Manages:
    - 2 boss encounter states (Grubbis, Thermaplugg)
    - GUID tracking for Blastmaster Shortfuse and Alarm-A-Bot 2600
    - Cave-in GO tracking (north/south)
    - Final Chamber door
    - 6 Gnome Face bomb trigger GOs
]]

-- ============================================================
-- Encounter type IDs
-- ============================================================
local TYPE_GRUBBIS      = 0
local TYPE_THERMAPLUGG  = 1

-- Encounter state values
local NOT_STARTED = 0
local IN_PROGRESS = 1
local DONE        = 2

-- ============================================================
-- NPC entries
-- ============================================================
local NPC_BLASTMASTER_SHORTFUSE = 7998
local NPC_ALARM_A_BOMB_2600     = 7897
local NPC_GRUBBIS               = 7979
local NPC_THERMAPLUGG           = 7800

-- ============================================================
-- Game Object entries
-- ============================================================
local GO_CAVE_IN_NORTH   = 146085
local GO_CAVE_IN_SOUTH   = 146086
local GO_THE_FINAL_CHAMBER = 142207
local GO_GNOME_FACE_1    = 142211
local GO_GNOME_FACE_2    = 142210
local GO_GNOME_FACE_3    = 142209
local GO_GNOME_FACE_4    = 142208
local GO_GNOME_FACE_5    = 142213
local GO_GNOME_FACE_6    = 142212

-- GUID data IDs
local DATA_BLASTMASTER    = 100
local DATA_ALARM_BOT      = 101
local DATA_CAVE_IN_NORTH  = 102
local DATA_CAVE_IN_SOUTH  = 103
local DATA_FINAL_CHAMBER  = 104
local DATA_GNOME_FACE_1   = 105
local DATA_GNOME_FACE_2   = 106
local DATA_GNOME_FACE_3   = 107
local DATA_GNOME_FACE_4   = 108
local DATA_GNOME_FACE_5   = 109
local DATA_GNOME_FACE_6   = 110

-- ============================================================
-- Instance script table
-- ============================================================
local instance = {}

function instance:OnCreatureCreate(input, entry, guid)
    local actions = {}

    if entry == NPC_BLASTMASTER_SHORTFUSE then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_BLASTMASTER, guid = guid })
    elseif entry == NPC_ALARM_A_BOMB_2600 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ALARM_BOT, guid = guid })
    end

    return actions
end

function instance:OnCreatureDeath(input, entry, guid)
    local actions = {}

    if entry == NPC_GRUBBIS then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_GRUBBIS, value = DONE })
    elseif entry == NPC_THERMAPLUGG then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_THERMAPLUGG, value = DONE })
    end

    return actions
end

function instance:OnGameObjectCreate(input, entry, guid)
    local actions = {}

    if entry == GO_CAVE_IN_NORTH then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_CAVE_IN_NORTH, guid = guid })
    elseif entry == GO_CAVE_IN_SOUTH then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_CAVE_IN_SOUTH, guid = guid })
    elseif entry == GO_THE_FINAL_CHAMBER then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_FINAL_CHAMBER, guid = guid })
    elseif entry == GO_GNOME_FACE_1 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GNOME_FACE_1, guid = guid })
    elseif entry == GO_GNOME_FACE_2 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GNOME_FACE_2, guid = guid })
    elseif entry == GO_GNOME_FACE_3 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GNOME_FACE_3, guid = guid })
    elseif entry == GO_GNOME_FACE_4 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GNOME_FACE_4, guid = guid })
    elseif entry == GO_GNOME_FACE_5 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GNOME_FACE_5, guid = guid })
    elseif entry == GO_GNOME_FACE_6 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GNOME_FACE_6, guid = guid })
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

RegisterInstanceScript(90, instance)
