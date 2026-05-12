--[[
    Feralas zone scripts
    Reference: kalimdor/feralas/feralas.cpp

    npc_screecher_spirit (entry 8612)
        OnGossipHello: open gossip menu 2039, award TalkedToCreature quest credit
        (equivalent to kill credit for the NPC's entry), and set the NPC
        non-selectable (one-time use).

    npc_shay_leafrunner — escort AI (deferred).
    npc_kindal_moonweaver — escort AI (deferred).
]]

local NPC_SCREECHER_SPIRIT  = 8612
local UNIT_FLAG_NOT_SELECTABLE = 0x02000000

local npc_screecher_spirit = {}

function npc_screecher_spirit:OnGossipHello(player, npc_guid)
    -- C++: SEND_GOSSIP_MENU(2039) + TalkedToCreature(entry, guid) + SetFlag NOT_SELECTABLE
    return {
        { action = "GOSSIP_MENU", npc_text_id = 2039 },
        { action = "GOSSIP_SEND" },
        { action = "KILL_CREDIT_NEAREST_CREATURE",
          creature_entry = NPC_SCREECHER_SPIRIT,
          search_radius  = 1.0 },
        { action = "SET_UNIT_FLAG", flag = UNIT_FLAG_NOT_SELECTABLE },
    }
end

RegisterGossipScript(NPC_SCREECHER_SPIRIT, npc_screecher_spirit)
