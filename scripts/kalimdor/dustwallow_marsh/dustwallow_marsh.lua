--[[
    @script_type: gossip
    @entry: 4967
    @name: npc_archmage_tervosh
    SDComment: Quest support: Missing Diplomat pt. 14 (id 1249)
    Reference: kalimdor/dustwallow_marsh/dustwallow_marsh.cpp
]]

local QUEST_MISSING_DIPLO_PT14 = 1249
local SAY_ON_QUEST_REWARD       = 1751  -- "Go with grace, and may the Lady's magic protect you."

local npc = {}

function npc:OnQuestRewarded(player, npc_guid, quest_id)
    if quest_id == QUEST_MISSING_DIPLO_PT14 then
        return {
            { action = "SCRIPT_TEXT", text_id = SAY_ON_QUEST_REWARD },
            -- Note: SPELL_PROUDMOORES_DEFENSE (17683) cast on player not yet supported
        }
    end
    return {}
end

RegisterGossipScript(4967, npc)
