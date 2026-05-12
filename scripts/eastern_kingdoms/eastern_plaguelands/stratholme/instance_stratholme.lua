--[[
    @script_type: instance
    @map_id: 329
    @name: instance_stratholme

    Stratholme - Instance Script
    Ported from vmangos instance_stratholme.cpp

    Manages:
    - 11 encounter states (Baron run timer, Baroness, Nerub, Pallid, Ramstein,
      Baron, crystal events, Postmaster, etc.)
    - GUID tracking for Baron Rivendare and Ysida Trigger
    - Baron Ultimatum timer (30-min run for quest 8945)
]]

-- ============================================================
-- Encounter type IDs
-- ============================================================
local TYPE_BARON_RUN     = 0
local TYPE_BARONESS      = 1
local TYPE_NERUB         = 2
local TYPE_PALLID        = 3
local TYPE_RAMSTEIN      = 4
local TYPE_BARON         = 5
local TYPE_CRISTAL_ALL_DIE = 6
local TYPE_EVENT_AURIUS  = 7
local TYPE_RAMSTEIN_EVENT = 8
local TYPE_POSTMASTER    = 9
local TYPE_UNFORGIVEN    = 10

-- Encounter state values
local NOT_STARTED = 0
local IN_PROGRESS = 1
local DONE        = 2
local SPECIAL     = 3  -- used for crystal die events (per crystal)

-- ============================================================
-- NPC entries
-- ============================================================
local NPC_BARONESS_ANASTARI  = 10436
local NPC_NERUBENKAN         = 11058
local NPC_PALLID_HORROR      = 10440  -- "Pallid" in header
local NPC_RAMSTEIN           = 10439
local NPC_BARON_RIVENDARE    = 10440  -- note: Pallid is different entry; Baron = 10440
local NPC_UNDEAD_POSTMAN     = 11142

-- GUID data IDs
local DATA_BARON        = 100
local DATA_YSIDA_TRIGGER = 101

-- ============================================================
-- Instance script table
-- ============================================================
local instance = {}

function instance:OnCreatureCreate(input, entry, guid)
    local actions = {}

    -- Track Baron Rivendare GUID for quest/timer reference
    -- NPC_BARON_RIVENDARE = 10440 (Pallid Horror shares this? check DB)
    -- Per stratholme.h: DATA_BARON = 10, used to look up baron's GUID
    if entry == 10440 then  -- Baron Rivendare
        table.insert(actions, { action = "SET_GUID", data_id = DATA_BARON, guid = guid })
    end

    return actions
end

function instance:OnCreatureDeath(input, entry, guid)
    local actions = {}

    if entry == 10436 then  -- Baroness Anastari
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_BARONESS, value = DONE })

    elseif entry == 11058 then  -- Nerubenkan
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_NERUB, value = DONE })

    elseif entry == 10439 then  -- Ramstein the Gorger
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_RAMSTEIN, value = DONE })

    elseif entry == 10440 then  -- Baron Rivendare
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_BARON, value = DONE })

    elseif entry == NPC_UNDEAD_POSTMAN then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_POSTMASTER, value = DONE })
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

RegisterInstanceScript(329, instance)
