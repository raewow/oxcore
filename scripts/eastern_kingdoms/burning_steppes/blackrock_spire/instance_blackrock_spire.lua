--[[
    @script_type: instance
    @map_id: 229
    @name: instance_blackrock_spire

    Blackrock Spire (LBRS/UBRS) - Instance Script
    Ported from vmangos instance_blackrock_spire.cpp

    Manages:
    - 8 encounter types (Room Event, Emberseer, Flamewreath, Stadium/Gyth,
      Valthalak, UBRS Door Event, Solakar Rookery, Drakkisath)
    - GUID tracking for key bosses and all event GOs
    - 7 room rune GOs for the Emberseer room event
    - UBRS door event with 6 braziers
    - Stadium event: Gyth arena with Lord Victor Nefarius
    - Drakkisath two-door sequence
]]

-- ============================================================
-- Encounter type IDs
-- ============================================================
local TYPE_ROOM_EVENT        = 0
local TYPE_EMBERSEER         = 1
local TYPE_FLAMEWREATH       = 2
local TYPE_STADIUM           = 3
local TYPE_VALTHALAK         = 4
local TYPE_EVENT_DOOR_UBRS   = 5
local TYPE_SOLAKAR           = 6
local TYPE_DRAKKISATH        = 7

-- Encounter state values
local NOT_STARTED = 0
local IN_PROGRESS = 1
local DONE        = 2

-- ============================================================
-- NPC entries
-- ============================================================
local NPC_SCARSHIELD_INFILTRATOR = 10299
local NPC_BLACKHAND_SUMMONER     = 9818
local NPC_BLACKHAND_VETERAN      = 9819
local NPC_LORD_VICTOR_NEFARIUS   = 10162
local NPC_REND_BLACKHAND         = 10429
local NPC_GYTH                   = 10339
local NPC_SOLAKAR                = 10264
local NPC_DRAKKISATH             = 10363
local NPC_THE_BEAST              = 10430

-- ============================================================
-- Game Object entries
-- ============================================================
-- Emberseer room
local GO_EMBERSEER_IN        = 175244
local GO_DOORS               = 175705   -- combat door (controlled by boss script)
local GO_EMBERSEER_OUT       = 175153
-- Stadium (Gyth arena)
local GO_GYTH_ENTRY_DOOR     = 164726
local GO_GYTH_COMBAT_DOOR    = 175185   -- per-wave, controlled by boss script
local GO_GYTH_EXIT_DOOR      = 175186
-- Drakkisath
local GO_DRAKKISATH_DOOR1    = 175946
local GO_DRAKKISATH_DOOR2    = 175947
-- Misc
local GO_BLACKROCK_ALTAR     = 175706
-- Room runes (7 rooms)
local GO_ROOM_1_RUNE         = 175197
local GO_ROOM_2_RUNE         = 175199
local GO_ROOM_3_RUNE         = 175195
local GO_ROOM_4_RUNE         = 175200
local GO_ROOM_5_RUNE         = 175198
local GO_ROOM_6_RUNE         = 175196
local GO_ROOM_7_RUNE         = 175194
-- UBRS door event
local GO_DOOR_UBRS           = 164725
local GO_BRAZIER01           = 175528
local GO_BRAZIER02           = 175529
local GO_BRAZIER03           = 175530
local GO_BRAZIER04           = 175531
local GO_BRAZIER05           = 175532
local GO_BRAZIER06           = 175533
-- Rookery / Solakar
local GO_ROOKERY_EGG         = 175124
local GO_FATHER_FLAME        = 175245

-- GUID data IDs
local DATA_NEFARIUS          = 100
local DATA_REND              = 101
local DATA_GYTH              = 102
local DATA_INFILTRATOR       = 103
local DATA_DRAKKISATH        = 104
local DATA_BEAST             = 105
local DATA_EMBERSEER_IN      = 110
local DATA_DOORS             = 111
local DATA_EMBERSEER_OUT     = 112
local DATA_GYTH_ENTRY_DOOR   = 113
local DATA_GYTH_EXIT_DOOR    = 114
local DATA_DRAKKISATH_DOOR1  = 115
local DATA_DRAKKISATH_DOOR2  = 116
local DATA_ALTAR             = 117
local DATA_ROOM_1_RUNE       = 120
local DATA_ROOM_2_RUNE       = 121
local DATA_ROOM_3_RUNE       = 122
local DATA_ROOM_4_RUNE       = 123
local DATA_ROOM_5_RUNE       = 124
local DATA_ROOM_6_RUNE       = 125
local DATA_ROOM_7_RUNE       = 126
local DATA_DOOR_UBRS         = 130
local DATA_BRAZIER01         = 131
local DATA_BRAZIER02         = 132
local DATA_BRAZIER03         = 133
local DATA_BRAZIER04         = 134
local DATA_BRAZIER05         = 135
local DATA_BRAZIER06         = 136
local DATA_FATHER_FLAME      = 140

