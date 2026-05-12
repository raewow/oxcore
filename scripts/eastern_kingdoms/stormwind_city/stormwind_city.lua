--[[
    Stormwind City zone scripts
    Reference: eastern_kingdoms/stormwind_city/stormwind_city.cpp

    npc_bartleby (entry 6090)
        Quest support: Beating Them Back! (id 1640)
        OnQuestAccept: set faction to enemy + start attacking player
        Note: AttackStart(player) requires creature-AI interaction not yet
              available from gossip callbacks. Faction change is applied;
              attack initiation is stubbed.

    npc_dashel_stonefist (entry 4961)
        Quest support: The Stonemason Conspiracy (id 1447)
        OnQuestAccept: say progress text + set neutral faction + summon thugs
        Note: SpawnCreature (NPC_OLD_TOWN_THUG=4969 x3) not yet available
              via gossip Lua API. Say + faction change are applied.
]]

-- npc_bartleby (entry 6090)
local QUEST_BEATING_THEM_BACK       = 1640
local FACTION_ENEMY                 = 168

local npc_bartleby = {}

function npc_bartleby:OnQuestAccept(player, npc_guid, quest_id)
    if quest_id == QUEST_BEATING_THEM_BACK then
        return {
            { action = "SET_FACTION", faction_id = FACTION_ENEMY },
            -- AttackStart(player) requires creature-AI trigger; not yet supported.
        }
    end
    return {}
end

RegisterGossipScript(6090, npc_bartleby)

-- npc_dashel_stonefist (entry 4961)
local QUEST_STONEMASON_CONSPIRACY   = 1447
local FACTION_NEUTRAL               = 189
local SAY_DASHEL_PROGRESS_1        = 1961
-- local NPC_OLD_TOWN_THUG          = 4969  -- 3 thugs spawned around Dashel; SpawnCreature not yet available

local npc_dashel = {}

function npc_dashel:OnQuestAccept(player, npc_guid, quest_id)
    if quest_id == QUEST_STONEMASON_CONSPIRACY then
        return {
            { action = "SCRIPT_TEXT", text_id = SAY_DASHEL_PROGRESS_1 },
            { action = "SET_FACTION", faction_id = FACTION_NEUTRAL },
            -- TODO: SpawnCreature(NPC_OLD_TOWN_THUG x3) when API available
        }
    end
    return {}
end

RegisterGossipScript(4961, npc_dashel)
