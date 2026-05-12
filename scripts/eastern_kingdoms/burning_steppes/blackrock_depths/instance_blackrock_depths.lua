--[[
    @script_type: instance
    @map_id: 230
    @name: instance_blackrock_depths

    Blackrock Depths - Instance Script
    Ported from vmangos instance_blackrock_depths.cpp

    Manages:
    - 10 encounter types (Ring of Law, Vault, Rocknot, Tomb of Seven, etc.)
    - GUID tracking for boss NPCs and key game objects
    - Door/gate control for arena, shadow vault, tomb, throne room
    - Flamelash dwarf rune game object tracking
]]

-- ============================================================
-- Encounter type IDs
-- ============================================================
local TYPE_RING_OF_LAW   = 0
local TYPE_VAULT         = 1
local TYPE_ROCKNOT       = 2
local TYPE_TOMB_OF_SEVEN = 3
local TYPE_LYCEUM        = 4
local TYPE_IRON_HALL     = 5
local TYPE_THUNDERBREW   = 6
local TYPE_RELIC_COFFER  = 7
local TYPE_DOOMGRIP      = 8
local TYPE_RIBBLY        = 9
local TYPE_FLAMELASH     = 10

-- Encounter state values
local NOT_STARTED = 0
local IN_PROGRESS = 1
local DONE        = 2

-- ============================================================
-- NPC entries
-- ============================================================
local NPC_EMPEROR              = 9019
local NPC_PRINCESS             = 8929
local NPC_PHALANX              = 9502
local NPC_HATEREL              = 9034
local NPC_ANGERREL             = 9035
local NPC_VILEREL              = 9036
local NPC_GLOOMREL             = 9037
local NPC_SEETHREL             = 9038
local NPC_DOOMREL              = 9039
local NPC_DOPEREL              = 9040
local NPC_THELDREN             = 16059
local NPC_GRIMSTONE            = 10096
local NPC_MAGMUS               = 9938
local NPC_PLUGGER_SPAZZRING    = 9499
local NPC_PRIVATE_ROCKNOT      = 9503
local NPC_MISTRESS_NAGMARA     = 9500
local NPC_FLAMELASH            = 9156
local NPC_GOLEM_LORD_ARGELMACH = 8983
local NPC_RIBBLY_S_CRONY       = 10043

-- ============================================================
-- Game Object entries
-- ============================================================
local GO_ARENA1             = 161525
local GO_ARENA2             = 161522
local GO_ARENA3             = 161524
local GO_ARENA4             = 161523
local GO_SHADOW_LOCK        = 161460
local GO_SHADOW_MECHANISM   = 161461
local GO_SHADOW_GIANT_DOOR  = 157923
local GO_SHADOW_DUMMY       = 161516
local GO_BAR_KEG_SHOT       = 170607
local GO_BAR_KEG_TRAP       = 171941
local GO_BAR_DOOR           = 170571
local GO_TOMB_ENTER         = 170576
local GO_TOMB_EXIT          = 170577
local GO_LYCEUM             = 170558
local GO_GOLEM_ROOM_N       = 170573
local GO_GOLEM_ROOM_S       = 170574
local GO_THRONE_ROOM        = 170575
local GO_SPECTRAL_CHALICE   = 164869
local GO_CHEST_SEVEN        = 169243
local GO_SECRET_DOOR        = 174553
local GO_ARENA_SPOILS       = 181074

-- Flamelash dwarf runes
local GO_DWARF_RUNE_A01 = 170578
local GO_DWARF_RUNE_B01 = 170579
local GO_DWARF_RUNE_C01 = 170580
local GO_DWARF_RUNE_D01 = 170581
local GO_DWARF_RUNE_E01 = 170582
local GO_DWARF_RUNE_F01 = 170583
local GO_DWARF_RUNE_G01 = 170584

-- Jail door GOs
local GO_JAIL_DOOR_SUPPLY  = 170561
local GO_JAIL_DOOR_DUGHAL  = 170562
local GO_JAIL_DOOR_TOBIAS  = 170566
local GO_JAIL_DOOR_CREST   = 170567
local GO_JAIL_DOOR_JAZ     = 170568
local GO_JAIL_DOOR_SHILL   = 170569