-- ============================================================
-- Instance script table
-- ============================================================
local instance = {}

function instance:OnCreatureCreate(input, entry, guid)
    local actions = {}

    if entry == NPC_LORD_VICTOR_NEFARIUS then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_NEFARIUS, guid = guid })
    elseif entry == NPC_REND_BLACKHAND then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_REND, guid = guid })
    elseif entry == NPC_GYTH then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GYTH, guid = guid })
    elseif entry == NPC_SCARSHIELD_INFILTRATOR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_INFILTRATOR, guid = guid })
    elseif entry == NPC_DRAKKISATH then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DRAKKISATH, guid = guid })
    elseif entry == NPC_THE_BEAST then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_BEAST, guid = guid })
    end

    return actions
end

function instance:OnCreatureDeath(input, entry, guid)
    local actions = {}

    if entry == NPC_GYTH then
        -- Stadium event complete: open exit door
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_STADIUM, value = DONE })
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_GYTH_EXIT_DOOR })

    elseif entry == NPC_SOLAKAR then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_SOLAKAR, value = DONE })

    elseif entry == NPC_DRAKKISATH then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_DRAKKISATH, value = DONE })
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DRAKKISATH_DOOR1 })
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DRAKKISATH_DOOR2 })
    end

    return actions
end

function instance:OnGameObjectCreate(input, entry, guid)
    local actions = {}

    if entry == GO_EMBERSEER_IN then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_EMBERSEER_IN, guid = guid })
        if input:GetData(TYPE_ROOM_EVENT) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_EMBERSEER_IN })
        end

    elseif entry == GO_DOORS then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOORS, guid = guid })

    elseif entry == GO_EMBERSEER_OUT then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_EMBERSEER_OUT, guid = guid })
        if input:GetData(TYPE_EMBERSEER) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_EMBERSEER_OUT })
        end

    elseif entry == GO_GYTH_ENTRY_DOOR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GYTH_ENTRY_DOOR, guid = guid })

    elseif entry == GO_GYTH_EXIT_DOOR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GYTH_EXIT_DOOR, guid = guid })
        if input:GetData(TYPE_STADIUM) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_GYTH_EXIT_DOOR })
        end

    elseif entry == GO_DRAKKISATH_DOOR1 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DRAKKISATH_DOOR1, guid = guid })
        if input:GetData(TYPE_DRAKKISATH) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DRAKKISATH_DOOR1 })
        end

    elseif entry == GO_DRAKKISATH_DOOR2 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DRAKKISATH_DOOR2, guid = guid })
        if input:GetData(TYPE_DRAKKISATH) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DRAKKISATH_DOOR2 })
        end

    elseif entry == GO_BLACKROCK_ALTAR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ALTAR, guid = guid })

    elseif entry == GO_ROOM_1_RUNE then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ROOM_1_RUNE, guid = guid })
    elseif entry == GO_ROOM_2_RUNE then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ROOM_2_RUNE, guid = guid })
    elseif entry == GO_ROOM_3_RUNE then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ROOM_3_RUNE, guid = guid })
    elseif entry == GO_ROOM_4_RUNE then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ROOM_4_RUNE, guid = guid })
    elseif entry == GO_ROOM_5_RUNE then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ROOM_5_RUNE, guid = guid })
    elseif entry == GO_ROOM_6_RUNE then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ROOM_6_RUNE, guid = guid })
    elseif entry == GO_ROOM_7_RUNE then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ROOM_7_RUNE, guid = guid })

    elseif entry == GO_DOOR_UBRS then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_UBRS, guid = guid })
        if input:GetData(TYPE_EVENT_DOOR_UBRS) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_UBRS })
        end

    elseif entry == GO_BRAZIER01 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_BRAZIER01, guid = guid })
        if input:GetData(TYPE_EVENT_DOOR_UBRS) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_BRAZIER01 })
        end
    elseif entry == GO_BRAZIER02 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_BRAZIER02, guid = guid })
        if input:GetData(TYPE_EVENT_DOOR_UBRS) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_BRAZIER02 })
        end
    elseif entry == GO_BRAZIER03 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_BRAZIER03, guid = guid })
        if input:GetData(TYPE_EVENT_DOOR_UBRS) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_BRAZIER03 })
        end
    elseif entry == GO_BRAZIER04 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_BRAZIER04, guid = guid })
        if input:GetData(TYPE_EVENT_DOOR_UBRS) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_BRAZIER04 })
        end
    elseif entry == GO_BRAZIER05 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_BRAZIER05, guid = guid })
        if input:GetData(TYPE_EVENT_DOOR_UBRS) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_BRAZIER05 })
        end
    elseif entry == GO_BRAZIER06 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_BRAZIER06, guid = guid })
        if input:GetData(TYPE_EVENT_DOOR_UBRS) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_BRAZIER06 })
        end

    elseif entry == GO_FATHER_FLAME then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_FATHER_FLAME, guid = guid })
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

RegisterInstanceScript(229, instance)
