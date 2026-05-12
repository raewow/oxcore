--[[
    Duskwood zone scripts
    Reference: eastern_kingdoms/duskwood/duskwood.cpp

    at_twilight_grove (trigger 4522, 4523)
        OnAreaTrigger: if player has quest Nightmare Corruption (8735) active,
        summon Twilight Corrupter (15625) near player if not already present,
        then whisper the player.
        Note: KilledMonsterCredit and SummonCreature not yet available in Lua executor;
        the whisper is also blocked (WhisperPlayer not yet available).
        This script handles what is currently possible: the trigger fires and logs.
        Full behavior requires SummonCreature and Whisper actions.
]]

-- at_twilight_grove
local QUEST_NIGHTMARE_CORRUPTION = 8735
local NPC_TWILIGHT_CORRUPTER     = 15625
local AT_TWILIGHT_GROVE_1        = 4522
local AT_TWILIGHT_GROVE_2        = 4523

local at_twilight_grove = {}

function at_twilight_grove:OnAreaTrigger(player)
    -- Only fire for players with the quest active
    -- Quest status check is not yet in the action system; script fires and
    -- SpawnCreature action will be ignored if executor doesn't support it yet.
    -- Reference: Handle_NightmareCorruption in duskwood.cpp
    return {
        { action = "SPAWN_CREATURE",
          entry = NPC_TWILIGHT_CORRUPTER,
          x = -10335.9, y = -489.051, z = 50.6233, o = 2.59373,
          duration_ms = 0 },
    }
end

RegisterAreaTriggerScript(AT_TWILIGHT_GROVE_1, at_twilight_grove)
RegisterAreaTriggerScript(AT_TWILIGHT_GROVE_2, at_twilight_grove)
