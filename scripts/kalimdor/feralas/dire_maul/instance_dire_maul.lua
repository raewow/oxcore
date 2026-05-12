--[[
    @script_type: instance
    @map_id: 429
    @name: instance_dire_maul

    Dire Maul - Instance Script
    Ported from vmangos instance_dire_maul.cpp

    Manages:
    - 12 encounter types across DM East, West, and North
    - DM East: Alzzin the Wildshaper, crumble wall, corrupt vine
    - DM West: Crystal event (5 crystals activate Immol'Thar), force field,
      magic vortex, Tortheldrin
    - DM North: King Gordok, Gordok Tribute chest, guard death tracking,
      Chorush the Observer, broken trap
]]

-- ============================================================
-- Encounter type IDs
-- ============================================================
local TYPE_CRISTAL_EVENT     = 1
local TYPE_IMMOL_THAR        = 2
local DATA_TENDRIS_AGGRO     = 3
local TYPE_BOSS_ZEVRIM       = 4
local TYPE_SPEAK_ECORCEFER   = 5
local TYPE_GORDOK_TRIBUTE    = 6
local TYPE_BROKEN_TRAP       = 7
local TYPE_GORDOK_OGRE_SUIT  = 8
local TYPE_CHORUSH_EQUIPMENT = 9
local TYPE_MOLDAR            = 10
local TYPE_ALZZIN            = 11

-- Encounter state values
local NOT_STARTED = 0
local IN_PROGRESS = 1
local DONE        = 2
local SPECIAL     = 3

-- ============================================================
-- NPC entries
-- ============================================================
-- DM East
local NPC_OLD_IRONBARK         = 11491
local NPC_ZEVRIM               = 11490
local NPC_ALZZIN               = 11492

-- DM West
local NPC_IMMOL_THAR_GARDIEN   = 11466
local NPC_IMMOL_THAR           = 11496
local NPC_TORTHELDRIN          = 11486
local NPC_RESTE_MANA           = 11483
local NPC_ARCANE_ABERRATION    = 11480
local NPC_TENDRIS              = 11489
local NPC_TENDRIS_PROTECTOR    = 11459

-- DM North
local NPC_GUARD_MOLDAR         = 14326
local NPC_GUARD_FENGUS         = 14321
local NPC_GUARD_SLIPKIK        = 14323
local NPC_CAPTAIN_KROMCRUSH    = 14325
local NPC_CHORUSH              = 14324
local NPC_KING_GORDOK          = 11501
local NPC_MIZZLE_THE_CRAFTY    = 14353

-- ============================================================
-- Game Object entries
-- ============================================================
-- DM East
local GO_CRUMBLE_WALL          = 177220
local GO_FELVINE_SHARD         = 179559
local GO_CORRUPT_VINE          = 179502
local GO_DOOR_ALZZIN_IN        = 181496

-- DM West
local GO_FORCE_FIELD           = 179503
local GO_MAGIC_VORTEX          = 179506
local GO_CRISTAL_1_EVENT       = 177259
local GO_CRISTAL_2_EVENT       = 177257
local GO_CRISTAL_3_EVENT       = 177258
local GO_CRISTAL_4_EVENT       = 179504
local GO_CRISTAL_5_EVENT       = 179505
local GO_RITUAL_CANDLE_AURA    = 179688

-- DM North
local GO_BROKEN_TRAP           = 179485
local GO_GORDOK_TRIBUTE_0      = 179564
local GO_GORDOK_TRIBUTE_1      = 300400
local GO_GORDOK_TRIBUTE_2      = 300401
local GO_GORDOK_TRIBUTE_3      = 300402
local GO_GORDOK_TRIBUTE_4      = 300403
local GO_GORDOK_TRIBUTE_5      = 300404
local GO_GORDOK_TRIBUTE_6      = 300405

-- GUID data IDs
-- DM East
local DATA_ALZZIN              = 100
local DATA_DOOR_ALZZIN_IN      = 101
local DATA_CRUMBLE_WALL        = 102
local DATA_CORRUPT_VINE        = 103

-- DM West
local DATA_IMMOL_THAR          = 110
local DATA_TORTHELDRIN         = 111
local DATA_FORCE_FIELD         = 112
local DATA_MAGIC_VORTEX        = 113
local DATA_CRISTAL_1           = 114
local DATA_CRISTAL_2           = 115
local DATA_CRISTAL_3           = 116
local DATA_CRISTAL_4           = 117
local DATA_CRISTAL_5           = 118
local DATA_RITUAL_CANDLE       = 119
local DATA_TENDRIS             = 120
local DATA_OLD_IRONBARK        = 121

-- DM North
local DATA_GUARD_SLIPKIK       = 130
local DATA_CAPTAIN_KROMCRUSH   = 131
local DATA_KING_GORDOK         = 132
local DATA_CHORUSH             = 133
local DATA_BROKEN_TRAP         = 134
local DATA_GORDOK_TRIBUTE_0    = 135
local DATA_GORDOK_TRIBUTE_1    = 136
local DATA_GORDOK_TRIBUTE_2    = 137
local DATA_GORDOK_TRIBUTE_3    = 138
local DATA_GORDOK_TRIBUTE_4    = 139
local DATA_GORDOK_TRIBUTE_5    = 140
local DATA_GORDOK_TRIBUTE_6    = 141

-- ============================================================
-- Instance script table
-- ============================================================
local instance = {}

function instance:OnCreatureCreate(input, entry, guid)
    local actions = {}

    if entry == NPC_ALZZIN then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_ALZZIN, guid = guid })

    elseif entry == NPC_IMMOL_THAR then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_IMMOL_THAR, guid = guid })

    elseif entry == NPC_TORTHELDRIN then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_TORTHELDRIN, guid = guid })

    elseif entry == NPC_TENDRIS then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_TENDRIS, guid = guid })

    elseif entry == NPC_OLD_IRONBARK then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_OLD_IRONBARK, guid = guid })

    elseif entry == NPC_GUARD_SLIPKIK then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GUARD_SLIPKIK, guid = guid })

    elseif entry == NPC_CAPTAIN_KROMCRUSH then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_CAPTAIN_KROMCRUSH, guid = guid })

    elseif entry == NPC_KING_GORDOK then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_KING_GORDOK, guid = guid })

    elseif entry == NPC_CHORUSH then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_CHORUSH, guid = guid })
    end

    return actions
