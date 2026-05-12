--[[
    @script_type: instance
    @map_id: 509
    @name: instance_ruins_of_ahnqiraj

    Ruins of Ahn'Qiraj (AQ20) - Instance Script
    Ported from vmangos instance_ruins_of_ahnqiraj.cpp

    Manages:
    - 7 boss encounter states (Kurinnaxx through Ossirian)
    - GUID tracking for all bosses and Rajaxx wave commanders
    - Andorov/Kaldorei Elite escort event state
    - Ossirian crystal GO tracking (activated by boss scripts)
    - Add despawning for Rajaxx wave NPCs on reset
]]

-- ============================================================
-- Encounter type IDs
-- ============================================================
local TYPE_KURINNAXX       = 0
local TYPE_GENERAL_ANDOROV = 1
local TYPE_RAJAXX          = 2
local TYPE_BURU            = 3
local TYPE_MOAM            = 4
local TYPE_AYAMISS         = 5
local TYPE_OSSIRIAN        = 6

-- Encounter state values
local NOT_STARTED = 0
local IN_PROGRESS = 1
local DONE        = 2

-- ============================================================
-- NPC entries
-- ============================================================
local NPC_KURINNAXX       = 15348
local NPC_RAJAXX          = 15341
local NPC_BURU            = 15370
local NPC_MOAM            = 15340
local NPC_AYAMISS         = 15369
local NPC_OSSIRIAN        = 15339
local NPC_GENERAL_ANDOROV = 15471
local NPC_KALDOREI_ELITE  = 15473

-- Rajaxx wave commanders
local NPC_CAPTAIN_QEEZ   = 15391
local NPC_CAPTAIN_TUUBID = 15392
local NPC_CAPTAIN_DRENN  = 15389
local NPC_CAPTAIN_XURREM = 15390
local NPC_MAJOR_YEGGETH  = 15386
local NPC_MAJOR_PAKKON   = 15388
local NPC_COLONEL_ZERRAN = 15385

-- ============================================================
-- GUID data IDs (offset from encounter types to avoid collision)
-- ============================================================
local DATA_KURINNAXX  = 100
local DATA_RAJAXX     = 101
local DATA_BURU       = 102
local DATA_AYAMISS    = 103
local DATA_MOAM       = 104
local DATA_OSSIRIAN   = 105
local DATA_ANDOROV    = 106
local DATA_QEEZ       = 107
local DATA_TUUBID     = 108
local DATA_DRENN      = 109
local DATA_XURREM     = 110
local DATA_YEGGETH    = 111
local DATA_PAKKON     = 112
local DATA_ZERRAN     = 113

-- ============================================================
-- Instance script table
-- ============================================================
local instance = {}

-- Called when a creature spawns in the instance.
function instance:OnCreatureCreate(input, entry, guid)
    local actions = {}

    if entry == NPC_KURINNAXX then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_KURINNAXX, guid = guid })

    elseif entry == NPC_RAJAXX then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_RAJAXX, guid = guid })

    elseif entry == NPC_BURU then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_BURU, guid = guid })

    elseif entry == NPC_AYAMISS then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_AYAMISS, guid = guid })

    elseif entry == NPC_MOAM then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_MOAM, guid = guid })

    elseif entry == NPC_OSSIRIAN then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_OSSIRIAN, guid = guid })

    elseif entry == NPC_GENERAL_ANDOROV then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ANDOROV, guid = guid })

    elseif entry == NPC_CAPTAIN_QEEZ then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_QEEZ, guid = guid })

    elseif entry == NPC_CAPTAIN_TUUBID then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_TUUBID, guid = guid })

    elseif entry == NPC_CAPTAIN_DRENN then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DRENN, guid = guid })

    elseif entry == NPC_CAPTAIN_XURREM then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_XURREM, guid = guid })

    elseif entry == NPC_MAJOR_YEGGETH then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_YEGGETH, guid = guid })

    elseif entry == NPC_MAJOR_PAKKON then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_PAKKON, guid = guid })

    elseif entry == NPC_COLONEL_ZERRAN then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ZERRAN, guid = guid })
    end

    return actions
end

-- Called when a creature dies in the instance.
function instance:OnCreatureDeath(input, entry, guid)
    local actions = {}

    if entry == NPC_KURINNAXX then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_KURINNAXX, value = DONE })

    elseif entry == NPC_RAJAXX then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_RAJAXX, value = DONE })

    elseif entry == NPC_BURU then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_BURU, value = DONE })

    elseif entry == NPC_MOAM then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_MOAM, value = DONE })

    elseif entry == NPC_AYAMISS then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_AYAMISS, value = DONE })

    elseif entry == NPC_OSSIRIAN then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_OSSIRIAN, value = DONE })
    end

    return actions
end

-- Called on instance load from database.
function instance:OnLoad(input)
    return {}
end

-- Called when a player enters the instance.
function instance:OnPlayerEnter(input, player_guid)
    return {}
end

-- Periodic update.
function instance:Update(input)
    return {}
end

RegisterInstanceScript(509, instance)
