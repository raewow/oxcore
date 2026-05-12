--[[
    Searing Gorge zone scripts
    Reference: eastern_kingdoms/searing_gorge/searing_gorge.cpp

    npc_dying_archaeologist (entry 8417)
        Quest support: A Little Help From My Friends (id 3566)
        OnGossipHello: show default gossip menu if Obsidion (8400) is alive
        OnQuestAccept: trigger Obsidion's awakening event (via AI cast/emote)
        Note: Obsidion-awakening AI logic is handled by creature_ai scripts.
              The GossipHello check for whether Obsidion is alive requires
              creature lookup — stub sends menu unconditionally for now.
]]

local QUEST_LITTLE_HELP             = 3566
local NPC_OBSIDION                  = 8400  -- referenced for context; Obsidion's entry

-- Default gossip text ID when the archaeologist has the quest available
local GOSSIP_TEXT_DEFAULT           = 0

local npc_archaeologist = {}

function npc_archaeologist:OnGossipHello(player, npc_guid)
    -- In the C++ reference, gossip menu is only shown if Obsidion is still alive.
    -- That check requires a world creature lookup not yet available via Lua API.
    -- Send default gossip menu unconditionally for now.
    return {
        { action = "GOSSIP_MENU", npc_text_id = GOSSIP_TEXT_DEFAULT },
        { action = "GOSSIP_SEND" },
    }
end

function npc_archaeologist:OnQuestAccept(player, npc_guid, quest_id)
    if quest_id == QUEST_LITTLE_HELP then
        -- In the C++ reference, this triggers Obsidion's pEventAI script via DoScriptText.
        -- That cross-creature AI activation is not yet available via Lua API.
        -- The quest accept is recorded; Obsidion event remains unimplemented.
        return {}
    end
    return {}
end

RegisterGossipScript(8417, npc_archaeologist)
