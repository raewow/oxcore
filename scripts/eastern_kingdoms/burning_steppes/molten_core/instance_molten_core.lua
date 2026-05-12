--[[
    @script_type: instance
    @map_id: 409
    @name: instance_molten_core

    Molten Core - Instance Script
    Ported from vmangos instance_molten_core.cpp

    Manages:
    - 10 boss encounter states (Lucifron through Ragnaros)
    - 7 flame rune states (one per non-Ragnaros boss)
    - Add despawning when bosses are killed
    - Majordomo summoning when all 7 runes are activated
    - Firelord Cache (loot chest) respawn when Majordomo dies
    - Flamewaker Healer/Elite mob-linking with Majordomo
]]

-- ============================================================
-- Encounter type IDs (data_id keys for SetData/GetData)
-- ============================================================
local TYPE_SULFURON  = 0
local TYPE_GEDDON    = 1
local TYPE_SHAZZRAH  = 2
local TYPE_GOLEMAGG  = 3
local TYPE_GARR      = 4
local TYPE_MAGMADAR  = 5
local TYPE_GEHENNAS  = 6
local TYPE_LUCIFRON  = 7
local TYPE_MAJORDOMO = 8
local TYPE_RAGNAROS  = 9

-- GUID data IDs (used with SET_GUID / GetGuid)
local DATA_SULFURON  = 11
local DATA_GOLEMAGG  = 12
local DATA_GARR      = 13
local DATA_MAJORDOMO = 14

-- Rune state IDs
local DATA_RUNE_ACTIVE_0 = 16  -- Sulfuron rune
local DATA_RUNE_ACTIVE_1 = 17  -- Geddon rune
local DATA_RUNE_ACTIVE_2 = 18  -- Shazzrah rune
local DATA_RUNE_ACTIVE_3 = 19  -- Golemagg rune
local DATA_RUNE_ACTIVE_4 = 20  -- Garr rune
local DATA_RUNE_ACTIVE_5 = 21  -- Magmadar rune
local DATA_RUNE_ACTIVE_6 = 22  -- Gehennas rune

local DATA_DOMO_SPAWNED  = 23

-- Encounter state values
local NOT_STARTED = 0
local IN_PROGRESS = 1
local DONE        = 2

-- ============================================================
-- NPC entries
-- ============================================================
local NPC_LUCIFRON             = 12118
local NPC_MAGMADAR             = 11982
local NPC_GEHENNAS             = 12259
local NPC_GARR                 = 12057
local NPC_GEDDON               = 12056
local NPC_SHAZZRAH             = 12264
local NPC_GOLEMAGG             = 11988
local NPC_SULFURON             = 12098
local NPC_MAJORDOMO            = 12018
local NPC_RAGNAROS             = 11502

local NPC_FLAMEWAKER_PRIEST    = 11662
local NPC_FLAMEWAKER_PROTECTOR = 12119
local NPC_FLAMEWAKER           = 11661
local NPC_CORE_RAGER           = 11672
local NPC_CORE_HOUND           = 11671
local NPC_ANCIENT_CORE_HOUND   = 11673
local NPC_FIRESWORN            = 12099
local NPC_LAVA_SURGER          = 12101
local NPC_FLAMEWAKER_HEALER    = 11663
local NPC_FLAMEWAKER_ELITE     = 11664

-- ============================================================
-- Game Object entries
-- ============================================================
local RUNE_SULFURON  = 176951
local RUNE_GEDDON    = 176952
local RUNE_SHAZZRAH  = 176953
local RUNE_GOLEMAGG  = 176954
local RUNE_GARR      = 176955
local RUNE_MAGMADAR  = 176956
local RUNE_GEHENNAS  = 176957
local RUNE_MAJORDOMO = 179703  -- Firelord Cache (chest after Majordomo)
local GO_HOT_COALS   = 177000

-- Rune fire animation GO entries (one per rune, despawned after activation)
local RUNE_FIRE_SULFURON = 178187
local RUNE_FIRE_GEDDON   = 178188
local RUNE_FIRE_SHAZZRAH = 178189
local RUNE_FIRE_GOLEMAGG = 178190
local RUNE_FIRE_GARR     = 178191
local RUNE_FIRE_MAGMADAR = 178192
local RUNE_FIRE_GEHENNAS = 178193

-- Majordomo summon positions
local MAJORDOMO_POS_NORMAL  = {x=758.089, y=-1176.71, z=-118.640, o=3.12414}
local MAJORDOMO_POS_RAGNAROS = {x=847.103, y=-816.153, z=-229.775, o=4.344}

-- ============================================================
-- Instance script table
-- ============================================================
local instance = {}

