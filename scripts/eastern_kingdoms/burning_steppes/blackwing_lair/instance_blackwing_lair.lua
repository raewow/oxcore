--[[
    @script_type: instance
    @map_id: 469
    @name: instance_blackwing_lair

    Blackwing Lair - Instance Script
    Ported from vmangos instance_blackwing_lair.cpp

    Manages:
    - 8 boss encounter states (Razorgore through Nefarian)
    - Door/gate control per boss room
    - Add despawning when relevant bosses are killed
    - Razorgore egg counter (all 30 eggs destroyed = phase complete)
    - Vael event (Technicians aggro zone)
]]

-- ============================================================
-- Encounter type IDs
-- ============================================================
local TYPE_RAZORGORE  = 0
local TYPE_VAELASTRASZ = 1
local TYPE_LASHLAYER  = 2
local TYPE_FIREMAW    = 3
local TYPE_EBONROC    = 4
local TYPE_FLAMEGOR   = 5
local TYPE_CHROMAGGUS = 6
local TYPE_NEFARIAN   = 7
local TYPE_VAEL_EVENT = 8
local TYPE_SCEPTER_RUN = 9

-- GUID data IDs (stored separately, offset from encounter IDs)
local DATA_RAZORGORE_GUID      = 100
local DATA_VAELASTRASZ_GUID    = 101
local DATA_LASHLAYER_GUID      = 102
local DATA_FIREMAW_GUID        = 103
local DATA_EBONROC_GUID        = 104
local DATA_FLAMEGOR_GUID       = 105
local DATA_CHROMAGGUS_GUID     = 106
local DATA_NEFARIUS_GUID       = 107  -- Lord Nefarian (NPC 10162)
local DATA_NEFARIAN_GUID       = 108  -- Nefarian (NPC 11583)
local DATA_GRETOK_GUID         = 109
local DATA_TRIGGER_GUID        = 110  -- Orb of Domination trigger NPC
local DATA_ORB_DOMINATION_GUID = 111

-- Special data slots
local DATA_EGG         = 120  -- egg state (IN_PROGRESS per egg, FAIL on wipe)
local DATA_HOW_EGG     = 121  -- count of destroyed eggs
local DATA_CHROM_BREATH = 122 -- random chromaggus breath set (0-19)
local DATA_NEF_COLOR   = 123  -- random nefarian class call color (0-19)

-- Door GUID data IDs
local DATA_DOOR_RAZORGORE_ENTER  = 130
local DATA_DOOR_RAZORGORE_EXIT   = 131
local DATA_DOOR_VAELASTRASZ      = 132
local DATA_DOOR_LASHLAYER        = 133
local DATA_DOOR_CHROMAGGUS_ENTER = 134
local DATA_DOOR_CHROMAGGUS_EXIT  = 135
local DATA_DOOR_CHROMAGGUS_SIDE  = 136
local DATA_DOOR_NEFARIAN         = 137

-- Encounter state values
local NOT_STARTED = 0
local IN_PROGRESS = 1
local DONE        = 2
local FAIL        = 3

local EGGS_COUNT  = 30

-- ============================================================
-- NPC entries
-- ============================================================
local NPC_RAZORGORE               = 12435
local NPC_VAELASTRASZ             = 13020
local NPC_LASHLAYER               = 12017
local NPC_FIREMAW                 = 11983
local NPC_EBONROC                 = 14601
local NPC_FLAMEGOR                = 11981
local NPC_CHROMAGGUS              = 14020
local NPC_NEFARIAN                = 11583
local NPC_LORD_NEFARIAN           = 10162
local NPC_ORB_OF_DOMINATION       = 14453
local NPC_GRETHOK_THE_CONTROLLER  = 12557
local NPC_BLACKWING_GUARDSMAN     = 14456
local NPC_BLACKWING_LEGGIONAIRE   = 12416
local NPC_BLACKWING_MAGE          = 12420
local NPC_DEATH_TALON_DRAGONSPAWN = 12422
local NPC_DEATH_TALON_CAPTAIN     = 12467
local NPC_DEATH_TALON_SEETHER     = 12464
local NPC_DEATH_TALON_WYRMKIN     = 12465
local NPC_DEATH_TALON_FLAMESCALE  = 12463
local NPC_DEATH_TALON_HATCHER     = 12468
local NPC_BLACKWING_TASKMASTER    = 12458
local NPC_BLACKWING_TECHNICIAN    = 13996
local NPC_CORRUPTED_GREEN_WHELP   = 14023
local NPC_CORRUPTED_RED_WHELP     = 14022
local NPC_CORRUPTED_BLUE_WHELP    = 14024
local NPC_CORRUPTED_BRONZE_WHELP  = 14025
local NPC_BLACKWING_WARLOCK       = 12459
local NPC_DEATH_TALON_OVERSEER    = 12461
local NPC_BLACKWING_SPELLBINDER   = 12457
local NPC_DEATH_TALON_WYRMGUARD   = 12460

