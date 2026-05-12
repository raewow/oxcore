--[[
    Silverpine Forest zone scripts
    Reference: eastern_kingdoms/silverpine_forest/silverpine_forest.cpp

    npc_deathstalker_erland (entry 1978)
        Quest support: The Deathstalkers (id 435)
        OnQuestAccept: set escort faction + say start text
        Note: escort AI logic handled by creature_ai script (not yet ported).
]]

local QUEST_THE_DEATHSTALKERS       = 435
local FACTION_ESCORTEE              = 232   -- generic escort faction (neutral passive)
local SAY_ERLAND_START_1            = 481

local npc_erland = {}

function npc_erland:OnQuestAccept(player, npc_guid, quest_id)
    if quest_id == QUEST_THE_DEATHSTALKERS then
        return {
            { action = "SET_FACTION", faction_id = FACTION_ESCORTEE },
            { action = "SCRIPT_TEXT", text_id = SAY_ERLAND_START_1 },
        }
    end
    return {}
end

RegisterGossipScript(1978, npc_erland)
