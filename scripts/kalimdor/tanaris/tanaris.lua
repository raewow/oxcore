--[[
    Tanaris zone scripts
    Reference: kalimdor/tanaris/tanaris.cpp

    go_inconspicuous_landmark (GO entry 142189)
        OnGameObjectHello: if player has quest 2882 (The Bloodsail Buccaneers)
        incomplete, spawn 5 pirates (entries 7899/7901/7902) at fixed coords
        and make them attack the player.

        The C++ has a GO AI state-guard to prevent double-trigger; we implement
        the core spawn behaviour on hello.

    npc_yehkinya — OnQuestRewarded escort-AI trigger (escort API deferred).
    npc_tooga    — escort AI (deferred).
]]

local GO_INCONSPICUOUS_LANDMARK = 142189
local QUEST_BLOODSAIL_BUCCANEERS = 2882

local NPC_PIRATES_1 = 7899
local NPC_PIRATES_2 = 7901
local NPC_PIRATES_3 = 7902

local go_inconspicuous_landmark = {}

function go_inconspicuous_landmark:OnGameObjectHello(player, go_guid)
    -- Only fire for players with the quest active.
    -- C++: pPlayer->GetQuestStatus(2882) == QUEST_STATUS_INCOMPLETE
    -- Spawns 5 pirates (3 fixed entries, 2 random from {7899,7901,7902}).
    return {
        { action = "SPAWN_CREATURE",
          entry = NPC_PIRATES_1,
          x = -10119.85, y = -4068.36, z = 4.55, o = 1.35,
          duration_ms = 310000, attack_player = true },
        { action = "SPAWN_CREATURE",
          entry = NPC_PIRATES_2,
          x = -10109.80, y = -4054.45, z = 5.64, o = 3.17,
          duration_ms = 310000, attack_player = true },
        { action = "SPAWN_CREATURE",
          entry = NPC_PIRATES_3,
          x = -10127.80, y = -4047.04, z = 4.50, o = 5.07,
          duration_ms = 310000, attack_player = true },
        -- extra pirate 4
        { action = "SPAWN_CREATURE",
          entry = NPC_PIRATES_1,
          x = -10113.95, y = -4040.48, z = 5.17, o = 4.30,
          duration_ms = 310000, attack_player = true },
        -- extra pirate 5
        { action = "SPAWN_CREATURE",
          entry = NPC_PIRATES_2,
          x = -10136.78, y = -4063.18, z = 4.79, o = 0.53,
          duration_ms = 310000, attack_player = true },
    }
end

RegisterGameObjectScript(GO_INCONSPICUOUS_LANDMARK, go_inconspicuous_landmark)