-- ============================================================
-- GUID data IDs (offset to avoid collision with encounter types)
-- ============================================================
local DATA_EMPEROR          = 100
local DATA_PRINCESS         = 101
local DATA_PHALANX          = 102
local DATA_HATEREL          = 103
local DATA_ANGERREL         = 104
local DATA_VILEREL          = 105
local DATA_GLOOMREL         = 106
local DATA_SEETHREL         = 107
local DATA_DOOMREL          = 108
local DATA_DOPEREL          = 109
local DATA_THELDREN         = 110
local DATA_GRIMSTONE        = 111
local DATA_MAGMUS           = 112
local DATA_PLUGGER          = 113
local DATA_ROCKNOT          = 114
local DATA_NAGMARA          = 115
local DATA_FLAMELASH        = 116
local DATA_ARGELMACH        = 117

-- Door GUID data IDs
local DATA_DOOR_ARENA1          = 130
local DATA_DOOR_ARENA2          = 131
local DATA_DOOR_ARENA3          = 132
local DATA_DOOR_ARENA4          = 133
local DATA_DOOR_SHADOW_LOCK     = 134
local DATA_DOOR_SHADOW_MECH     = 135
local DATA_DOOR_SHADOW_GIANT    = 136
local DATA_DOOR_SHADOW_DUMMY    = 137
local DATA_DOOR_BAR_KEG         = 138
local DATA_DOOR_BAR_KEG_TRAP    = 139
local DATA_DOOR_BAR_DOOR        = 140
local DATA_DOOR_TOMB_ENTER      = 141
local DATA_DOOR_TOMB_EXIT       = 142
local DATA_DOOR_LYCEUM          = 143
local DATA_DOOR_GOLEM_N         = 144
local DATA_DOOR_GOLEM_S         = 145
local DATA_DOOR_THRONE          = 146
local DATA_GO_CHALICE           = 147
local DATA_GO_CHEST_SEVEN       = 148
local DATA_GO_SECRET_DOOR       = 149
local DATA_GO_ARENA_SPOILS      = 150

-- Rune GUID data IDs
local DATA_RUNE_A = 160
local DATA_RUNE_B = 161
local DATA_RUNE_C = 162
local DATA_RUNE_D = 163
local DATA_RUNE_E = 164
local DATA_RUNE_F = 165
local DATA_RUNE_G = 166

-- ============================================================
-- Instance script table
-- ============================================================
local instance = {}

-- Called when a creature spawns in the instance.
function instance:OnCreatureCreate(input, entry, guid)
    local actions = {}

    if entry == NPC_EMPEROR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_EMPEROR, guid = guid })

    elseif entry == NPC_PRINCESS then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_PRINCESS, guid = guid })

    elseif entry == NPC_PHALANX then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_PHALANX, guid = guid })

    elseif entry == NPC_HATEREL then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_HATEREL, guid = guid })

    elseif entry == NPC_ANGERREL then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ANGERREL, guid = guid })

    elseif entry == NPC_VILEREL then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_VILEREL, guid = guid })

    elseif entry == NPC_GLOOMREL then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GLOOMREL, guid = guid })

    elseif entry == NPC_SEETHREL then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_SEETHREL, guid = guid })

    elseif entry == NPC_DOOMREL then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOMREL, guid = guid })

    elseif entry == NPC_DOPEREL then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOPEREL, guid = guid })

    elseif entry == NPC_THELDREN then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_THELDREN, guid = guid })

    elseif entry == NPC_GRIMSTONE then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GRIMSTONE, guid = guid })

    elseif entry == NPC_MAGMUS then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_MAGMUS, guid = guid })

    elseif entry == NPC_PLUGGER_SPAZZRING then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_PLUGGER, guid = guid })

    elseif entry == NPC_PRIVATE_ROCKNOT then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ROCKNOT, guid = guid })

    elseif entry == NPC_MISTRESS_NAGMARA then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_NAGMARA, guid = guid })

    elseif entry == NPC_FLAMELASH then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_FLAMELASH, guid = guid })

    elseif entry == NPC_GOLEM_LORD_ARGELMACH then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ARGELMACH, guid = guid })
    end

    return actions
