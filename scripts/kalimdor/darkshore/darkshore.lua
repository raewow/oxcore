--[[
    @script_type: gossip
    @entry: 3882
    @name: npc_therylune
    SDComment: Quest support: Therylune's Escape (id 945)
    Reference: kalimdor/darkshore/darkshore.cpp
    Note: Escort AI logic is handled by creature_ai script (not yet ported).
          This script handles the quest-accept faction change and say.
]]

local QUEST_THERYLUNE_ESCAPE            = 945
local SAY_THERYLUNE_START               = 1189
local FACTION_ESCORT_A_NEUTRAL_PASSIVE  = 79

local npc = {}

function npc:OnQuestAccept(player, npc_guid, quest_id)
    if quest_id == QUEST_THERYLUNE_ESCAPE then
        return {
            { action = "SET_FACTION", faction_id = FACTION_ESCORT_A_NEUTRAL_PASSIVE },
            { action = "SCRIPT_TEXT", text_id = SAY_THERYLUNE_START },
        }
    end
    return {}
end

RegisterGossipScript(3882, npc)
