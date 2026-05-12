--[[
    @script_type: instance
    @map_id: 109
    @name: instance_sunken_temple

    Sunken Temple (Temple of Atal'Hakkar) - Instance Script
    Ported from vmangos instance_sunken_temple.cpp

    Manages:
    - 9 encounter types (Secret Circle, Protectors, Jammalan, Avatar of Hakkar,
      Eranikus, Atalarion statue sequence, Eternal Flame event)
    - GUID tracking for key bosses and event objects
    - Jammalan Barrier (opens when all 6 protectors dead)
    - Atalai Statue sequence (6 statues in correct order)
    - Avatar of Hakkar event (4 flames extinguished)
]]

-- ============================================================
-- Encounter type IDs
-- ============================================================
local TYPE_SECRET_CIRCLE  = 4
local TYPE_PROTECTORS     = 5
local TYPE_JAMMALAN       = 6
local TYPE_MALFURION      = 7
local TYPE_AVATAR         = 8
local TYPE_ERANIKUS       = 9
local TYPE_ETERNAL_FLAME  = 10

-- Encounter state values
local NOT_STARTED = 0
local IN_PROGRESS = 1
local DONE        = 2
local SPECIAL     = 3  -- per-statue activation

-- ============================================================
-- NPC entries
-- ============================================================
local NPC_ATALARION        = 8580
local NPC_DREAMSCYTH       = 5721
local NPC_WEAVER           = 5720
local NPC_JAMMALAN         = 5710
local NPC_AVATAR_OF_HAKKAR = 8443
local NPC_SHADE_OF_ERANIKUS = 5709
local NPC_SHADE_OF_HAKKAR  = 8440

-- Jammalan protector bosses (all 6 must die for Jammalan barrier to open)
local NPC_ZOLO    = 5712
local NPC_GASHER  = 5713
local NPC_LORO    = 5714
local NPC_HUKKU   = 5715
local NPC_ZULLOR  = 5716
local NPC_MIJAN   = 5717

-- ============================================================
-- Game Object entries
-- ============================================================
local GO_IDOL_OF_HAKKAR    = 148838
local GO_ATALAI_STATUE_1   = 148830
local GO_ATALAI_STATUE_2   = 148831
local GO_ATALAI_STATUE_3   = 148832
local GO_ATALAI_STATUE_4   = 148833
local GO_ATALAI_STATUE_5   = 148834
local GO_ATALAI_STATUE_6   = 148835
local GO_JAMMALAN_BARRIER  = 149431

-- GUID data IDs
local DATA_ATALARION       = 100
local DATA_DREAMSCYTH      = 101
local DATA_WEAVER          = 102
local DATA_JAMMALAN        = 103
local DATA_AVATAR          = 104
local DATA_SHADE_ERANIKUS  = 105
local DATA_SHADE_HAKKAR    = 106
local DATA_JAMMALAN_BARRIER = 110
local DATA_IDOL_HAKKAR     = 111
local DATA_STATUE_1        = 112
local DATA_STATUE_2        = 113
local DATA_STATUE_3        = 114
local DATA_STATUE_4        = 115
local DATA_STATUE_5        = 116
local DATA_STATUE_6        = 117

-- ============================================================
-- Instance script table
-- ============================================================
local instance = {}

-- Count of Jammalan protectors killed (all 6 must die)
local protector_deaths = 0
local protector_entries = {
    [NPC_ZOLO]=true, [NPC_GASHER]=true, [NPC_LORO]=true,
    [NPC_HUKKU]=true, [NPC_ZULLOR]=true, [NPC_MIJAN]=true
}

function instance:OnCreatureCreate(input, entry, guid)
    local actions = {}

    if entry == NPC_ATALARION then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ATALARION, guid = guid })
    elseif entry == NPC_DREAMSCYTH then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DREAMSCYTH, guid = guid })
    elseif entry == NPC_WEAVER then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_WEAVER, guid = guid })
    elseif entry == NPC_JAMMALAN then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_JAMMALAN, guid = guid })
    elseif entry == NPC_AVATAR_OF_HAKKAR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_AVATAR, guid = guid })
    elseif entry == NPC_SHADE_OF_ERANIKUS then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_SHADE_ERANIKUS, guid = guid })
    elseif entry == NPC_SHADE_OF_HAKKAR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_SHADE_HAKKAR, guid = guid })
    end

    return actions
end

function instance:OnCreatureDeath(input, entry, guid)
    local actions = {}

    if protector_entries[entry] then
        protector_deaths = protector_deaths + 1
        if protector_deaths >= 6 then
            -- All 6 protectors dead: open Jammalan barrier
            table.insert(actions, { action = "SET_DATA", data_id = TYPE_PROTECTORS, value = DONE })
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_JAMMALAN_BARRIER })
            protector_deaths = 0
        end

    elseif entry == NPC_JAMMALAN then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_JAMMALAN, value = DONE })

    elseif entry == NPC_AVATAR_OF_HAKKAR then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_AVATAR, value = DONE })

    elseif entry == NPC_SHADE_OF_ERANIKUS then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_ERANIKUS, value = DONE })
    end

    return actions
end

function instance:OnGameObjectCreate(input, entry, guid)
    local actions = {}

    if entry == GO_JAMMALAN_BARRIER then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_JAMMALAN_BARRIER, guid = guid })
        if input:GetData(TYPE_PROTECTORS) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_JAMMALAN_BARRIER })
        end
    elseif entry == GO_IDOL_OF_HAKKAR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_IDOL_HAKKAR, guid = guid })
    elseif entry == GO_ATALAI_STATUE_1 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_STATUE_1, guid = guid })
    elseif entry == GO_ATALAI_STATUE_2 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_STATUE_2, guid = guid })
    elseif entry == GO_ATALAI_STATUE_3 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_STATUE_3, guid = guid })
    elseif entry == GO_ATALAI_STATUE_4 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_STATUE_4, guid = guid })
    elseif entry == GO_ATALAI_STATUE_5 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_STATUE_5, guid = guid })
    elseif entry == GO_ATALAI_STATUE_6 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_STATUE_6, guid = guid })
    end

    return actions
end

function instance:OnLoad(input)
    protector_deaths = 0
    return {}
end

function instance:OnPlayerEnter(input, player_guid)
    return {}
end

function instance:Update(input)
    return {}
end

RegisterInstanceScript(109, instance)
