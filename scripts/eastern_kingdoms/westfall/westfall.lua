--[[
    Westfall zone scripts
    Reference: eastern_kingdoms/westfall/westfall.cpp

    npc_daphne_stilwell (entry 6182)
        Quest support: The Defias Brotherhood (id 1651)
        OnQuestAccept: say start text + begin escort
        Note: escort AI logic handled by creature_ai script (not yet ported).
]]

local QUEST_DEFIAS_BROTHERHOOD      = 1651
local SAY_DS_START                  = 2360

local npc_daphne = {}

function npc_daphne:OnQuestAccept(player, npc_guid, quest_id)
    if quest_id == QUEST_DEFIAS_BROTHERHOOD then
        return {
            { action = "SCRIPT_TEXT", text_id = SAY_DS_START },
        }
    end
    return {}
end

RegisterGossipScript(6182, npc_daphne)