-- ============================================================
-- Game Object entries
-- ============================================================
local GO_DOOR_RAZORGORE_ENTER    = 176964
local GO_DOOR_RAZORGORE_EXIT     = 176965
local GO_DOOR_NEFARIAN           = 176966
local GO_DOOR_CHROMAGGUS_ENTER   = 179115
local GO_DOOR_CHROMAGGUS_SIDE    = 179116
local GO_DOOR_CHROMAGGUS_EXIT    = 179117
local GO_DOOR_VAELASTRASZ        = 179364
local GO_DOOR_LASHLAYER          = 179365
local GO_BLACK_DRAGON_EGG        = 177807
local GO_ORB_OF_DOMINATION       = 177808
local GO_SUPPRESSION_ENGINE      = 179784

-- ============================================================
-- Instance script table
-- ============================================================
local instance = {}

-- Called when a creature spawns in the instance.
function instance:OnCreatureCreate(input, entry, guid)
    local actions = {}

    if entry == NPC_RAZORGORE then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_RAZORGORE_GUID, guid = guid })

    elseif entry == NPC_VAELASTRASZ then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_VAELASTRASZ_GUID, guid = guid })
        if input:GetData(TYPE_VAELASTRASZ) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_LASHLAYER then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_LASHLAYER_GUID, guid = guid })

    elseif entry == NPC_FIREMAW then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_FIREMAW_GUID, guid = guid })

    elseif entry == NPC_EBONROC then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_EBONROC_GUID, guid = guid })

    elseif entry == NPC_FLAMEGOR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_FLAMEGOR_GUID, guid = guid })

    elseif entry == NPC_CHROMAGGUS then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_CHROMAGGUS_GUID, guid = guid })

    elseif entry == NPC_NEFARIAN then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_NEFARIAN_GUID, guid = guid })

    elseif entry == NPC_LORD_NEFARIAN then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_NEFARIUS_GUID, guid = guid })
        if input:GetData(TYPE_NEFARIAN) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_ORB_OF_DOMINATION then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_TRIGGER_GUID, guid = guid })

    elseif entry == NPC_GRETHOK_THE_CONTROLLER then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GRETOK_GUID, guid = guid })
        if input:GetData(TYPE_RAZORGORE) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_BLACKWING_GUARDSMAN then
        if input:GetData(TYPE_RAZORGORE) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_CORRUPTED_GREEN_WHELP
        or entry == NPC_CORRUPTED_BLUE_WHELP
        or entry == NPC_CORRUPTED_RED_WHELP
        or entry == NPC_CORRUPTED_BRONZE_WHELP then
        local lashlayer_state = input:GetData(TYPE_LASHLAYER)
        if lashlayer_state == DONE or lashlayer_state == IN_PROGRESS then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_DEATH_TALON_SEETHER
        or entry == NPC_DEATH_TALON_WYRMKIN
        or entry == NPC_DEATH_TALON_FLAMESCALE
        or entry == NPC_DEATH_TALON_CAPTAIN
        or entry == NPC_DEATH_TALON_HATCHER
        or entry == NPC_BLACKWING_TASKMASTER then
        if input:GetData(TYPE_LASHLAYER) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_BLACKWING_TECHNICIAN
        or entry == NPC_BLACKWING_WARLOCK
        or entry == NPC_BLACKWING_SPELLBINDER
        or entry == NPC_DEATH_TALON_WYRMGUARD
        or entry == NPC_DEATH_TALON_OVERSEER then
        local firemaw_done  = input:GetData(TYPE_FIREMAW)  == DONE
        local ebonroc_done  = input:GetData(TYPE_EBONROC)  == DONE
        local flamegor_done = input:GetData(TYPE_FLAMEGOR) == DONE
        if firemaw_done and ebonroc_done and flamegor_done then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end
    end

    return actions
end