end

function instance:OnCreatureDeath(input, entry, guid)
    local actions = {}

    if entry == NPC_ALZZIN then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_ALZZIN, value = DONE })
        -- Open crumble wall and corrupt vine on Alzzin death
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_CRUMBLE_WALL })
        table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_CORRUPT_VINE })

    elseif entry == NPC_KING_GORDOK then
        -- Gordok Tribute becomes accessible
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_GORDOK_TRIBUTE, value = DONE })

    elseif entry == NPC_GUARD_MOLDAR then
        table.insert(actions, { action = "SET_DATA", data_id = TYPE_MOLDAR, value = DONE })
        if input:GetData(TYPE_GORDOK_TRIBUTE) ~= DONE then
            table.insert(actions, { action = "SET_DATA", data_id = TYPE_GORDOK_TRIBUTE, value = SPECIAL })
        end

    elseif entry == NPC_GUARD_FENGUS
        or entry == NPC_GUARD_SLIPKIK
        or entry == NPC_CAPTAIN_KROMCRUSH
        or entry == NPC_CHORUSH
    then
        if input:GetData(TYPE_GORDOK_TRIBUTE) ~= DONE then
            table.insert(actions, { action = "SET_DATA", data_id = TYPE_GORDOK_TRIBUTE, value = SPECIAL })
        end
    end

    return actions
end

function instance:OnGameObjectCreate(input, entry, guid)
    local actions = {}

    -- DM East
    if entry == GO_DOOR_ALZZIN_IN then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_DOOR_ALZZIN_IN, guid = guid })

    elseif entry == GO_CRUMBLE_WALL then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_CRUMBLE_WALL, guid = guid })
        if input:GetData(TYPE_ALZZIN) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_CRUMBLE_WALL })
        end

    elseif entry == GO_CORRUPT_VINE then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_CORRUPT_VINE, guid = guid })
        if input:GetData(TYPE_ALZZIN) == DONE then
            table.insert(actions, { action = "OPEN_DOOR", data_id = DATA_CORRUPT_VINE })
        end

    -- DM West
    elseif entry == GO_FORCE_FIELD then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_FORCE_FIELD, guid = guid })

    elseif entry == GO_MAGIC_VORTEX then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_MAGIC_VORTEX, guid = guid })

    elseif entry == GO_CRISTAL_1_EVENT then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_CRISTAL_1, guid = guid })
    elseif entry == GO_CRISTAL_2_EVENT then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_CRISTAL_2, guid = guid })
    elseif entry == GO_CRISTAL_3_EVENT then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_CRISTAL_3, guid = guid })
    elseif entry == GO_CRISTAL_4_EVENT then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_CRISTAL_4, guid = guid })
    elseif entry == GO_CRISTAL_5_EVENT then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_CRISTAL_5, guid = guid })

    elseif entry == GO_RITUAL_CANDLE_AURA then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_RITUAL_CANDLE, guid = guid })

    -- DM North
    elseif entry == GO_BROKEN_TRAP then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_BROKEN_TRAP, guid = guid })

    elseif entry == GO_GORDOK_TRIBUTE_0 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GORDOK_TRIBUTE_0, guid = guid })
    elseif entry == GO_GORDOK_TRIBUTE_1 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GORDOK_TRIBUTE_1, guid = guid })
    elseif entry == GO_GORDOK_TRIBUTE_2 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GORDOK_TRIBUTE_2, guid = guid })
    elseif entry == GO_GORDOK_TRIBUTE_3 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GORDOK_TRIBUTE_3, guid = guid })
    elseif entry == GO_GORDOK_TRIBUTE_4 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GORDOK_TRIBUTE_4, guid = guid })
    elseif entry == GO_GORDOK_TRIBUTE_5 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GORDOK_TRIBUTE_5, guid = guid })
    elseif entry == GO_GORDOK_TRIBUTE_6 then
        table.insert(actions, { action = "SET_GUID", data_id = DATA_GORDOK_TRIBUTE_6, guid = guid })
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

RegisterInstanceScript(429, instance)
