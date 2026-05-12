--[[
    @script_type: instance
    @map_id: 531
    @name: instance_temple_of_ahnqiraj

    Temple of Ahn'Qiraj (AQ40) - Instance Script
    Ported from vmangos instance_temple_of_ahnqiraj.cpp

    Manages:
    - 9 boss encounter states (Skeram through C'Thun)
    - Door/gate control for Skeram gate, Twins enter/exit doors
    - Add despawning when bosses are killed
    - Bug Trio death counter (all 3 must die for DONE)
    - Boss GUID tracking for C'Thun, Twins, Sartura, Eye of C'Thun, etc.
]]

-- ============================================================
-- Encounter type IDs
-- ============================================================
local TYPE_SKERAM   = 0
local TYPE_SARTURA  = 1
local TYPE_FANKRISS = 2
local TYPE_HUHURAN  = 3
local TYPE_TWINS    = 4
local TYPE_CTHUN    = 5
local TYPE_BUG_TRIO = 6
local TYPE_VISCIDUS = 7
local TYPE_OURO     = 8

-- Encounter state values
local NOT_STARTED = 0
local IN_PROGRESS = 1
local DONE        = 2
local SPECIAL     = 3  -- used for Bug Trio individual death counter

-- ============================================================
-- NPC entries
-- ============================================================
local NPC_SKERAM                = 15263
local NPC_BATTLEGUARD_SARTURA   = 15516
local NPC_FANKRISS              = 15510
local NPC_PRINCESS_HUHURAN      = 15509
local NPC_VEKLOR                = 15276
local NPC_VEKNILASH             = 15275
local NPC_EYE_OF_CTHUN          = 15589
local NPC_CTHUN                 = 15727
local NPC_KRI                   = 15511
local NPC_VEM                   = 15544
local NPC_PRINCESS_YAUJ         = 15543
local NPC_VISCIDUS               = 15299
local NPC_OURO                  = 15517
local NPC_OURO_SPAWNER          = 15957
local NPC_MASTERS_EYE           = 15963
local NPC_CTHUN_PORTAL          = 15896
local NPC_SARTURA_S_ROYAL_GUARD = 15984

-- Add NPC entries (despawned after boss kill)
local NPC_ANUBISATH_SENTINEL     = 15264
local NPC_OBSIDIAN_ERADICATOR    = 15262
local NPC_QIRAJI_BRAINWASHER     = 15247
local NPC_VEKNISS_GUARDIAN       = 15233
local NPC_VEKNISS_WARRIOR        = 15230
local NPC_VEKNISS_DRONE          = 15300
local NPC_VEKNISS_SOLDIER        = 15229
local NPC_VEKNISS_HIVE_CRAWLER   = 15240
local NPC_VEKNISS_WASP           = 15236
local NPC_QIRAJI_LASHER          = 15249
local NPC_VEKNISS_STINGER        = 15235
local NPC_ANUBISATH_DEFENDER     = 15277
local NPC_QIRAJI_SCARAB          = 15316
local NPC_QIRAJI_SCORPION        = 15317
local NPC_QIRAJI_MINDSLAYER      = 15246
local NPC_QIRAJI_SLAYER          = 15250
local NPC_QIRAJI_CHAMPION        = 15252
local NPC_ANUBISATH_WARDER       = 15311
local NPC_OBSIDIAN_NULLIFIER     = 15312
local NPC_OURO_SCARAB            = 15718

-- ============================================================
-- Game Object entries
-- ============================================================
local GO_SKERAM_GATE      = 180636
local GO_TWINS_ENTER_DOOR = 180634
local GO_TWINS_EXIT_DOOR  = 180635
local GO_GRASP_OF_CTHUN   = 180745

-- GUID data IDs for boss GUIDs
local DATA_SARTURA     = 100
local DATA_VEKLOR      = 101
local DATA_VEKNILASH   = 102
local DATA_EYE_OF_CTHUN = 103
local DATA_CTHUN       = 104
local DATA_MASTERS_EYE = 105
local DATA_OURO_SPAWNER = 106
local DATA_CTHUN_PORTAL = 107

-- Door GUID data IDs
local DATA_DOOR_SKERAM_GATE      = 110
local DATA_DOOR_TWINS_ENTER      = 111
local DATA_DOOR_TWINS_EXIT       = 112

-- ============================================================
-- Instance script table
-- ============================================================
local instance = {}

-- Bug Trio death counter (in-memory only, resets on wipe)
local bug_trio_deaths = 0

-- Called when a creature spawns in the instance.
function instance:OnCreatureCreate(input, entry, guid)
    local actions = {}

    -- Track boss GUIDs
    if entry == NPC_BATTLEGUARD_SARTURA then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_SARTURA, guid = guid })
        if input:GetData(TYPE_SARTURA) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end
    elseif entry == NPC_VEKLOR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_VEKLOR, guid = guid })
    elseif entry == NPC_VEKNILASH then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_VEKNILASH, guid = guid })
    elseif entry == NPC_EYE_OF_CTHUN then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_EYE_OF_CTHUN, guid = guid })
    elseif entry == NPC_CTHUN then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_CTHUN, guid = guid })
    elseif entry == NPC_MASTERS_EYE then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_MASTERS_EYE, guid = guid })
        -- Despawn Eye if Twins dialogue already done
        if input:GetData(TYPE_TWINS) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end
    elseif entry == NPC_OURO_SPAWNER then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_OURO_SPAWNER, guid = guid })
    elseif entry == NPC_CTHUN_PORTAL then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_CTHUN_PORTAL, guid = guid })

    -- Adds: despawn if their boss is already killed
    elseif entry == NPC_ANUBISATH_SENTINEL or entry == NPC_OBSIDIAN_ERADICATOR then
        if input:GetData(TYPE_SKERAM) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_QIRAJI_BRAINWASHER or entry == NPC_VEKNISS_GUARDIAN
        or entry == NPC_VEKNISS_WARRIOR or entry == NPC_SARTURA_S_ROYAL_GUARD then
        if input:GetData(TYPE_SARTURA) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_VEKNISS_DRONE or entry == NPC_VEKNISS_SOLDIER then
        if input:GetData(TYPE_FANKRISS) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_VEKNISS_HIVE_CRAWLER or entry == NPC_VEKNISS_WASP
        or entry == NPC_QIRAJI_LASHER or entry == NPC_VEKNISS_STINGER then
        if input:GetData(TYPE_HUHURAN) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_ANUBISATH_DEFENDER or entry == NPC_QIRAJI_SCARAB
        or entry == NPC_QIRAJI_SCORPION then
        if input:GetData(TYPE_TWINS) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_QIRAJI_MINDSLAYER or entry == NPC_QIRAJI_SLAYER
        or entry == NPC_QIRAJI_CHAMPION or entry == NPC_ANUBISATH_WARDER
        or entry == NPC_OBSIDIAN_NULLIFIER then
        if input:GetData(TYPE_CTHUN) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_OURO_SCARAB then
        -- Ouro Scarabs only exist during Ouro encounter; despawn otherwise
        if input:GetData(TYPE_OURO) ~= IN_PROGRESS then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end
    end

    return actions