-- Called when a creature dies in the instance.
function instance:OnCreatureDeath(input, entry, guid)
    local actions = {}

    if entry == NPC_RAZORGORE then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_RAZORGORE, value = DONE })
        -- Open exit door
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_RAZORGORE_EXIT })
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_RAZORGORE_ENTER })

    elseif entry == NPC_VAELASTRASZ then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_VAELASTRASZ, value = DONE })
        -- Open the next door (Razorgore exit reopens, Vael door opens)
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_RAZORGORE_EXIT })
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_VAELASTRASZ })

    elseif entry == NPC_LASHLAYER then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_LASHLAYER, value = DONE })
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_LASHLAYER })

    elseif entry == NPC_FIREMAW then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_FIREMAW, value = DONE })

    elseif entry == NPC_EBONROC then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_EBONROC, value = DONE })

    elseif entry == NPC_FLAMEGOR then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_FLAMEGOR, value = DONE })

    elseif entry == NPC_CHROMAGGUS then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_CHROMAGGUS, value = DONE })
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_CHROMAGGUS_EXIT })

    elseif entry == NPC_NEFARIAN then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_NEFARIAN, value = DONE })
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_NEFARIAN })
    end

    return actions
end

-- Called when a creature enters combat in the instance.
function instance:OnCreatureEnterCombat(input, entry, guid)
    local actions = {}

    if entry == NPC_GRETHOK_THE_CONTROLLER or entry == NPC_BLACKWING_GUARDSMAN then
        -- Razorgore encounter begins; pull Razorgore into combat if not already
        if input:GetData(TYPE_RAZORGORE) == NOT_STARTED then
            table.insert(actions, { action = "SET_DATA", data_id = TYPE_RAZORGORE, value = IN_PROGRESS })
        end
        table.insert(actions, { action = "SET_COMBAT_WITH_ZONE" })

    elseif entry == NPC_RAZORGORE then
        -- Grethok should also aggro
        table.insert(actions, { action = "SET_COMBAT_WITH_ZONE" })

    elseif entry == NPC_VAELASTRASZ then
        -- Close Razorgore exit door while Vael is in progress
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_VAELASTRASZ, value = IN_PROGRESS })
        table.insert(actions, { action = "CLOSE_DOOR", data_id = DATA_DOOR_RAZORGORE_EXIT })
    end

    return actions
end

-- Called when a game object spawns in the instance.
function instance:OnGameObjectCreate(input, entry, guid)
    local actions = {}

    if entry == GO_DOOR_RAZORGORE_ENTER then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_RAZORGORE_ENTER, guid = guid })

    elseif entry == GO_DOOR_RAZORGORE_EXIT then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_RAZORGORE_EXIT, guid = guid })
        if input:GetData(TYPE_RAZORGORE) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_RAZORGORE_EXIT })
        end

    elseif entry == GO_DOOR_NEFARIAN then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_NEFARIAN, guid = guid })

    elseif entry == GO_DOOR_CHROMAGGUS_ENTER then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_CHROMAGGUS_ENTER, guid = guid })

    elseif entry == GO_DOOR_CHROMAGGUS_SIDE then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_CHROMAGGUS_SIDE, guid = guid })
        if input:GetData(TYPE_CHROMAGGUS) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_CHROMAGGUS_SIDE })
        end

    elseif entry == GO_DOOR_CHROMAGGUS_EXIT then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_CHROMAGGUS_EXIT, guid = guid })
        if input:GetData(TYPE_CHROMAGGUS) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_CHROMAGGUS_EXIT })
        end

    elseif entry == GO_DOOR_VAELASTRASZ then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_VAELASTRASZ, guid = guid })
        if input:GetData(TYPE_VAELASTRASZ) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_VAELASTRASZ })
        end

    elseif entry == GO_DOOR_LASHLAYER then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_LASHLAYER, guid = guid })
        if input:GetData(TYPE_LASHLAYER) == DONE then
            -- Lashlayer door is deleted when done in vmangos; respawn with 0 duration to hide it
            table.insert(actions, { action = "RESPAWN_GAMEOBJECT", guid = guid, duration = 0 })
        end

    elseif entry == GO_ORB_OF_DOMINATION then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ORB_DOMINATION_GUID, guid = guid })
        if input:GetData(TYPE_RAZORGORE) == DONE then
            table.insert(actions, { action = "DESPAWN_GAMEOBJECT", guid = guid })
        end

    elseif entry == GO_BLACK_DRAGON_EGG then
        if input:GetData(TYPE_RAZORGORE) == DONE then
            table.insert(actions, { action = "DESPAWN_GAMEOBJECT", guid = guid })
        end

    elseif entry == GO_SUPPRESSION_ENGINE then
        if input:GetData(TYPE_LASHLAYER) == DONE then
            table.insert(actions, { action = "DESPAWN_GAMEOBJECT", guid = guid })
        end
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

-- Periodic update (not heavily used for BWL).
function instance:Update(input)
    return {}
end

RegisterInstanceScript(469, instance)
