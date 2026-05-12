--[[
    Hinterlands zone scripts
    Reference: eastern_kingdoms/hinterlands/hinterlands.cpp

    go_lards_picnic_basket (GO entry 179910)
        OnGameObjectHello: spawn 3x Kidnapper Vilebranch (entry 14748) at the
        player's current position. C++: pPlayer->SummonCreature(entry, x, y, z,
        TEMPSUMMON_TIMED_OR_DEAD_DESPAWN, 30000).

    npc_rinji (entry 2713) — escort AI, not yet ported.
]]

local GO_LARDS_PICNIC_BASKET   = 179910
local NPC_KIDNAPPER_VILEBRANCH = 14748

local go_lards_picnic_basket = {}

function go_lards_picnic_basket:OnGameObjectHello(player, go_guid)
    return {
        { action = "SPAWN_CREATURE_AT_PLAYER", entry = NPC_KIDNAPPER_VILEBRANCH, duration_ms = 30000,
          summon_type = "TIMED_DESPAWN" },
        { action = "SPAWN_CREATURE_AT_PLAYER", entry = NPC_KIDNAPPER_VILEBRANCH, duration_ms = 30000,
          summon_type = "TIMED_DESPAWN" },
        { action = "SPAWN_CREATURE_AT_PLAYER", entry = NPC_KIDNAPPER_VILEBRANCH, duration_ms = 30000,
          summon_type = "TIMED_DESPAWN" },
    }
end

RegisterGameObjectScript(GO_LARDS_PICNIC_BASKET, go_lards_picnic_basket)
