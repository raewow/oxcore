--[[
    Stonetalon Mountains zone scripts
    Reference: kalimdor/stonetalon_mountains/stonetalon_mountains.cpp

    npc_piznik (entry 4276)
        Quest support: Weapons of Choice (id 1090)
        OnQuestAccept: set faction to escort-friendly-active
        Note: escort AI logic handled by creature_ai script (not yet ported).
]]

local QUEST_WEAPONS_OF_CHOICE               = 1090
local FACTION_ESCORT_N_FRIEND_ACTIVE        = 250   -- generic neutral-friendly escort faction

local npc_piznik = {}

function npc_piznik:OnQuestAccept(player, npc_guid, quest_id)
    if quest_id == QUEST_WEAPONS_OF_CHOICE then
        return {
            { action = "SET_FACTION", faction_id = FACTION_ESCORT_N_FRIEND_ACTIVE },
        }
    end
    return {}
end

RegisterGossipScript(4276, npc_piznik)
