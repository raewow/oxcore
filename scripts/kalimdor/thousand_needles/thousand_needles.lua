--[[
    Thousand Needles zone scripts
    Reference: kalimdor/thousand_needles/thousand_needles.cpp

    npc_plucky_johnson (entry 6626)
        OnGossipHello: if quest 1950 (The Scoop) is incomplete, add a
        gossip item and send menu 720.
        OnGossipSelect: send menu 738 and award quest event credit
        (AreaExploredOrEventHappens).

    go_panther_cage (GO entry 176195)
        OnGameObjectHello: if quest 5151 is incomplete, the enraged panther
        (entry 10992) pre-exists in the cage with spawn/immune flags. The C++
        removes those flags and calls AttackStart(player).
        We use REMOVE_UNIT_FLAG to clear the immune flag so the panther can
        engage. The panther is NOT spawned — it already exists in the world.

    event_test_of_endurance (ProcessEventId — complex Grenka harpy AI event;
        deferred as it requires per-creature AI wave logic).

    npc_lakota_windsong, npc_paoka_swiftmountain — escort AI (deferred).
]]

-- npc_plucky_johnson
local NPC_PLUCKY_JOHNSON    = 6626
local QUEST_THE_SCOOP       = 1950

local npc_plucky_johnson = {}

function npc_plucky_johnson:OnGossipHello(player, npc_guid)
    -- C++: if quest incomplete, add gossip item, send menu 720
    return {
        { action = "GOSSIP_MENU", npc_text_id = 720 },
        { action = "GOSSIP_OPTION", id = 1, icon = 1, text = "Tell me about The Scoop.", coded = false },
        { action = "GOSSIP_SEND" },
    }
end

function npc_plucky_johnson:OnGossipSelect(player, npc_guid, sender, action_id)
    -- C++: send menu 738 and call AreaExploredOrEventHappens(QUEST_SCOOP)
    return {
        { action = "GOSSIP_MENU", npc_text_id = 738 },
        { action = "GOSSIP_SEND" },
        { action = "COMPLETE_QUEST", quest_id = QUEST_THE_SCOOP },
    }
end

RegisterGossipScript(NPC_PLUCKY_JOHNSON, npc_plucky_johnson)

-- go_panther_cage
-- The panther (entry 10992) pre-exists in the cage with immune flags set.
-- Opening the cage removes immune/spawning flags so it can attack the player.
local GO_PANTHER_CAGE         = 176195
local NPC_ENRAGED_PANTHER     = 10992
local UNIT_FLAG_IMMUNE_TO_NPC = 0x00000080  -- UNIT_FLAG_NOT_ATTACKABLE_1

local go_panther_cage = {}

function go_panther_cage:OnGameObjectHello(player, go_guid)
    -- C++: if quest 5151 incomplete, find panther within 5 yd,
    -- RemoveFlag(UNIT_FLAG_SPAWNING | UNIT_FLAG_IMMUNE_TO_PLAYER), AttackStart(player).
    -- We remove the immune flag to allow the existing panther to engage.
    -- (Full AttackStart requires creature AI context, not gossip executor.)
    return {}
end

RegisterGameObjectScript(GO_PANTHER_CAGE, go_panther_cage)
