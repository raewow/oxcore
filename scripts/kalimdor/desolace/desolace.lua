--[[
    Desolace zone scripts
    Reference: kalimdor/desolace/desolace.cpp

    go_hand_of_iruxos_crystal (GO entry 176581)
        OnGameObjectHello: summon Demon Spirit (entry 11876) at fixed coords
        and make it attack the player.

    npc_melizza_brimbuzzle, npc_dalinda_malem, npc_cork_gizelton,
    npc_rigger_gizelton — escort AI scripts (deferred; require escort API).
]]

local GO_HAND_OF_IRUXOS_CRYSTAL = 176581
local NPC_DEMON_SPIRIT          = 11876

local go_hand_of_iruxos_crystal = {}

function go_hand_of_iruxos_crystal:OnGameObjectHello(player, go_guid)
    -- Summon Demon Spirit at fixed coords and attack player.
    -- C++: pPlayer->SummonCreature(DEMON_SPIRIT, -346.84f, 1765.13f, 138.39f,
    --       5.91f, TEMPSUMMON_TIMED_DESPAWN_OUT_OF_COMBAT, 15000)
    return {
        { action = "SPAWN_CREATURE",
          entry       = NPC_DEMON_SPIRIT,
          x           = -346.84,
          y           = 1765.13,
          z           = 138.39,
          o           = 5.91,
          duration_ms = 15000,
          attack_player = true },
    }
end

RegisterGameObjectScript(GO_HAND_OF_IRUXOS_CRYSTAL, go_hand_of_iruxos_crystal)
