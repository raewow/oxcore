--[[
    Teldrassil zone scripts
    Reference: kalimdor/teldrassil/teldrassil.cpp

    npc_mist (entry 3568)
        Quest support: Mist (id 938)
        OnQuestAccept: set faction to Darnassus + say text

    npc_treshala_fallowbrook (entry 4521)
        Quest support: Teldrassil (id 1142)
        OnQuestRewarded: emote OneShot_Cry
]]

-- npc_mist (entry 3568)
local QUEST_MIST                    = 938
local FACTION_DARNASSUS             = 79
local SAY_MIST_AT_HOME              = 1330

local npc_mist = {}

function npc_mist:OnQuestAccept(player, npc_guid, quest_id)
    if quest_id == QUEST_MIST then
        return {
            { action = "SET_FACTION", faction_id = FACTION_DARNASSUS },
            { action = "SCRIPT_TEXT", text_id = SAY_MIST_AT_HOME },
        }
    end
    return {}
end

RegisterGossipScript(3568, npc_mist)

-- npc_treshala_fallowbrook (entry 4521)
local QUEST_TELDRASSIL              = 1142
local EMOTE_ONESHOT_CRY             = 40

local npc_treshala = {}

function npc_treshala:OnQuestRewarded(player, npc_guid, quest_id)
    if quest_id == QUEST_TELDRASSIL then
        return {
            { action = "EMOTE", emote_id = EMOTE_ONESHOT_CRY },
        }
    end
    return {}
end

RegisterGossipScript(4521, npc_treshala)
