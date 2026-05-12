--[[
    @script_type: creature_ai
    @entry: 0
    @name: lib_constants
    @priority: 1

    Shared constants for all scripts.
    Loaded first due to low priority number.
]]

-- Encounter states
NOT_STARTED = 0
IN_PROGRESS = 1
FAIL = 2
DONE = 3
SPECIAL = 4

-- Common emote IDs
EMOTE_GENERIC_FRENZY_KILL = 7797

-- Blackrock Depths data IDs
BRD = {
    -- Encounter types
    TYPE_RING_OF_LAW       = 0,
    TYPE_VAULT             = 1,
    TYPE_ROCKNOT           = 2,
    TYPE_TOMB_OF_SEVEN     = 3,
    TYPE_LYCEUM            = 4,
    TYPE_IRON_HALL         = 5,
    TYPE_THUNDERBREW       = 6,
    TYPE_RELIC_COFFER      = 7,
    TYPE_DOOMGRIP          = 8,
    TYPE_RIBBLY            = 9,

    -- NPC entries
    NPC_MAGMUS             = 9938,
    NPC_EMPEROR            = 9019,
    NPC_PRINCESS           = 8929,
    NPC_PHALANX            = 9502,
    NPC_GRIZZLE            = 9028,
    NPC_GOROSH             = 9027,
    NPC_ANUBSHIAH          = 9031,
    NPC_GERSTAHN           = 9018,
}

