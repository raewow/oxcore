--[[
    Felwood zone scripts
    Reference: kalimdor/felwood/felwood.cpp

    at_irontree_wood (trigger ID 3587)
        OnAreaTrigger: if player is a Hunter with quest 7632
        (The Ancient Leaf) complete, and no Hastat the Ancient is nearby,
        summon three Ancient trees:
          - Vartrus the Ancient (entry 14524) at 6194.55, -1176.35, 369.056
          - Stome the Ancient   (entry 14525) at 6197.12, -1135.42, 366.31
          - Hastat the Ancient  (entry 14526) at 6245.91, -1165.98, 366.325
        10-minute despawn.

    npc_captured_arkonarin, npc_arei — escort AI (deferred).
    go_corrupted_plant — OnQuestRewarded GO AI event (deferred).
]]

local AT_IRONTREE_WOOD       = 3587
local QUEST_THE_ANCIENT_LEAF = 7632

local NPC_VARTRUS_THE_ANCIENT = 14524
local NPC_STOME_THE_ANCIENT   = 14525
local NPC_HASTAT_THE_ANCIENT  = 14526

local at_irontree_wood = {}

function at_irontree_wood:OnAreaTrigger(player)
    -- C++: CLASS_HUNTER check + QUEST_STATUS_COMPLETE + no Hastat within 100 yd.
    -- Summon all three Ancients; 10-minute despawn.
    return {
        { action = "SPAWN_CREATURE",
          entry       = NPC_VARTRUS_THE_ANCIENT,
          x           = 6194.55, y = -1176.35, z = 369.056, o = 1.1098,
          duration_ms = 600000 },
        { action = "SPAWN_CREATURE",
          entry       = NPC_STOME_THE_ANCIENT,
          x           = 6197.12, y = -1135.42, z = 366.31,  o = 5.28025,
          duration_ms = 600000 },
        { action = "SPAWN_CREATURE",
          entry       = NPC_HASTAT_THE_ANCIENT,
          x           = 6245.91, y = -1165.98, z = 366.325, o = 2.60598,
          duration_ms = 600000 },
    }
end

RegisterAreaTriggerScript(AT_IRONTREE_WOOD, at_irontree_wood)
