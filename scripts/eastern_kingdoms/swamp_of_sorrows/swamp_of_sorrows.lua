--[[
    Swamp of Sorrows zone scripts
    Reference: eastern_kingdoms/swamp_of_sorrows/swamp_of_sorrows.cpp

    npc_galen_goodward (entry 5391)
        Quest support: Galen's Escape (id 1393)
        OnQuestAccept: set escort faction + say text
        Note: escort AI logic handled by creature_ai script (not yet ported).
]]

local QUEST_GALENS_ESCAPE           = 1393
local FACTION_ESCORT_H_NEUTRAL_ACTIVE = 495  -- Horde-side neutral active escort faction
local SAY_GALEN_QUEST_ACCEPTED      = 1854

local npc_galen = {}

function npc_galen:OnQuestAccept(player, npc_guid, quest_id)
    if quest_id == QUEST_GALENS_ESCAPE then
        return {
            { action = "SET_FACTION", faction_id = FACTION_ESCORT_H_NEUTRAL_ACTIVE },
            { action = "SCRIPT_TEXT", text_id = SAY_GALEN_QUEST_ACCEPTED },
        }
    end
    return {}
end

RegisterGossipScript(5391, npc_galen)