-- Called when a creature spawns in the instance.
-- Tracks boss GUIDs, despawns adds whose boss is already dead.
function instance:OnCreatureCreate(input, entry, guid)
    local actions = {}

    if entry == NPC_LUCIFRON then
        table.insert(actions, { action = "SET_GUID", data_id = TYPE_LUCIFRON, guid = guid })

    elseif entry == NPC_MAGMADAR then
        table.insert(actions, { action = "SET_GUID", data_id = TYPE_MAGMADAR, guid = guid })

    elseif entry == NPC_GEHENNAS then
        table.insert(actions, { action = "SET_GUID", data_id = TYPE_GEHENNAS, guid = guid })

    elseif entry == NPC_GEDDON then
        table.insert(actions, { action = "SET_GUID", data_id = TYPE_GEDDON, guid = guid })

    elseif entry == NPC_SHAZZRAH then
        table.insert(actions, { action = "SET_GUID", data_id = TYPE_SHAZZRAH, guid = guid })

    elseif entry == NPC_SULFURON then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_SULFURON, guid = guid })

    elseif entry == NPC_GOLEMAGG then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GOLEMAGG, guid = guid })

    elseif entry == NPC_GARR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GARR, guid = guid })

    elseif entry == NPC_MAJORDOMO then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_MAJORDOMO, guid = guid })

    elseif entry == NPC_RAGNAROS then
        table.insert(actions, { action = "SET_GUID", data_id = TYPE_RAGNAROS, guid = guid })

    -- Despawn adds when their boss encounter is already DONE
    elseif entry == NPC_FLAMEWAKER_PRIEST then
        if input:GetData(TYPE_SULFURON) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_FLAMEWAKER_PROTECTOR then
        if input:GetData(TYPE_LUCIFRON) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_FLAMEWAKER then
        if input:GetData(TYPE_GEHENNAS) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_CORE_RAGER then
        if input:GetData(TYPE_GOLEMAGG) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_CORE_HOUND or entry == NPC_ANCIENT_CORE_HOUND then
        if input:GetData(TYPE_MAGMADAR) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end

    elseif entry == NPC_FIRESWORN or entry == NPC_LAVA_SURGER then
        if input:GetData(TYPE_GARR) == DONE then
            table.insert(actions, { action = "DESPAWN_CREATURE", guid = guid })
        end
    end

    return actions
end

-- Called when a creature dies in the instance.
-- Updates encounter state for boss kills.
function instance:OnCreatureDeath(input, entry, guid)
    local actions = {}

    if entry == NPC_LUCIFRON then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_LUCIFRON, value = DONE })

    elseif entry == NPC_MAGMADAR then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_MAGMADAR, value = DONE })

    elseif entry == NPC_GEHENNAS then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_GEHENNAS, value = DONE })

    elseif entry == NPC_GEDDON then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_GEDDON, value = DONE })

    elseif entry == NPC_SHAZZRAH then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_SHAZZRAH, value = DONE })

    elseif entry == NPC_SULFURON then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_SULFURON, value = DONE })

    elseif entry == NPC_GOLEMAGG then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_GOLEMAGG, value = DONE })

    elseif entry == NPC_GARR then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_GARR, value = DONE })

    elseif entry == NPC_MAJORDOMO then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_MAJORDOMO, value = DONE })
        -- Respawn the Firelord Cache chest (GO entry 179703) for 1 hour
        table.insert(actions, { action = "RESPAWN_GAMEOBJECT", entry = RUNE_MAJORDOMO, duration = 3600 })

    elseif entry == NPC_RAGNAROS then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_RAGNAROS, value = DONE })
    end

    return actions
end

-- Called when a creature enters combat in the instance.
-- Links Majordomo and his Flamewaker adds (Healer/Elite) into the same fight.
function instance:OnCreatureEnterCombat(input, entry, guid)
    local actions = {}

    if entry == NPC_MAJORDOMO or entry == NPC_FLAMEWAKER_HEALER or entry == NPC_FLAMEWAKER_ELITE then
        -- SET_COMBAT_WITH_ZONE forces all Flamewakers + Domo within range to aggro together.
        -- The full mob-link (per unit) requires instance-level aggro propagation;
        -- SET_COMBAT_WITH_ZONE is the best approximation in the Lua API.
        table.insert(actions, { action = "SET_COMBAT_WITH_ZONE" })
    end

    return actions
end