end

-- Called when a creature dies in the instance.
function instance:OnCreatureDeath(input, entry, guid)
    local actions = {}

    if entry == NPC_SKERAM then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_SKERAM, value = DONE })
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_SKERAM_GATE })

    elseif entry == NPC_BATTLEGUARD_SARTURA then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_SARTURA, value = DONE })

    elseif entry == NPC_FANKRISS then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_FANKRISS, value = DONE })

    elseif entry == NPC_PRINCESS_HUHURAN then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_HUHURAN, value = DONE })
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_TWINS_ENTER })

    elseif entry == NPC_VEKLOR or entry == NPC_VEKNILASH then
        -- Either twin can trigger DONE; only update if not already set
        if input:GetData(TYPE_TWINS) ~= DONE then
            table.insert(actions, { action = "SET_DATA", data_id = TYPE_TWINS, value = DONE })
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_TWINS_ENTER })
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_TWINS_EXIT })
        end

    elseif entry == NPC_KRI or entry == NPC_VEM or entry == NPC_PRINCESS_YAUJ then
        -- Bug Trio: each death increments counter; all 3 must die for DONE
        bug_trio_deaths = bug_trio_deaths + 1
        if bug_trio_deaths >= 3 then
            table.insert(actions, { action = "SET_DATA", data_id = TYPE_BUG_TRIO, value = DONE })
            bug_trio_deaths = 0
        end

    elseif entry == NPC_VISCIDUS then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_VISCIDUS, value = DONE })

    elseif entry == NPC_OURO then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_OURO, value = DONE })

    elseif entry == NPC_CTHUN then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_CTHUN, value = DONE })
    end

    return actions
end

-- Called when a creature enters combat.
function instance:OnCreatureEnterCombat(input, entry, guid)
    local actions = {}

    if entry == NPC_BATTLEGUARD_SARTURA then
        if input:GetData(TYPE_SARTURA) == NOT_STARTED then
            table.insert(actions, { action = "SET_DATA", data_id = TYPE_SARTURA, value = IN_PROGRESS })
        end

    elseif entry == NPC_KRI or entry == NPC_VEM or entry == NPC_PRINCESS_YAUJ then
        if input:GetData(TYPE_BUG_TRIO) ~= DONE then
            table.insert(actions, { action = "SET_DATA", data_id = TYPE_BUG_TRIO, value = IN_PROGRESS })
            bug_trio_deaths = 0
        end
    end

    return actions
end

-- Called when a game object spawns in the instance.
function instance:OnGameObjectCreate(input, entry, guid)
    local actions = {}

    if entry == GO_SKERAM_GATE then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_SKERAM_GATE, guid = guid })
        if input:GetData(TYPE_SKERAM) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_SKERAM_GATE })
        end

    elseif entry == GO_TWINS_ENTER_DOOR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_TWINS_ENTER, guid = guid })
        if input:GetData(TYPE_HUHURAN) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_TWINS_ENTER })
        end

    elseif entry == GO_TWINS_EXIT_DOOR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_TWINS_EXIT, guid = guid })
        if input:GetData(TYPE_TWINS) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_DOOR_TWINS_EXIT })
        end
    end

    return actions
end

-- Called on instance load from database.
function instance:OnLoad(input)
    bug_trio_deaths = 0
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

RegisterInstanceScript(531, instance)
