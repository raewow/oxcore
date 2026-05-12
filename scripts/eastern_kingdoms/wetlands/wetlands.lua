--[[
    Wetlands zone scripts
    Reference: eastern_kingdoms/wetlands/wetlands.cpp

    npc_mikhail (entry 4963)
        Quest support: Apprentice's Duties (id 1249) / part of Missing Diplomat chain
        OnGossipHello: if player has completed quest 1248, offer quest 1249;
                       otherwise send text 1713
        OnQuestAccept: set escort faction + begin escort
        Note: escort AI and npc_tapoke_slim_jahn (4962) interaction handled
              by creature_ai scripts (not yet ported).
              Quest-state check in GossipHello requires player quest API
              not yet available via gossip Lua callbacks.
              Sending the quest via gossip requires GOSSIP_QUEST action.
]]

local QUEST_MISSING_DIPLOMAT_PT4    = 1248  -- prerequisite
local QUEST_MISSING_DIPLOMAT_PT5    = 1249  -- the one Mikhail gives
local FACTION_ESCORT_A_NEUTRAL_PASSIVE = 113
local GOSSIP_TEXT_MIKHAIL           = 1713

local npc_mikhail = {}

function npc_mikhail:OnGossipHello(player, npc_guid)
    -- C++ reference: if player has completed quest 1248, show quest 1249.
    -- Player quest-state check not yet available in Lua gossip API.
    -- Fall back to sending the available quest unconditionally.
    return {
        { action = "GOSSIP_MENU", npc_text_id = GOSSIP_TEXT_MIKHAIL },
        { action = "GOSSIP_QUEST", quest_id = QUEST_MISSING_DIPLOMAT_PT5 },
        { action = "GOSSIP_SEND" },
    }
end

function npc_mikhail:OnQuestAccept(player, npc_guid, quest_id)
    if quest_id == QUEST_MISSING_DIPLOMAT_PT5 then
        return {
            { action = "SET_FACTION", faction_id = FACTION_ESCORT_A_NEUTRAL_PASSIVE },
        }
    end
    return {}
end

RegisterGossipScript(4963, npc_mikhail)