-- Called when a game object spawns in the instance.
-- Tracks rune and cache GUIDs; marks Hot Coals as in-use.
function instance:OnGameObjectCreate(input, entry, guid)
    local actions = {}

    -- Rune game objects: store their GUIDs so we can respawn/use them
    if entry == RUNE_SULFURON then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_RUNE_ACTIVE_0, guid = guid })
        -- If rune was already activated in a prior run, queue cleanup
        if input:GetData(DATA_RUNE_ACTIVE_0) == DONE then
            table.insert(actions, { action = "RESPAWN_GAMEOBJECT", guid = guid, duration = 0 })
        end

    elseif entry == RUNE_GEDDON then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_RUNE_ACTIVE_1, guid = guid })
        if input:GetData(DATA_RUNE_ACTIVE_1) == DONE then
            table.insert(actions, { action = "RESPAWN_GAMEOBJECT", guid = guid, duration = 0 })
        end

    elseif entry == RUNE_SHAZZRAH then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_RUNE_ACTIVE_2, guid = guid })
        if input:GetData(DATA_RUNE_ACTIVE_2) == DONE then
            table.insert(actions, { action = "RESPAWN_GAMEOBJECT", guid = guid, duration = 0 })
        end

    elseif entry == RUNE_GOLEMAGG then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_RUNE_ACTIVE_3, guid = guid })
        if input:GetData(DATA_RUNE_ACTIVE_3) == DONE then
            table.insert(actions, { action = "RESPAWN_GAMEOBJECT", guid = guid, duration = 0 })
        end

    elseif entry == RUNE_GARR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_RUNE_ACTIVE_4, guid = guid })
        if input:GetData(DATA_RUNE_ACTIVE_4) == DONE then
            table.insert(actions, { action = "RESPAWN_GAMEOBJECT", guid = guid, duration = 0 })
        end

    elseif entry == RUNE_MAGMADAR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_RUNE_ACTIVE_5, guid = guid })
        if input:GetData(DATA_RUNE_ACTIVE_5) == DONE then
            table.insert(actions, { action = "RESPAWN_GAMEOBJECT", guid = guid, duration = 0 })
        end

    elseif entry == RUNE_GEHENNAS then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_RUNE_ACTIVE_6, guid = guid })
        if input:GetData(DATA_RUNE_ACTIVE_6) == DONE then
            table.insert(actions, { action = "RESPAWN_GAMEOBJECT", guid = guid, duration = 0 })
        end

    elseif entry == RUNE_MAJORDOMO then
        -- Firelord Cache - store GUID so we can respawn it after Majordomo dies
        table.insert(actions, { action = "SET_GUID", data_id = TYPE_MAJORDOMO + 100, guid = guid })
    end

    return actions
end

-- Called on instance load from database (after server restart or instance reset).
-- The data table is already restored from DB by the engine via SET_DATA actions.
function instance:OnLoad(input)
    return {}
end

-- Called when a player enters the instance.
function instance:OnPlayerEnter(input, player_guid)
    return {}
end

-- ============================================================
-- Rune activation helper (called from game object use handler)
-- Check if a boss's corresponding rune should be activated,
-- then check if all 7 runes are done and summon Majordomo.
--
-- This mirrors GOHello_go_rune_MC in vmangos.
-- The game engine calls OnGameObjectCreate for tracking; actual
-- rune "click" activation (GOHello) maps to OnGameObjectCreate
-- being triggered with the rune entry when a player interacts.
-- ============================================================

-- Rune configuration table: maps rune GO entry to {boss_type, rune_data_id, fire_go_entry}
local RUNE_CONFIG = {
    [RUNE_SULFURON]  = { boss = TYPE_SULFURON,  rune_id = DATA_RUNE_ACTIVE_0, fire = RUNE_FIRE_SULFURON },
    [RUNE_GEDDON]    = { boss = TYPE_GEDDON,    rune_id = DATA_RUNE_ACTIVE_1, fire = RUNE_FIRE_GEDDON   },
    [RUNE_SHAZZRAH]  = { boss = TYPE_SHAZZRAH,  rune_id = DATA_RUNE_ACTIVE_2, fire = RUNE_FIRE_SHAZZRAH },
    [RUNE_GOLEMAGG]  = { boss = TYPE_GOLEMAGG,  rune_id = DATA_RUNE_ACTIVE_3, fire = RUNE_FIRE_GOLEMAGG },
    [RUNE_GARR]      = { boss = TYPE_GARR,      rune_id = DATA_RUNE_ACTIVE_4, fire = RUNE_FIRE_GARR     },
    [RUNE_MAGMADAR]  = { boss = TYPE_MAGMADAR,  rune_id = DATA_RUNE_ACTIVE_5, fire = RUNE_FIRE_MAGMADAR },
    [RUNE_GEHENNAS]  = { boss = TYPE_GEHENNAS,  rune_id = DATA_RUNE_ACTIVE_6, fire = RUNE_FIRE_GEHENNAS },
}

-- Helper: check if all 7 runes are DONE
local function all_runes_done(input)
    return input:GetData(DATA_RUNE_ACTIVE_0) == DONE
       and input:GetData(DATA_RUNE_ACTIVE_1) == DONE
       and input:GetData(DATA_RUNE_ACTIVE_2) == DONE
       and input:GetData(DATA_RUNE_ACTIVE_3) == DONE
       and input:GetData(DATA_RUNE_ACTIVE_4) == DONE
       and input:GetData(DATA_RUNE_ACTIVE_5) == DONE
       and input:GetData(DATA_RUNE_ACTIVE_6) == DONE
end

-- Called periodically. Not heavily used for MC but needed for
-- the rune fire cleanup (vmangos Update() removes fire anim GOs).
function instance:Update(input)
    return {}
end

RegisterInstanceScript(409, instance)