end

-- Called when a creature dies in the instance.
function instance:OnCreatureDeath(input, entry, guid)
    local actions = {}

    if entry == NPC_THELDREN then
        -- Ring of Law challenger boss killed -> DONE
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_RING_OF_LAW, value = DONE })
        -- Open all arena doors
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_ARENA1 })
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_ARENA2 })
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_ARENA3 })
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_ARENA4 })

    elseif entry == NPC_FLAMELASH then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_FLAMELASH, value = DONE })

    elseif entry == NPC_MAGMUS then
        -- Iron Hall complete; open throne room door
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_IRON_HALL, value = DONE })
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_THRONE })

    elseif entry == NPC_EMPEROR then
        -- Throne room complete
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_IRON_HALL, value = DONE })
    end

    return actions
end

-- Called when a game object spawns in the instance.
function instance:OnGameObjectCreate(input, entry, guid)
    local actions = {}

    if entry == GO_ARENA1 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_ARENA1, guid = guid })
    elseif entry == GO_ARENA2 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_ARENA2, guid = guid })
    elseif entry == GO_ARENA3 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_ARENA3, guid = guid })
    elseif entry == GO_ARENA4 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_ARENA4, guid = guid })
    elseif entry == GO_SHADOW_LOCK then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_SHADOW_LOCK, guid = guid })
    elseif entry == GO_SHADOW_MECHANISM then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_SHADOW_MECH, guid = guid })
    elseif entry == GO_SHADOW_GIANT_DOOR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_SHADOW_GIANT, guid = guid })
    elseif entry == GO_SHADOW_DUMMY then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_SHADOW_DUMMY, guid = guid })
    elseif entry == GO_BAR_KEG_SHOT then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_BAR_KEG, guid = guid })
    elseif entry == GO_BAR_KEG_TRAP then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_BAR_KEG_TRAP, guid = guid })
    elseif entry == GO_BAR_DOOR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_BAR_DOOR, guid = guid })
    elseif entry == GO_TOMB_ENTER then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_TOMB_ENTER, guid = guid })
    elseif entry == GO_TOMB_EXIT then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_TOMB_EXIT, guid = guid })
        if input:GetData(TYPE_TOMB_OF_SEVEN) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_TOMB_EXIT })
        end
    elseif entry == GO_LYCEUM then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_LYCEUM, guid = guid })
    elseif entry == GO_GOLEM_ROOM_N then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_GOLEM_N, guid = guid })
    elseif entry == GO_GOLEM_ROOM_S then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_GOLEM_S, guid = guid })
    elseif entry == GO_THRONE_ROOM then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_THRONE, guid = guid })
    elseif entry == GO_SPECTRAL_CHALICE then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GO_CHALICE, guid = guid })
    elseif entry == GO_CHEST_SEVEN then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GO_CHEST_SEVEN, guid = guid })
    elseif entry == GO_SECRET_DOOR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GO_SECRET_DOOR, guid = guid })
    elseif entry == GO_ARENA_SPOILS then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GO_ARENA_SPOILS, guid = guid })

    -- Flamelash dwarf runes
    elseif entry == GO_DWARF_RUNE_A01 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_RUNE_A, guid = guid })
    elseif entry == GO_DWARF_RUNE_B01 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_RUNE_B, guid = guid })
    elseif entry == GO_DWARF_RUNE_C01 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_RUNE_C, guid = guid })
    elseif entry == GO_DWARF_RUNE_D01 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_RUNE_D, guid = guid })
    elseif entry == GO_DWARF_RUNE_E01 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_RUNE_E, guid = guid })
    elseif entry == GO_DWARF_RUNE_F01 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_RUNE_F, guid = guid })
    elseif entry == GO_DWARF_RUNE_G01 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_RUNE_G, guid = guid })
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

RegisterInstanceScript(230, instance)
