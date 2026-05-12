--[[
    @script_type: instance
    @map_id: 309
    @name: instance_zulgurub

    Zul'Gurub - Instance Script
    Ported from vmangos instance_zulgurub.cpp

    Manages:
    - 7 High Priest boss encounter states + Hakkar, Ohgan, random boss, Jindo, Gahzranka
    - Boss GUID tracking for Thekal's trinket (Lorkhan, Zath, Thekal), Jindo, Hakkar, Gahzranka
    - Add despawning for bosses already killed (HandleLoadCreature pattern)
    - Hakkar Power Stacks: each living High Priest adds 1 stack to Hakkar
      (actual spell-casting handled at boss AI level via CAST_SPELL actions)
    - Random world boss (Grilek/Hazzarah/Renataki/Wushoolay) tracking
]]

-- ============================================================
-- Encounter type IDs
-- ============================================================
local TYPE_HAKKAR_POWER      = 0   -- triggers Hakkar power stack update
local TYPE_ARLOKK            = 1
local TYPE_JEKLIK            = 2
local TYPE_VENOXIS           = 3
local TYPE_MARLI             = 4
local TYPE_OHGAN             = 5
local TYPE_THEKAL            = 6
local TYPE_THEKAL_DEATH_TIME = 7   -- timestamp: last fake death
local TYPE_THEKAL_REZ_TIME   = 8   -- timestamp: last rez started
local TYPE_HAKKAR            = 9
local TYPE_RANDOM_BOSS       = 10
local TYPE_JINDO             = 11
local TYPE_GAHZRANKA         = 12

-- GUID data IDs (offset to avoid collision with encounter types)
local DATA_JINDO       = 113
local DATA_LORKHAN     = 114
local DATA_THEKAL      = 115
local DATA_ZATH        = 116
local DATA_HAKKAR      = 117
local DATA_GAHZRANKA   = 118

-- Encounter state values
local NOT_STARTED = 0
local IN_PROGRESS = 1
local DONE        = 2

-- ============================================================
-- NPC entries
-- ============================================================
local NPC_LORKHAN              = 11347
local NPC_ZATH                 = 11348
local NPC_THEKAL               = 14509
local NPC_JINDO                = 11380
local NPC_HAKKAR               = 14834
local NPC_VENOXIS              = 14507
local NPC_ARLOKK               = 14515
local NPC_MARLI                = 14510
local NPC_JEKLIK               = 14517
local NPC_GAHZRANKA            = 15114
local NPC_RAZZASHI_SKITTERER   = 14880
local NPC_RAZZASHI_VENOMBROOD  = 14532
local NPC_HAKARI_SHADOWCASTER  = 11338
local NPC_RAZZASHI_BROODWIDOW  = 11370

-- Random boss entries (one spawned per week rotation)
local NPC_GRILEK    = 15082
local NPC_HAZZARAH  = 15083
local NPC_RENATAKI  = 15084
local NPC_WUSHOOLAY = 15085

-- ============================================================
-- Instance script table
-- ============================================================
local instance = {}

-- Called when a creature spawns in the instance.
-- Tracks boss GUIDs; despawns if already DONE.
function instance:OnCreatureCreate(input, entry, guid)
    local actions = {}

    if entry == NPC_LORKHAN then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_LORKHAN, guid = guid })
        if input:GetData(TYPE_THEKAL) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_ZATH then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ZATH, guid = guid })
        if input:GetData(TYPE_THEKAL) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_THEKAL then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_THEKAL, guid = guid })
        if input:GetData(TYPE_THEKAL) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_JINDO then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_JINDO, guid = guid })
        if input:GetData(TYPE_JINDO) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_HAKKAR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_HAKKAR, guid = guid })
        if input:GetData(TYPE_HAKKAR) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_VENOXIS then
        if input:GetData(TYPE_VENOXIS) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_ARLOKK then
        if input:GetData(TYPE_ARLOKK) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_MARLI then
        if input:GetData(TYPE_MARLI) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_JEKLIK then
        if input:GetData(TYPE_JEKLIK) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_GAHZRANKA then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GAHZRANKA, guid = guid })
        if input:GetData(TYPE_GAHZRANKA) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_GRILEK or entry == NPC_HAZZARAH
        or entry == NPC_RENATAKI or entry == NPC_WUSHOOLAY then
        if input:GetData(TYPE_RANDOM_BOSS) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end
    end

    return actions
end

-- Called when a creature dies in the instance.
function instance:OnCreatureDeath(input, entry, guid)
    local actions = {}

    if entry == NPC_ARLOKK then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_ARLOKK, value = DONE })

    elseif entry == NPC_JEKLIK then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_JEKLIK, value = DONE })

    elseif entry == NPC_VENOXIS then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_VENOXIS, value = DONE })

    elseif entry == NPC_MARLI then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_MARLI, value = DONE })

    elseif entry == NPC_THEKAL then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_THEKAL, value = DONE })

    elseif entry == NPC_HAKKAR then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_HAKKAR, value = DONE })

    elseif entry == NPC_JINDO then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_JINDO, value = DONE })

    elseif entry == NPC_GAHZRANKA then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_GAHZRANKA, value = DONE })

    elseif entry == NPC_GRILEK or entry == NPC_HAZZARAH
        or entry == NPC_RENATAKI or entry == NPC_WUSHOOLAY then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_RANDOM_BOSS, value = DONE })
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

RegisterInstanceScript(309, instance)
